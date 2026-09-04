package main

import (
	"errors"
	"net"
	"testing"
	"time"

	"github.com/sagernet/wireguard-go/conn"
)

func TestLoopbackBindCloseWakesReceiver(t *testing.T) {
	b := &loopBind{fault: func(error) { t.Error("unexpected bind fault") }}
	fns, port, err := b.Open(0)
	if err != nil {
		t.Fatal(err)
	}
	if port == 0 || port == 9090 || b.udp.LocalAddr().(*net.UDPAddr).IP.String() != "127.0.0.1" {
		t.Fatal("unexpected bind")
	}
	done := make(chan error, 1)
	go func() {
		_, err := fns[0]([][]byte{make([]byte, 2048)}, make([]int, 1), make([]conn.Endpoint, 1))
		done <- err
	}()
	if err := b.Close(); err != nil {
		t.Fatal(err)
	}
	select {
	case err := <-done:
		if !errors.Is(err, net.ErrClosed) {
			t.Fatal("close not propagated", err)
		}
	case <-time.After(time.Second):
		t.Fatal("receive blocked after close")
	}
	if err := b.Close(); err != nil {
		t.Fatal(err)
	}
	if _, err := b.ParseEndpoint("192.0.2.1:1234"); err == nil {
		t.Fatal("nonloopback accepted")
	}
	if err := b.SetMark(1); err == nil {
		t.Fatal("mark accepted")
	}
}
