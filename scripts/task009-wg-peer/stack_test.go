package main

import (
	"context"
	"errors"
	"os"
	"sync"
	"testing"
	"time"
)

func TestDNSProbeDiscardBoundsAndNoStackDelivery(t *testing.T) {
	watch := newObserver([]byte("0123456789abcdef"))
	m, err := newMemoryTun(peerIP, watch)
	if err != nil {
		t.Fatal(err)
	}
	defer m.Close()
	m.dnsProbe = &dnsProbeSink{fail: watch.fail}
	// 包内容不作为目标配置或路由输入；覆盖 IPv4、IPv6 和最小长度。
	for _, raw := range [][]byte{{0xff}, append([]byte{0x45}, make([]byte, 39)...), append([]byte{0x60}, make([]byte, 39)...)} {
		framed := append([]byte{0, 0, 0}, raw...)
		if n, err := m.Write([][]byte{framed}, 3); err != nil || n != 1 {
			t.Fatal("bounded packet was not discarded", n, err)
		}
	}
	packets, size := m.dnsProbe.result()
	if packets != 3 || size != 81 {
		t.Fatalf("discard counters: %d/%d", packets, size)
	}
	if rx, tx, err := watch.result(); err != nil || rx != 0 || tx != 0 {
		t.Fatal("DNS probe entered business observer", rx, tx, err)
	}
	if got := m.s.Stats().IP.PacketsReceived.Value(); got != 0 {
		t.Fatal("DNS probe injected packets into IP stack", got)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Millisecond)
	defer cancel()
	if packet := m.link.ReadContext(ctx); packet != nil {
		packet.DecRef()
		t.Fatal("DNS probe generated outbound business packet")
	}
}

func TestDNSProbeDiscardLimitsAndStickyFailure(t *testing.T) {
	watch := newObserver(nil)
	d := &dnsProbeSink{fail: watch.fail}
	for range 64 {
		if n, err := d.discard([][]byte{make([]byte, 1280)}, 0); err != nil || n != 1 {
			t.Fatal("packet within maximum rejected", n, err)
		}
	}
	if packets, size := d.result(); packets != 64 || size != 81920 {
		t.Fatalf("maximum counters: %d/%d", packets, size)
	}
	for range 2 {
		if n, err := d.discard([][]byte{{1}}, 0); err == nil || n != 0 {
			t.Fatal("overflow or post-failure packet accepted", n, err)
		}
	}
	if packets, size := d.result(); packets != 64 || size != 81920 {
		t.Fatal("overflow changed counters")
	}
	if _, _, err := watch.result(); err == nil {
		t.Fatal("overflow failure not propagated")
	}
}

func TestDNSProbeDiscardRejectsOffsetLengthAndPartialBatch(t *testing.T) {
	for _, tc := range []struct {
		name     string
		bufs     [][]byte
		offset   int
		accepted int
	}{
		{"negative_offset", [][]byte{{1}}, -1, 0},
		{"empty_packet", [][]byte{{}}, 0, 0},
		{"offset_at_end", [][]byte{{1}}, 1, 0},
		{"offset_past_end", [][]byte{{1}}, 2, 0},
		{"oversize", [][]byte{make([]byte, 1281)}, 0, 0},
		{"partial_batch", [][]byte{{1}, {}}, 0, 1},
	} {
		t.Run(tc.name, func(t *testing.T) {
			watch := newObserver(nil)
			d := &dnsProbeSink{fail: watch.fail}
			if n, err := d.discard(tc.bufs, tc.offset); err == nil || n != tc.accepted {
				t.Fatal("invalid bounds not rejected", n, err)
			}
			if packets, size := d.result(); packets != uint32(tc.accepted) || size != uint32(tc.accepted) {
				t.Fatal("invalid packet included in counters")
			}
			if _, _, err := watch.result(); err == nil {
				t.Fatal("invalid bounds not reported to main loop")
			}
		})
	}
}

func TestDNSProbeConcurrentDiscardAndCloseHaveStableCounters(t *testing.T) {
	watch := newObserver(nil)
	m, err := newMemoryTun(peerIP, watch)
	if err != nil {
		t.Fatal(err)
	}
	defer m.Close()
	m.dnsProbe = &dnsProbeSink{fail: watch.fail}
	var writers sync.WaitGroup
	for range 64 {
		writers.Add(1)
		go func() {
			defer writers.Done()
			if _, err := m.Write([][]byte{{1}}, 0); err != nil {
				t.Error(err)
			}
		}()
	}
	writers.Wait()
	if packets, size := m.dnsProbe.result(); packets != 64 || size != 64 {
		t.Fatalf("concurrent counters: %d/%d", packets, size)
	}
	readDone := make(chan error, 1)
	go func() { _, err := m.Read([][]byte{make([]byte, 1280)}, make([]int, 1), 0); readDone <- err }()
	if err := m.Close(); err != nil {
		t.Fatal(err)
	}
	select {
	case err := <-readDone:
		if !errors.Is(err, os.ErrClosed) {
			t.Fatal("close did not cancel outbound read", err)
		}
	case <-time.After(time.Second):
		t.Fatal("outbound read did not stop")
	}
	for range 8 {
		writers.Add(1)
		go func() {
			defer writers.Done()
			if _, err := m.Write([][]byte{{1}}, 0); !errors.Is(err, os.ErrClosed) {
				t.Error("late write accepted", err)
			}
		}()
	}
	writers.Wait()
	if packets, size := m.dnsProbe.result(); packets != 64 || size != 64 {
		t.Fatal("late writes changed final counters")
	}
	if _, _, err := watch.result(); err != nil {
		t.Fatal("normal close became packet failure", err)
	}
}

func TestDNSProbeCloseSerializesWithInFlightWrites(t *testing.T) {
	watch := newObserver(nil)
	m, err := newMemoryTun(peerIP, watch)
	if err != nil {
		t.Fatal(err)
	}
	defer m.Close()
	m.dnsProbe = &dnsProbeSink{fail: watch.fail}
	start := make(chan struct{})
	results := make(chan bool, 64)
	for range 64 {
		go func() {
			<-start
			_, err := m.Write([][]byte{{1}}, 0)
			if err != nil && !errors.Is(err, os.ErrClosed) {
				t.Error("concurrent close produced unexpected error", err)
			}
			results <- err == nil
		}()
	}
	close(start)
	if err := m.Close(); err != nil {
		t.Fatal(err)
	}
	var accepted uint32
	for range 64 {
		if <-results {
			accepted++
		}
	}
	if packets, size := m.dnsProbe.result(); packets != accepted || size != accepted {
		t.Fatalf("close raced counters: accepted=%d packets=%d bytes=%d", accepted, packets, size)
	}
	if _, err := m.dnsProbe.discard([][]byte{{1}}, 0); !errors.Is(err, os.ErrClosed) {
		t.Fatal("closed sink accepted late packet", err)
	}
	if packets, size := m.dnsProbe.result(); packets != accepted || size != accepted {
		t.Fatal("closed counters changed")
	}
}

func TestMemoryTCPUDPICMP(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := selftest(ctx); err != nil {
		t.Fatal(err)
	}
}

// 三个阶段复用同一内存栈和固定 ICMP id，验证每轮关闭后能重新绑定。
func TestMemoryICMPThreePhasesReleaseFixedIdentifier(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	watch := rejectWatch(t)
	a, err := newMemoryTun(peerIP, watch)
	if err != nil {
		t.Fatal(err)
	}
	b, err := newMemoryTun(dutIP, nil)
	if err != nil {
		a.Close()
		t.Fatal(err)
	}
	var workers sync.WaitGroup
	bridge := func(from, to *memoryTun) {
		defer workers.Done()
		for {
			packet := from.link.ReadContext(ctx)
			if packet == nil {
				return
			}
			var raw []byte
			for _, segment := range packet.AsSlices() {
				raw = append(raw, segment...)
			}
			packet.DecRef()
			if from.watch != nil {
				from.watch.inspect(raw, false)
			}
			if _, err := to.Write([][]byte{raw}, 0); err != nil {
				return
			}
		}
	}
	workers.Add(2)
	go bridge(a, b)
	go bridge(b, a)
	defer func() { cancel(); a.Close(); b.Close(); workers.Wait() }()
	for phase := 1; phase <= 3; phase++ {
		if phase > 1 {
			watch.mu.Lock()
			watch.reject.active = false
			watch.mu.Unlock()
			if err := watch.beginRejectPhase(phase); err != nil {
				t.Fatal(err)
			}
		}
		watch.mu.Lock()
		health := watch.reject.health
		watch.mu.Unlock()
		if err := icmpProbe(ctx, a, health.token, health); err != nil {
			t.Fatalf("phase %d: %v", phase, err)
		}
		health.mu.Lock()
		sent, received := health.icmpSent, health.icmpReceived
		health.mu.Unlock()
		if sent != 3 || received != 3 {
			t.Fatalf("phase %d: sent=%d received=%d", phase, sent, received)
		}
	}
}
