package main

import (
	"bytes"
	"context"
	"encoding/binary"
	"errors"
	"sync"
	"testing"
	"time"

	"github.com/sagernet/gvisor/pkg/buffer"
	"github.com/sagernet/gvisor/pkg/tcpip/adapters/gonet"
	"github.com/sagernet/gvisor/pkg/tcpip/network/ipv4"
	"github.com/sagernet/gvisor/pkg/tcpip/stack"
)

func udpPayload(token []byte, seq uint32) []byte {
	b := make([]byte, 20)
	copy(b, token)
	binary.BigEndian.PutUint32(b[16:], seq)
	return b
}
func udpPacket(incoming bool, port uint16, payload []byte, withChecksum bool) []byte {
	b := make([]byte, 28+len(payload))
	b[0] = 0x45
	b[8] = 64
	b[9] = 17
	binary.BigEndian.PutUint16(b[2:4], uint16(len(b)))
	src, dst := peerIP, dutIP
	sp, dp := uint16(18081), port
	if incoming {
		src, dst = dutIP, peerIP
		sp, dp = dp, sp
	}
	copy(b[12:16], src[:])
	copy(b[16:20], dst[:])
	binary.BigEndian.PutUint16(b[10:12], checksum(b[:20], 0))
	h := b[20:]
	binary.BigEndian.PutUint16(h[:2], sp)
	binary.BigEndian.PutUint16(h[2:4], dp)
	binary.BigEndian.PutUint16(h[4:6], uint16(len(h)))
	copy(h[8:], payload)
	if withChecksum {
		pseudo := uint32(17 + len(h))
		for _, a := range [][4]byte{src, dst} {
			pseudo += uint32(binary.BigEndian.Uint16(a[:2])) + uint32(binary.BigEndian.Uint16(a[2:]))
		}
		sum := checksum(h, pseudo)
		if sum == 0 {
			sum = 65535
		}
		binary.BigEndian.PutUint16(h[6:8], sum)
	}
	return b
}
func TestUDPBoundaryValidation(t *testing.T) {
	token := []byte("0123456789abcdef")
	for _, name := range []string{"three_replies", "zero_checksum", "wrong_token", "wrong_sequence", "wrong_port", "wrong_source", "truncated", "oversize", "wrong_checksum", "fragment", "requests_only", "reply_without_request", "extra_datagram", "tcp_in_udp", "udp_in_tcp"} {
		t.Run(name, func(t *testing.T) {
			o := newObserver(token)
			o.udpMode = name != "udp_in_tcp"
			if name == "three_replies" || name == "zero_checksum" || name == "extra_datagram" {
				for seq := uint32(1); seq <= 3; seq++ {
					o.inspect(udpPacket(true, 40000, udpPayload(token, seq), name != "zero_checksum"), true)
					o.inspect(udpPacket(false, 40000, udpPayload(token, seq), name != "zero_checksum"), false)
				}
				if name == "extra_datagram" {
					o.inspect(udpPacket(true, 40000, udpPayload(token, 4), true), true)
				}
			} else {
				payload := udpPayload(token, 1)
				if name == "wrong_token" {
					payload[0] ^= 1
				}
				if name == "wrong_sequence" {
					payload[19] = 2
				}
				if name == "oversize" {
					payload = append(payload, 0)
				}
				p := udpPacket(true, 40000, payload, true)
				switch name {
				case "wrong_port":
					o.inspect(p, true)
					p = udpPacket(false, 40001, payload, true)
				case "wrong_source":
					p[15] = 3
					p[10], p[11] = 0, 0
					p[26], p[27] = 0, 0
					binary.BigEndian.PutUint16(p[10:12], checksum(p[:20], 0))
				case "truncated":
					p = p[:len(p)-1]
				case "wrong_checksum":
					p[26] ^= 1
				case "fragment":
					p[6] = 0x20
					p[10], p[11] = 0, 0
					binary.BigEndian.PutUint16(p[10:12], checksum(p[:20], 0))
				case "reply_without_request":
					p = udpPacket(false, 40000, payload, true)
				case "tcp_in_udp":
					p = tcpPacket(true, 1, 0, 2, nil)
				}
				o.inspect(p, name != "wrong_port" && name != "reply_without_request")
			}
			ctx, cancel := context.WithTimeout(context.Background(), 10*time.Millisecond)
			defer cancel()
			err := o.waitUDP(ctx)
			positive := name == "three_replies" || name == "zero_checksum"
			if (err == nil) != positive {
				t.Fatalf("UDP boundary positive=%v expected %v", err == nil, positive)
			}
			if positive && (o.udpReceived != 3 || o.udpReplied != 3) {
				t.Fatal("incorrect boundary count")
			}
		})
	}
}

func memoryUDPPair(t *testing.T, captureOutbound bool) (context.Context, *livePeer, *memoryTun) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	w := newObserver([]byte("0123456789abcdef"))
	w.udpMode = true
	a, err := newMemoryTun(peerIP, w)
	if err != nil {
		cancel()
		t.Fatal(err)
	}
	b, err := newMemoryTun(dutIP, nil)
	if err != nil {
		cancel()
		a.Close()
		t.Fatal(err)
	}
	u, err := newUDPService(a)
	if err != nil {
		cancel()
		a.Close()
		b.Close()
		t.Fatal(err)
	}
	p := &livePeer{tun: a, udp: u, watch: w}
	var bridges sync.WaitGroup
	bridges.Add(2)
	bridge := func(from, to *memoryTun) {
		defer bridges.Done()
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
			if from.watch != nil && captureOutbound {
				from.watch.inspect(raw, false)
			}
			if to.watch != nil {
				to.watch.inspect(raw, true)
			}
			received := stack.NewPacketBuffer(stack.PacketBufferOptions{Payload: buffer.MakeWithData(raw)})
			to.link.InjectInbound(ipv4.ProtocolNumber, received)
			received.DecRef()
		}
	}
	go bridge(a, b)
	go bridge(b, a)
	t.Cleanup(func() { cancel(); u.close(); a.Close(); b.Close(); bridges.Wait() })
	return ctx, p, b
}
func TestMemoryUDPServiceRequiresBoundaryReplies(t *testing.T) {
	for _, capture := range []bool{true, false} {
		t.Run(map[bool]string{true: "full_boundary", false: "submission_without_boundary"}[capture], func(t *testing.T) {
			ctx, p, clientStack := memoryUDPPair(t, capture)
			done := make(chan error, 1)
			go func() { done <- serveUDP(ctx, p) }()
			local, remote := address(dutIP, 40000), address(peerIP, 18081)
			client, err := gonet.DialUDP(clientStack.s, &local, &remote, ipv4.ProtocolNumber)
			if err != nil {
				t.Fatal(err)
			}
			defer client.Close()
			if err = client.SetDeadline(deadline(ctx)); err != nil {
				t.Fatal(err)
			}
			for seq := uint32(1); seq <= 3; seq++ {
				want := udpPayload(p.watch.token, seq)
				if n, err := client.Write(want); err != nil || n != 20 {
					t.Fatal("send", err)
				}
				var got [21]byte
				n, err := client.Read(got[:])
				if err != nil || !bytes.Equal(got[:n], want) {
					t.Fatal("echo", err)
				}
			}
			select {
			case err := <-done:
				if (err == nil) != capture {
					t.Fatalf("boundary proof=%v expected %v", err == nil, capture)
				}
			case <-ctx.Done():
				t.Fatal("service did not finish")
			}
		})
	}
}
func TestUDPServiceRejectsInvalidPayloadAndChangedSource(t *testing.T) {
	for _, name := range []string{"token", "order", "short", "long", "source_port"} {
		t.Run(name, func(t *testing.T) {
			ctx, p, clientStack := memoryUDPPair(t, true)
			done := make(chan error, 1)
			go func() { done <- serveUDP(ctx, p) }()
			local, remote := address(dutIP, 40000), address(peerIP, 18081)
			client, err := gonet.DialUDP(clientStack.s, &local, &remote, ipv4.ProtocolNumber)
			if err != nil {
				t.Fatal(err)
			}
			defer client.Close()
			payload := udpPayload(p.watch.token, 1)
			if name == "source_port" {
				client.SetDeadline(deadline(ctx))
				if _, err = client.Write(payload); err != nil {
					t.Fatal(err)
				}
				var reply [21]byte
				if n, err := client.Read(reply[:]); err != nil || !bytes.Equal(reply[:n], payload) {
					t.Fatal("first valid request", err)
				}
				local = address(dutIP, 40001)
				second, err := gonet.DialUDP(clientStack.s, &local, &remote, ipv4.ProtocolNumber)
				if err != nil {
					t.Fatal(err)
				}
				defer second.Close()
				client = second
				payload = udpPayload(p.watch.token, 2)
			} else {
				switch name {
				case "token":
					payload[0] ^= 1
				case "order":
					payload[19] = 2
				case "short":
					payload = payload[:19]
				case "long":
					payload = append(payload, 0)
				}
			}
			if err = client.SetDeadline(deadline(ctx)); err != nil {
				t.Fatal(err)
			}
			if _, err = client.Write(payload); err != nil {
				t.Fatal(err)
			}
			select {
			case err := <-done:
				if err == nil {
					t.Fatal("invalid request accepted")
				}
			case <-ctx.Done():
				t.Fatal("service did not reject")
			}
			client.SetReadDeadline(time.Now().Add(20 * time.Millisecond))
			var b [21]byte
			if _, err = client.Read(b[:]); err == nil {
				t.Fatal("invalid request echoed")
			}
		})
	}
}
func TestUDPReadinessCancellationPreservesEndpoint(t *testing.T) {
	_, p, _ := memoryUDPPair(t, true)
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if err := p.udp.waitReadable(ctx); !errors.Is(err, context.Canceled) {
		t.Fatal("cancel not propagated", err)
	}
	if _, err := p.udp.endpoint.GetLocalAddress(); err != nil {
		t.Fatal("cancellation closed UDP endpoint")
	}
}
