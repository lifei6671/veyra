package main

import (
	"context"
	"encoding/binary"
	"testing"
	"time"
)

func tcpPacket(incoming bool, seq, ack uint32, flags byte, payload []byte) []byte {
	b := make([]byte, 40+len(payload))
	b[0] = 0x45
	binary.BigEndian.PutUint16(b[2:4], uint16(len(b)))
	b[8] = 64
	b[9] = 6
	src, dst := peerIP, dutIP
	sp, dp := uint16(18080), uint16(40000)
	if incoming {
		src, dst = dutIP, peerIP
		sp, dp = dp, sp
	}
	copy(b[12:16], src[:])
	copy(b[16:20], dst[:])
	binary.BigEndian.PutUint16(b[10:12], checksum(b[:20], 0))
	h := b[20:]
	binary.BigEndian.PutUint16(h[0:2], sp)
	binary.BigEndian.PutUint16(h[2:4], dp)
	binary.BigEndian.PutUint32(h[4:8], seq)
	binary.BigEndian.PutUint32(h[8:12], ack)
	h[12] = 0x50
	h[13] = flags
	binary.BigEndian.PutUint16(h[14:16], 32768)
	copy(h[20:], payload)
	pseudo := uint32(6 + len(h))
	for _, a := range [][4]byte{src, dst} {
		pseudo += uint32(binary.BigEndian.Uint16(a[:2])) + uint32(binary.BigEndian.Uint16(a[2:]))
	}
	binary.BigEndian.PutUint16(h[16:18], checksum(h, pseudo))
	return b
}
func handshake(o *observer, start uint32) {
	o.inspect(tcpPacket(true, 100, 0, 2, nil), true)
	o.inspect(tcpPacket(false, start-1, 101, 18, nil), false)
	o.inspect(tcpPacket(true, 101, start, 16, []byte("HEAD")), true)
}
func TestResponseRequiresCompleteACK(t *testing.T) {
	for _, name := range []string{"handshake_only", "write_only", "partial_ack", "partial_response", "full", "split_retransmit", "wrap", "reset", "bad_checksum", "future_ack", "wrong_tuple"} {
		t.Run(name, func(t *testing.T) {
			o := newObserver(nil)
			start := uint32(500)
			if name == "wrap" {
				start = ^uint32(0) - 20
			}
			handshake(o, start)
			if name != "handshake_only" {
				if name == "split_retransmit" {
					o.inspect(tcpPacket(false, start, 105, 24, []byte(response[:20])), false)
					o.inspect(tcpPacket(false, start, 105, 24, []byte(response[:20])), false)
					o.inspect(tcpPacket(false, start+20, 105, 24, []byte(response[20:])), false)
				} else if name == "partial_response" {
					o.inspect(tcpPacket(false, start, 105, 24, []byte(response[:20])), false)
				} else {
					o.inspect(tcpPacket(false, start, 105, 24, []byte(response)), false)
				}
			}
			if name != "handshake_only" && name != "write_only" {
				ack := start + uint32(len(response))
				if name == "partial_ack" {
					ack--
				}
				if name == "future_ack" {
					ack += 2
				}
				flags := byte(16)
				if name == "reset" {
					flags |= 4
				}
				p := tcpPacket(true, 105, ack, flags, nil)
				if name == "bad_checksum" {
					p[36] ^= 1
				}
				if name == "wrong_tuple" {
					p[20] = 1
					binary.BigEndian.PutUint16(p[36:38], 0)
					pseudo := uint32(26)
					for _, a := range [][4]byte{dutIP, peerIP} {
						pseudo += uint32(binary.BigEndian.Uint16(a[:2])) + uint32(binary.BigEndian.Uint16(a[2:]))
					}
					binary.BigEndian.PutUint16(p[36:38], checksum(p[20:], pseudo))
				}
				o.inspect(p, true)
			}
			ctx, cancel := context.WithTimeout(context.Background(), 10*time.Millisecond)
			defer cancel()
			err := o.waitACK(ctx)
			positive := name == "full" || name == "split_retransmit" || name == "wrap"
			if (err == nil) != positive {
				t.Fatalf("TCP delivery confirmation=%v expected %v", err == nil, positive)
			}
		})
	}
}
func TestACKWaitBroadcastDoesNotCompeteWithFailureObserver(t *testing.T) {
	o := newObserver(nil)
	handshake(o, 500)
	o.inspect(tcpPacket(false, 500, 105, 24, []byte(response)), false)
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	done := make(chan error, 1)
	go func() { done <- o.waitACK(ctx) }()
	o.inspect(tcpPacket(true, 105, 500+uint32(len(response)), 16, nil), true)
	select {
	case <-o.wake:
	default:
	}
	if err := <-done; err != nil {
		t.Fatal(err)
	}
}
