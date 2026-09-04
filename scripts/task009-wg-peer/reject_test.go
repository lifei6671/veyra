package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/binary"
	"encoding/json"
	"errors"
	"io"
	"net"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"

	"github.com/sagernet/gvisor/pkg/buffer"
	"github.com/sagernet/gvisor/pkg/tcpip"
	"github.com/sagernet/gvisor/pkg/tcpip/adapters/gonet"
	"github.com/sagernet/gvisor/pkg/tcpip/network/ipv4"
	"github.com/sagernet/gvisor/pkg/tcpip/stack"
)

func rejectInitFrame() []byte {
	var fields map[string]any
	_ = json.Unmarshal(initFrame(), &fields)
	fields["op"] = "init_reject"
	fields["tcp_port"] = 41001
	fields["udp_port"] = 41002
	b, _ := json.Marshal(fields)
	return append(b, '\n')
}
func TestRejectProtocolStrictFields(t *testing.T) {
	good := rejectInitFrame()
	if _, err := decodeCommand(good); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"zero_port", "same_port", "api_port", "missing_port", "phase_on_init", "old_with_port", "missing_phase", "extra_phase", "false_stop", "extra_empty_token"} {
		t.Run(name, func(t *testing.T) {
			var fields map[string]any
			_ = json.Unmarshal(good, &fields)
			switch name {
			case "zero_port":
				fields["tcp_port"] = 0
			case "same_port":
				fields["tcp_port"] = 41002
			case "api_port":
				fields["udp_port"] = 9090
			case "missing_port":
				delete(fields, "udp_port")
			case "phase_on_init":
				fields["phase"] = 1
			case "old_with_port":
				fields["op"] = "init"
			default:
				fields = map[string]any{"v": 1, "run_id": strings.Repeat("a", 32), "op": "probe_local"}
				switch name {
				case "missing_phase":
					fields["op"] = "begin_phase"
				case "extra_phase":
					fields["phase"] = 1
				case "false_stop":
					fields["op"] = "finish_phase"
					fields["dut_stopped"] = false
				case "extra_empty_token":
					fields["token"] = ""
				}
			}
			b, _ := json.Marshal(fields)
			if _, err := decodeCommand(b); err == nil {
				t.Fatal("invalid frame accepted")
			}
		})
	}
}

func rejectWatch(t *testing.T) *observer {
	t.Helper()
	o := newObserver([]byte("0123456789abcdef"))
	o.reject = &rejectObserver{tcpPort: 41001, udpPort: 41002}
	if err := o.beginRejectPhase(1); err != nil {
		t.Fatal(err)
	}
	return o
}
func localPacket(f localFlow, incoming bool, seq uint32, flags byte, payload []byte) []byte {
	headerSize := 20
	if f.proto == 17 {
		headerSize = 8
	}
	b := make([]byte, 20+headerSize+len(payload))
	b[0] = 0x45
	b[8] = 64
	b[9] = f.proto
	binary.BigEndian.PutUint16(b[2:4], uint16(len(b)))
	src, dst, sp, dp := peerIP, f.destination, f.source, f.port
	if incoming {
		src, dst, sp, dp = dst, src, dp, sp
	}
	copy(b[12:16], src[:])
	copy(b[16:20], dst[:])
	binary.BigEndian.PutUint16(b[10:12], checksum(b[:20], 0))
	h := b[20:]
	binary.BigEndian.PutUint16(h[:2], sp)
	binary.BigEndian.PutUint16(h[2:4], dp)
	if f.proto == 6 {
		h[12] = 0x50
		h[13] = flags
		binary.BigEndian.PutUint32(h[4:8], seq)
	} else {
		binary.BigEndian.PutUint16(h[4:6], uint16(len(h)))
	}
	copy(h[headerSize:], payload)
	pseudo := uint32(f.proto) + uint32(len(h))
	for _, a := range [][4]byte{src, dst} {
		pseudo += uint32(binary.BigEndian.Uint16(a[:2])) + uint32(binary.BigEndian.Uint16(a[2:]))
	}
	sum := checksum(h, pseudo)
	at := 16
	if f.proto == 17 {
		at = 6
		if sum == 0 {
			sum = 65535
		}
	}
	binary.BigEndian.PutUint16(h[at:at+2], sum)
	return b
}
func TestRejectObservationRequiresActualFlowAndExactEcho(t *testing.T) {
	for _, name := range []string{"tcp", "udp", "no_send", "not_started", "wrong_phase", "wrong_tuple", "wrong_checksum", "missing_reply", "late_phase", "old_icmp"} {
		t.Run(name, func(t *testing.T) {
			o := rejectWatch(t)
			index := 0
			if name == "udp" || name == "wrong_phase" {
				index = 2
			}
			f := &o.reject.flows[index]
			f.started = name != "not_started"
			sample := *f
			if name == "old_icmp" {
				oldToken := bytes.Clone(o.reject.health.token)
				o.reject.active = false
				if err := o.beginRejectPhase(2); err != nil {
					t.Fatal(err)
				}
				if bytes.Equal(oldToken, o.reject.health.token) {
					t.Fatal("ICMP phase token reused")
				}
				o.reject.health.icmpActive = true
				body := make([]byte, 25)
				binary.BigEndian.PutUint16(body[4:6], 9)
				binary.BigEndian.PutUint16(body[6:8], 1)
				copy(body[8:], oldToken)
				body[24] = 1
				binary.BigEndian.PutUint16(body[2:4], checksum(body, 0))
				raw := make([]byte, 45)
				raw[0] = 0x45
				raw[8] = 64
				raw[9] = 1
				binary.BigEndian.PutUint16(raw[2:4], 45)
				copy(raw[12:16], dutIP[:])
				copy(raw[16:20], peerIP[:])
				binary.BigEndian.PutUint16(raw[10:12], checksum(raw[:20], 0))
				copy(raw[20:], body)
				o.inspect(raw, true)
				if _, _, err := o.result(); err == nil {
					t.Fatal("old phase ICMP accepted")
				}
				return
			}
			if name != "no_send" {
				payload := []byte(nil)
				if sample.proto == 17 {
					payload = sample.payload[:]
				}
				raw := localPacket(sample, false, 10, 2, payload)
				if name == "wrong_phase" {
					bad := sample
					bad.payload[16] = 2
					raw = localPacket(bad, false, 10, 2, bad.payload[:])
				}
				if name == "wrong_tuple" {
					bad := sample
					bad.destination = [4]byte{198, 18, 0, 3}
					raw = localPacket(bad, false, 10, 2, nil)
				}
				if name == "wrong_checksum" {
					raw[10] ^= 1
				}
				if name == "late_phase" {
					o.reject.active = false
				}
				o.inspect(raw, false)
				if name == "tcp" {
					o.inspect(localPacket(sample, true, 20, 18, nil), true)
					o.inspect(localPacket(sample, false, 11, 16, sample.payload[:]), false)
					o.inspect(localPacket(sample, true, 21, 16, sample.payload[:]), true)
				}
				if name == "udp" {
					o.inspect(localPacket(sample, true, 0, 0, sample.payload[:]), true)
				}
			}
			_, err := o.localFlowResult(index, true)
			positive := name == "tcp" || name == "udp"
			if (err == nil) != positive {
				t.Fatalf("observation success=%v expected %v", err == nil, positive)
			}
		})
	}
}

func TestRejectPhaseResetRequiresFreshBootstrap(t *testing.T) {
	o := rejectWatch(t)
	o.reject.bootstrap.acked = true
	o.reject.bootstrap.rx = 4
	o.reject.health.icmpSent = 3
	o.reject.health.icmpReceived = 3
	o.reject.flows[0].sent = true
	o.total = 123
	if err := o.beginRejectPhase(2); err == nil {
		t.Fatal("phase advanced while DUT active")
	}
	o.reject.active = false
	if err := o.beginRejectPhase(3); err == nil {
		t.Fatal("phase skipped")
	}
	if err := o.beginRejectPhase(2); err != nil {
		t.Fatal(err)
	}
	if o.reject.bootstrap.acked || o.reject.bootstrap.rx != 0 || o.reject.health.icmpSent != 0 || o.reject.health.icmpActive || o.reject.flows[0].sent || o.total != 123 {
		t.Fatal("proof leaked across phases or lifetime limit reset")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Millisecond)
	defer cancel()
	if err := o.reject.bootstrap.waitACK(ctx); !errors.Is(err, context.DeadlineExceeded) {
		t.Fatal("new bootstrap inherited prior ACK")
	}
}

func TestRejectErrorClassification(t *testing.T) {
	cases := []struct {
		err  error
		want string
	}{{nil, "none"}, {syscall.ECONNREFUSED, "refused"}, {syscall.ECONNRESET, "reset"}, {io.EOF, "eof"}, {context.DeadlineExceeded, "timeout"}}
	for _, c := range cases {
		got, err := localError(c.err)
		if err != nil || got != c.want {
			t.Fatal(got, err)
		}
	}
	for _, err := range []error{context.Canceled, io.ErrUnexpectedEOF, syscall.EHOSTUNREACH, syscall.EADDRINUSE, errors.New("decode")} {
		if _, fault := localError(err); fault == nil {
			t.Fatal("environment error accepted as rejection")
		}
	}
}

// 两个纯内存栈模拟目标可用/不可用，绝不启动真实 DUT 或访问宿主 listener。
func TestRejectMemoryFourCasesAndHealth(t *testing.T) {
	for _, open := range []bool{true, false} {
		t.Run(map[bool]string{true: "targets_open", false: "targets_closed"}[open], func(t *testing.T) {
			ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
			defer cancel()
			o := rejectWatch(t)
			a, err := newMemoryTun(peerIP, o)
			if err != nil {
				t.Fatal(err)
			}
			targetOptions := rejectWatch(t)
			b, err := newMemoryTun(dutIP, targetOptions)
			if err != nil {
				a.Close()
				t.Fatal(err)
			}
			b.watch = nil
			if err := b.s.AddProtocolAddress(1, tcpip.ProtocolAddress{Protocol: ipv4.ProtocolNumber, AddressWithPrefix: tcpip.AddrFrom4(hostIP).WithPrefix()}, stack.AddressProperties{}); err != nil {
				t.Fatal(err.String())
			}
			var workers sync.WaitGroup
			var closers []io.Closer
			workers.Add(2)
			bridge := func(from, to *memoryTun) {
				defer workers.Done()
				for {
					pkt := from.link.ReadContext(ctx)
					if pkt == nil {
						return
					}
					var raw []byte
					for _, s := range pkt.AsSlices() {
						raw = append(raw, s...)
					}
					pkt.DecRef()
					if from.watch != nil {
						from.watch.inspect(raw, false)
					}
					if to.watch != nil {
						to.watch.inspect(raw, true)
					}
					v := stack.NewPacketBuffer(stack.PacketBufferOptions{Payload: buffer.MakeWithData(raw)})
					to.link.InjectInbound(ipv4.ProtocolNumber, v)
					v.DecRef()
				}
			}
			go bridge(a, b)
			go bridge(b, a)
			t.Cleanup(func() {
				cancel()
				for _, c := range closers {
					c.Close()
				}
				a.Close()
				b.Close()
				workers.Wait()
			})
			served := make(chan bool, 4)
			if open {
				for _, ip := range [][4]byte{dutIP, hostIP} {
					listener, err := gonet.ListenTCP(b.s, address(ip, 41001), ipv4.ProtocolNumber)
					if err != nil {
						t.Fatal(err)
					}
					closers = append(closers, listener)
					workers.Add(1)
					go func() {
						defer workers.Done()
						c, err := listener.Accept()
						if err != nil {
							return
						}
						defer c.Close()
						c.SetDeadline(deadline(ctx))
						var payload [20]byte
						_, err = io.ReadFull(c, payload[:])
						if err != nil {
							served <- false
							return
						}
						n, err := c.Write(payload[:])
						served <- err == nil && n == 20
					}()
					local := address(ip, 41002)
					u, err := gonet.DialUDP(b.s, &local, nil, ipv4.ProtocolNumber)
					if err != nil {
						t.Fatal(err)
					}
					closers = append(closers, u)
					workers.Add(1)
					go func() {
						defer workers.Done()
						u.SetDeadline(deadline(ctx))
						var payload [21]byte
						n, remote, err := u.ReadFrom(payload[:])
						if err != nil {
							served <- false
							return
						}
						written, err := u.WriteTo(payload[:n], remote)
						served <- err == nil && written == 20
					}()
				}
			}
			results, err := probeLocal(ctx, &livePeer{tun: a, watch: o})
			if err != nil {
				t.Fatal(err)
			}
			if len(results) != 4 {
				t.Fatal("case count")
			}
			for _, r := range results {
				if !r.Sent || r.EqualEcho != open || (r.Error == "none") != open {
					t.Fatalf("unexpected result %#v", r)
				}
			}
			if open {
				for i := 0; i < 4; i++ {
					if !<-served {
						t.Fatal("target did not receive/echo")
					}
				}
			}
			if o.reject.health.icmpSent != 3 || o.reject.health.icmpReceived != 3 {
				t.Fatal("health missing")
			}
		})
	}
}

func TestRejectMissingBootstrapFailsAndHolds(t *testing.T) {
	input, writer := io.Pipe()
	reader, output := io.Pipe()
	done := make(chan int, 1)
	go func() { done <- run(input, output); output.Close() }()
	defer writer.Close()
	defer input.Close()
	defer reader.Close()
	if _, err := writer.Write(rejectInitFrame()); err != nil {
		t.Fatal(err)
	}
	r := bufio.NewReader(reader)
	read := func() map[string]json.RawMessage {
		t.Helper()
		line, err := r.ReadBytes('\n')
		if err != nil {
			t.Fatal(err)
		}
		var e map[string]json.RawMessage
		if err = json.Unmarshal(line, &e); err != nil {
			t.Fatal(err)
		}
		return e
	}
	ready := read()
	if string(ready["event"]) != `"ready"` {
		t.Fatal("not ready")
	}
	var port int
	_ = json.Unmarshal(ready["udp_port"], &port)
	for i, frame := range []string{`{"v":1,"op":"begin_phase","run_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","phase":1}`, `{"v":1,"op":"probe_local","run_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}`} {
		if _, err := io.WriteString(writer, frame+"\n"); err != nil {
			t.Fatal(err)
		}
		e := read()
		want := []string{`"phase_ready"`, `"failed"`}[i]
		if string(e["event"]) != want {
			t.Fatal("unexpected event")
		}
		if i == 1 && (string(e["stage"]) != `"protocol"` || string(e["code"]) != `"invalid_input"`) {
			t.Fatal("missing bootstrap failure category")
		}
	}
	socket, err := net.ListenUDP("udp4", &net.UDPAddr{IP: net.IPv4(127, 0, 0, 1), Port: port})
	if err == nil {
		socket.Close()
		t.Fatal("failure released peer")
	}
	if _, err := io.WriteString(writer, `{"v":1,"op":"shutdown","run_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","dut_stopped":true}`+"\n"); err != nil {
		t.Fatal(err)
	}
	if string(read()["event"]) != `"stopped"` {
		t.Fatal("cleanup missing")
	}
	select {
	case exit := <-done:
		if exit == 0 {
			t.Fatal("missing bootstrap reported success")
		}
	case <-time.After(3 * time.Second):
		t.Fatal("cleanup hung")
	}
}
