package main

import (
	"bytes"
	"encoding/binary"
	"errors"
)

var hostIP = [4]byte{127, 0, 0, 1}

type localFlow struct {
	started, sent bool
	source, port  uint16
	destination   [4]byte
	proto         byte
	payload       [20]byte
	start         [2]uint32
	haveStart     [2]bool
	covered       [2][20]bool
	udpRX, udpTX  int
}
type rejectObserver struct {
	tcpPort, udpPort  uint16
	phase             int
	active            bool
	bootstrap, health *observer
	flows             [4]localFlow
}

// 状态替换始终持有最外层观察器锁；tun goroutine 不读取可变裸指针。
func (o *observer) beginRejectPhase(phase int) error {
	o.mu.Lock()
	defer o.mu.Unlock()
	r := o.reject
	if o.err != nil {
		return o.err
	}
	if r.active || phase != r.phase+1 || phase > 3 {
		return errors.New("phase sequence")
	}
	r.phase = phase
	r.active = true
	r.bootstrap = newObserver(o.token)
	healthToken := bytes.Clone(o.token)
	healthToken[15] ^= byte(phase)
	r.health = newObserver(healthToken)
	r.flows = [4]localFlow{}
	for i := range r.flows {
		f := &r.flows[i]
		f.source = uint16(30000 + phase*10 + i + 1)
		f.destination = dutIP
		if i%2 == 1 {
			f.destination = hostIP
		}
		f.proto = 6
		f.port = r.tcpPort
		if i >= 2 {
			f.proto = 17
			f.port = r.udpPort
		}
		copy(f.payload[:16], o.token)
		f.payload[16] = byte(phase)
		f.payload[17] = byte(i + 1)
	}
	o.broadcast()
	return nil
}

func (r *rejectObserver) inspect(o *observer, p packet, raw []byte, incoming bool) {
	if !r.active {
		o.err = errors.New("packet outside phase")
		return
	}
	if p.proto == 1 {
		if incoming && p.body[0] == 3 && p.body[1] == 3 {
			// ICMP Port Unreachable 只能引用本阶段已发送的精确流，不能冒充健康 Echo。
			quoted := p.body[8:]
			if p.dst != peerIP || (p.src != dutIP && p.src != hostIP) || len(quoted) < 28 || quoted[0]>>4 != 4 {
				o.err = errors.New("ICMP quote")
				return
			}
			h := int(quoted[0]&15) * 4
			if h < 20 || h+8 > len(quoted) || checksum(quoted[:h], 0) != 0 {
				o.err = errors.New("ICMP quote bounds")
				return
			}
			for i := range r.flows {
				f := &r.flows[i]
				if f.started && f.sent && quoted[9] == f.proto && [4]byte(quoted[12:16]) == peerIP && [4]byte(quoted[16:20]) == f.destination && binary.BigEndian.Uint16(quoted[h:h+2]) == f.source && binary.BigEndian.Uint16(quoted[h+2:h+4]) == f.port {
					return
				}
			}
			o.err = errors.New("ICMP quote tuple")
			return
		}
		r.health.inspect(raw, incoming)
		_, _, o.err = r.health.result()
		return
	}
	source, dest := binary.BigEndian.Uint16(p.body[:2]), binary.BigEndian.Uint16(p.body[2:4])
	if p.proto == 6 && ((incoming && dest == 18080) || (!incoming && source == 18080)) {
		r.bootstrap.inspect(raw, incoming)
		_, _, o.err = r.bootstrap.result()
		return
	}
	for i := range r.flows {
		f := &r.flows[i]
		if f.proto != p.proto {
			continue
		}
		if incoming {
			if p.src != f.destination || p.dst != peerIP || source != f.port || dest != f.source {
				continue
			}
		} else if p.src != peerIP || p.dst != f.destination || source != f.source || dest != f.port {
			continue
		}
		if !f.started {
			o.err = errors.New("flow not started")
			return
		}
		if p.proto == 17 {
			if !bytes.Equal(p.body[8:], f.payload[:]) {
				o.err = errors.New("UDP phase payload")
				return
			}
			if incoming {
				f.udpRX++
				if f.udpRX != 1 {
					o.err = errors.New("extra UDP echo")
				}
			} else {
				f.udpTX++
				if f.udpTX != 1 {
					o.err = errors.New("extra UDP request")
				} else {
					f.sent = true
				}
			}
			return
		}
		b := p.body
		side := 0
		if incoming {
			side = 1
		}
		seq := binary.BigEndian.Uint32(b[4:8])
		flags := b[13]
		if flags&2 != 0 {
			if f.haveStart[side] && f.start[side] != seq+1 {
				o.err = errors.New("flow SYN changed")
				return
			}
			f.start[side] = seq + 1
			f.haveStart[side] = true
			if !incoming && flags&16 == 0 {
				f.sent = true
			}
			seq++
		}
		payload := b[int(b[12]>>4)*4:]
		if len(payload) > 0 {
			delta := uint32(seq - f.start[side])
			if !f.haveStart[side] || delta >= 20 || uint64(delta)+uint64(len(payload)) > 20 {
				o.err = errors.New("TCP phase range")
				return
			}
			for j, v := range payload {
				index := int(delta) + j
				if v != f.payload[index] {
					o.err = errors.New("TCP phase payload")
					return
				}
				f.covered[side][index] = true
			}
		}
		return // 合法 RST/FIN/ACK 由连接和结果分类处理，不能当应用回显。
	}
	o.err = errors.New("unknown phase tuple")
}

func (o *observer) localFlowResult(index int, equal bool) (bool, error) {
	o.mu.Lock()
	defer o.mu.Unlock()
	if o.err != nil {
		return false, o.err
	}
	f := &o.reject.flows[index]
	if !f.sent {
		return false, errors.New("no tun submission")
	}
	if equal {
		if f.proto == 17 {
			if f.udpTX != 1 || f.udpRX != 1 {
				return false, errors.New("missing UDP boundary")
			}
		} else {
			for _, side := range f.covered {
				for _, covered := range side {
					if !covered {
						return false, errors.New("missing TCP boundary")
					}
				}
			}
		}
	}
	return true, nil
}
