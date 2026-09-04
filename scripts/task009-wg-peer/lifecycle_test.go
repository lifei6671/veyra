package main

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"io"
	"net"
	"strings"
	"testing"
	"time"
)

// peer 单独运行的取消自测，不创建或连接 sing-box DUT。
func TestProtocolFailureHoldsPortUntilShutdown(t *testing.T) {
	for _, scenario := range []struct{ name, initial, invalid string }{
		{"tcp_early_icmp", "init", "probe_icmp"},
		{"udp_icmp", "init_udp", "probe_icmp"},
		{"udp_reinit", "init_udp", "init_udp"},
		{"udp_to_tcp", "init_udp", "init"},
		{"tcp_to_udp", "init", "init_udp"},
	} {
		t.Run(scenario.name, func(t *testing.T) {
			inReader, inWriter := io.Pipe()
			outReader, outWriter := io.Pipe()
			done := make(chan int, 1)
			go func() { done <- run(inReader, outWriter); outWriter.Close() }()
			defer inWriter.Close()
			defer inReader.Close()
			defer outReader.Close()
			initial := strings.Replace(string(initFrame()), `"op":"init"`, `"op":"`+scenario.initial+`"`, 1)
			if _, err := io.WriteString(inWriter, initial); err != nil {
				t.Fatal(err)
			}
			reader := bufio.NewReader(outReader)
			readEvent := func() map[string]json.RawMessage {
				t.Helper()
				line, err := reader.ReadBytes('\n')
				if err != nil {
					t.Fatal(err)
				}
				var e map[string]json.RawMessage
				if err = json.Unmarshal(line, &e); err != nil {
					t.Fatal(err)
				}
				return e
			}
			e := readEvent()
			if string(e["event"]) != `"ready"` {
				t.Fatalf("unexpected event %s", e["event"])
			}
			var port int
			if err := json.Unmarshal(e["udp_port"], &port); err != nil {
				t.Fatal(err)
			}
			invalid := `{"v":1,"op":"probe_icmp","run_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}` + "\n"
			if scenario.invalid != "probe_icmp" {
				invalid = strings.Replace(string(initFrame()), `"op":"init"`, `"op":"`+scenario.invalid+`"`, 1)
			}
			if _, err := io.WriteString(inWriter, invalid); err != nil {
				t.Fatal(err)
			}
			e = readEvent()
			if string(e["event"]) != `"failed"` || string(e["stage"]) != `"protocol"` || string(e["code"]) != `"invalid_input"` {
				t.Fatal("missing failure")
			}
			u, err := net.ListenUDP("udp4", &net.UDPAddr{IP: net.IPv4(127, 0, 0, 1), Port: port})
			if err == nil {
				u.Close()
				t.Fatal("failure released owned peer port")
			}
			select {
			case <-done:
				t.Fatal("failure exited before DUT stop confirmation")
			default:
			}
			shutdown := `{"v":1,"op":"shutdown","run_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","dut_stopped":true}` + "\n"
			if _, err := io.WriteString(inWriter, shutdown); err != nil {
				t.Fatal(err)
			}
			e = readEvent()
			if string(e["event"]) != `"stopped"` || string(e["resources_closed"]) != "true" {
				t.Fatal("missing cleanup")
			}
			select {
			case code := <-done:
				if code == 0 {
					t.Fatal("business failure reported success")
				}
			case <-time.After(3 * time.Second):
				t.Fatal("helper cleanup blocked")
			}
			u, err = net.ListenUDP("udp4", &net.UDPAddr{IP: net.IPv4(127, 0, 0, 1), Port: port})
			if err != nil {
				t.Fatal("peer port not released", err)
			}
			u.Close()
		})
	}
}

type cancelledConnection struct {
	reader  *strings.Reader
	cancel  context.CancelFunc
	written int
}

func (c *cancelledConnection) Read(b []byte) (int, error) {
	n, err := c.reader.Read(b)
	if c.reader.Len() == 0 {
		c.cancel()
	}
	return n, err
}
func (c *cancelledConnection) Write(b []byte) (int, error)    { c.written += len(b); return len(b), nil }
func (*cancelledConnection) Close() error                     { return nil }
func (*cancelledConnection) LocalAddr() net.Addr              { return &net.TCPAddr{} }
func (*cancelledConnection) RemoteAddr() net.Addr             { return &net.TCPAddr{} }
func (*cancelledConnection) SetDeadline(time.Time) error      { return nil }
func (*cancelledConnection) SetReadDeadline(time.Time) error  { return nil }
func (*cancelledConnection) SetWriteDeadline(time.Time) error { return nil }

type singleConnection struct{ net.Conn }

func (l singleConnection) Accept() (net.Conn, error) { return l.Conn, nil }
func (l singleConnection) Close() error              { return l.Conn.Close() }
func (l singleConnection) Addr() net.Addr            { return l.Conn.LocalAddr() }
func TestCancelledRequestDoesNotStartHTTPResponse(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	token := strings.Repeat("a", 32)
	c := &cancelledConnection{reader: strings.NewReader("HEAD /task009-wg?token=" + token + " HTTP/1.1\r\nHost: 198.18.0.2:18080\r\n\r\n"), cancel: cancel}
	p := &livePeer{listener: singleConnection{c}}
	if err := serveHTTP(ctx, p, token); !errors.Is(err, context.Canceled) {
		t.Fatal("cancellation not propagated", err)
	}
	if c.written != 0 {
		t.Fatal("cancelled request still sent response")
	}
}
func TestOutputInheritsHardDeadline(t *testing.T) {
	r, w := io.Pipe()
	done := make(chan struct{})
	defer close(done)
	defer r.Close()
	defer w.Close()
	output := boundedOutput(w, done, time.Now().Add(30*time.Millisecond))
	start := time.Now()
	if err := output(event{"v": 1}); err == nil {
		t.Fatal("blocked output succeeded")
	}
	if time.Since(start) > 500*time.Millisecond {
		t.Fatal("ignored hard deadline")
	}
}
