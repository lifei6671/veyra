package main

import (
	"bytes"
	"context"
	"encoding/binary"
	"errors"
	"sync"
)

const response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"

var dutIP = [4]byte{198, 18, 0, 1}
var peerIP = [4]byte{198, 18, 0, 2}

type packet struct {
	src, dst [4]byte
	proto    byte
	body     []byte
}

func checksum(b []byte, sum uint32) uint16 {
	for len(b) >= 2 {
		sum += uint32(binary.BigEndian.Uint16(b))
		b = b[2:]
	}
	if len(b) == 1 {
		sum += uint32(b[0]) << 8
	}
	for sum>>16 != 0 {
		sum = (sum & 65535) + (sum >> 16)
	}
	return ^uint16(sum)
}

func parsePacket(raw []byte) (packet, error) {
	if len(raw) < 20 || raw[0]>>4 != 4 {
		return packet{}, errors.New("IPv4 header")
	}
	h := int(raw[0]&15) * 4
	n := int(binary.BigEndian.Uint16(raw[2:4]))
	if h < 20 || h > len(raw) || n < h || n > len(raw) || binary.BigEndian.Uint16(raw[6:8])&0x3fff != 0 || checksum(raw[:h], 0) != 0 {
		return packet{}, errors.New("IPv4 bounds/checksum")
	}
	p := packet{src: [4]byte(raw[12:16]), dst: [4]byte(raw[16:20]), proto: raw[9], body: raw[h:n]}
	switch p.proto {
	case 6:
		if len(p.body) < 20 || int(p.body[12]>>4)*4 < 20 || int(p.body[12]>>4)*4 > len(p.body) {
			return packet{}, errors.New("TCP bounds")
		}
		pseudo := uint32(6 + len(p.body))
		for _, addr := range [][4]byte{p.src, p.dst} {
			pseudo += uint32(binary.BigEndian.Uint16(addr[:2])) + uint32(binary.BigEndian.Uint16(addr[2:]))
		}
		if checksum(p.body, pseudo) != 0 {
			return packet{}, errors.New("TCP checksum")
		}
	case 1:
		if len(p.body) < 8 || checksum(p.body, 0) != 0 {
			return packet{}, errors.New("ICMP bounds/checksum")
		}
	case 17:
		if len(p.body) < 8 || int(binary.BigEndian.Uint16(p.body[4:6])) != len(p.body) {
			return packet{}, errors.New("UDP bounds")
		}
		if binary.BigEndian.Uint16(p.body[6:8]) != 0 {
			pseudo := uint32(17 + len(p.body))
			for _, a := range [][4]byte{p.src, p.dst} {
				pseudo += uint32(binary.BigEndian.Uint16(a[:2])) + uint32(binary.BigEndian.Uint16(a[2:]))
			}
			if checksum(p.body, pseudo) != 0 {
				return packet{}, errors.New("UDP checksum")
			}
		}
	default:
		return packet{}, errors.New("protocol")
	}
	return p, nil
}

// 只有确认同一 TCP 连接的完整响应覆盖和远端累计 ACK，才满足阳性条件。
type observer struct {
	mu           sync.Mutex
	wake         chan struct{}
	changed      chan struct{}
	err          error
	total        uint64
	rx, tx       uint64
	clientPort   uint16
	start        uint32
	haveStart    bool
	covered      [len(response)]bool
	fin          bool
	acks         []uint32
	acked        bool
	token        []byte
	icmpActive   bool
	icmpSent     int
	icmpReceived int
	allowUDP     bool
	udpMode      bool
	udpReceived  int
	udpReplied   int
	udpPort      uint16
	reject       *rejectObserver
}

func newObserver(token []byte) *observer {
	return &observer{wake: make(chan struct{}, 1), changed: make(chan struct{}), token: bytes.Clone(token)}
}
func (o *observer) signal() {
	select {
	case o.wake <- struct{}{}:
	default:
	}
}
func (o *observer) fail(err error) {
	o.mu.Lock()
	if o.err == nil {
		o.err = err
	}
	o.broadcast()
	o.mu.Unlock()
}
func (o *observer) broadcast() { close(o.changed); o.changed = make(chan struct{}); o.signal() }
func (o *observer) inspect(raw []byte, incoming bool) {
	p, err := parsePacket(raw)
	o.mu.Lock()
	defer o.mu.Unlock()
	defer o.broadcast()
	if o.err != nil {
		return
	}
	if err != nil {
		o.err = err
		return
	}
	o.total += uint64(len(raw))
	if o.total > 1<<20 {
		o.err = errors.New("byte limit")
		return
	}
	if o.reject != nil {
		o.reject.inspect(o, p, raw, incoming)
		return
	}
	if (incoming && (p.src != dutIP || p.dst != peerIP)) || (!incoming && (p.src != peerIP || p.dst != dutIP)) {
		o.err = errors.New("packet address")
		return
	}
	if o.udpMode {
		if p.proto != 17 {
			o.err = errors.New("UDP scenario protocol")
			return
		}
		o.udp(p, incoming)
		return
	}
	switch p.proto {
	case 6:
		o.tcp(p, incoming)
	case 1:
		o.icmp(p, incoming)
	case 17:
		if !o.allowUDP {
			o.err = errors.New("unexpected live UDP")
		}
	default:
		o.err = errors.New("unexpected live protocol")
	}
}
func (o *observer) tcp(p packet, incoming bool) {
	b := p.body
	source, dest := binary.BigEndian.Uint16(b), binary.BigEndian.Uint16(b[2:])
	flags := b[13]
	if incoming {
		o.rx++
		if o.clientPort == 0 && dest == 18080 && flags&2 != 0 && flags&16 == 0 {
			o.clientPort = source
		}
		if source != o.clientPort || dest != 18080 {
			o.err = errors.New("TCP tuple")
			return
		}
	} else {
		o.tx++
		if source != 18080 || dest != o.clientPort {
			o.err = errors.New("TCP tuple")
			return
		}
	}
	if flags&4 != 0 {
		o.err = errors.New("TCP reset")
		return
	}
	seq := binary.BigEndian.Uint32(b[4:8])
	if !incoming {
		if flags&2 != 0 {
			if o.haveStart && o.start != seq+1 {
				o.err = errors.New("SYN changed")
				return
			}
			o.start, o.haveStart = seq+1, true
		}
		payload := b[int(b[12]>>4)*4:]
		if len(payload) != 0 {
			if !o.haveStart {
				o.err = errors.New("missing SYN")
				return
			}
			if flags&2 != 0 {
				seq++
			}
			delta := uint32(seq - o.start)
			if delta >= 1<<31 || uint64(delta)+uint64(len(payload)) > uint64(len(response)) {
				o.err = errors.New("response range")
				return
			}
			for i, value := range payload {
				index := int(delta) + i
				if value != response[index] {
					o.err = errors.New("response bytes")
					return
				}
				o.covered[index] = true
			}
		}
		if flags&1 != 0 && o.haveStart {
			if uint32(seq-o.start)+uint32(len(payload)) != uint32(len(response)) {
				o.err = errors.New("FIN range")
				return
			}
			o.fin = true
		}
	} else if flags&16 != 0 {
		if len(o.acks) >= 128 {
			o.err = errors.New("ACK limit")
			return
		}
		o.acks = append(o.acks, binary.BigEndian.Uint32(b[8:12]))
	}
	o.checkACK()
}
func (o *observer) checkACK() {
	if !o.haveStart {
		return
	}
	for _, covered := range o.covered {
		if !covered {
			return
		}
	}
	end := o.start + uint32(len(response))
	limit := end
	if o.fin {
		limit++
	}
	for _, ack := range o.acks {
		if uint32(ack-end) < 1<<31 && uint32(limit-ack) < 1<<31 {
			o.acked = true
		}
	}
}
func (o *observer) icmp(p packet, incoming bool) {
	b := p.body
	if !o.icmpActive || b[1] != 0 || binary.BigEndian.Uint16(b[4:6]) != 9 || len(b) != 8+len(o.token)+1 || !bytes.Equal(b[8:len(b)-1], o.token) {
		o.err = errors.New("ICMP content")
		return
	}
	seq := int(binary.BigEndian.Uint16(b[6:8]))
	if seq < 1 || seq > 3 || b[len(b)-1] != byte(seq) {
		o.err = errors.New("ICMP sequence")
		return
	}
	if incoming {
		if b[0] != 0 || seq != o.icmpReceived+1 || seq > o.icmpSent {
			o.err = errors.New("ICMP reply")
			return
		}
		o.icmpReceived++
	} else {
		if b[0] != 8 || seq != o.icmpSent+1 || o.icmpSent != o.icmpReceived {
			o.err = errors.New("ICMP request")
			return
		}
		o.icmpSent++
	}
}
func (o *observer) waitACK(ctx context.Context) error {
	for {
		if err := ctx.Err(); err != nil {
			return err
		}
		o.mu.Lock()
		err, acked, changed := o.err, o.acked, o.changed
		o.mu.Unlock()
		if err != nil {
			return err
		}
		if acked {
			return nil
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-changed:
		}
	}
}
func (o *observer) result() (uint64, uint64, error) {
	o.mu.Lock()
	defer o.mu.Unlock()
	return o.rx, o.tx, o.err
}
