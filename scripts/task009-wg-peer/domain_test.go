package main

import (
	"bytes"
	"context"
	"crypto/tls"
	"encoding/binary"
	"encoding/json"
	"errors"
	"io"
	"net"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/sagernet/gvisor/pkg/tcpip/adapters/gonet"
	"github.com/sagernet/gvisor/pkg/tcpip/network/ipv4"
)

func TestDomainProtocolClosedFields(t *testing.T) {
	for _, op := range []string{"init_domain_http", "init_domain_tls"} {
		good := bytes.Replace(initFrame(), []byte(`"op":"init"`), []byte(`"op":"`+op+`"`), 1)
		if c, err := decodeCommand(good); err != nil || c.Op != op {
			t.Fatal("domain init rejected")
		}
		for _, field := range []string{`"host":"veyra.disign.me",`, `"phase":null,`, `"virtual_tcp_port":null,`, `"dut_stopped":null,`, `"op":"init",`, `"tls":true,`} {
			bad := append([]byte("{"+field), good[1:]...)
			if _, err := decodeCommand(bad); err == nil {
				t.Fatal("domain extra or duplicate field accepted")
			}
		}
	}
}

func domainPacket(incoming bool, seq, ack uint32, flags byte, payload []byte, tlsMode bool) []byte {
	b := tcpPacket(incoming, seq, ack, flags, payload)
	if incoming {
		copy(b[16:20], domainIP[:])
	} else {
		copy(b[12:16], domainIP[:])
	}
	if tlsMode {
		if incoming {
			binary.BigEndian.PutUint16(b[22:24], 18443)
		} else {
			binary.BigEndian.PutUint16(b[20:22], 18443)
		}
	}
	domainChecksums(b)
	return b
}

func domainChecksums(b []byte) {
	binary.BigEndian.PutUint16(b[10:12], 0)
	binary.BigEndian.PutUint16(b[10:12], checksum(b[:20], 0))
	binary.BigEndian.PutUint16(b[36:38], 0)
	pseudo := uint32(6 + len(b) - 20)
	for i := 12; i < 20; i += 2 {
		pseudo += uint32(binary.BigEndian.Uint16(b[i : i+2]))
	}
	binary.BigEndian.PutUint16(b[36:38], checksum(b[20:], pseudo))
}

func TestDomainObserverHTTPACKAndLateFailures(t *testing.T) {
	for _, name := range []string{"full", "no_ack", "partial_ack", "second_syn", "wrong_address", "wrong_port", "extra_request", "reset_after_ack", "bad_checksum", "fragment", "oversize", "wrong_protocol", "packet_limit"} {
		t.Run(name, func(t *testing.T) {
			o := newObserver(nil)
			o.domain = &domainObserver{}
			o.inspect(domainPacket(true, 100, 0, 2, nil, false), true)
			o.inspect(domainPacket(false, 499, 101, 18, nil, false), false)
			o.inspect(domainPacket(true, 101, 500, 24, []byte("HEAD"), false), true)
			o.domain.headerBytes = 4
			o.inspect(domainPacket(false, 500, 105, 24, []byte(response), false), false)
			ack := uint32(500 + len(response))
			if name == "partial_ack" {
				ack--
			}
			if name != "no_ack" {
				o.inspect(domainPacket(true, 105, ack, 16, nil, false), true)
			}
			switch name {
			case "second_syn":
				o.inspect(domainPacket(true, 200, 0, 2, nil, false), true)
			case "extra_request":
				o.inspect(domainPacket(true, 105, ack, 24, []byte("HEAD"), false), true)
			case "reset_after_ack":
				o.inspect(domainPacket(true, 105, ack, 20, nil, false), true)
			case "packet_limit":
				for range 1025 {
					o.inspect(domainPacket(true, 105, ack, 16, nil, false), true)
				}
			case "wrong_address", "wrong_port", "bad_checksum", "fragment", "oversize", "wrong_protocol":
				b := domainPacket(true, 105, ack, 16, nil, false)
				switch name {
				case "wrong_address":
					b[19]--
					domainChecksums(b)
				case "wrong_port":
					b[23]--
					domainChecksums(b)
				case "bad_checksum":
					b[37] ^= 1
				case "fragment":
					b[6] = 0x20
					domainChecksums(b)
				case "oversize":
					b = domainPacket(true, 105, ack, 24, make([]byte, 1241), false)
				case "wrong_protocol":
					b[9] = 1
					domainChecksums(b)
				}
				o.inspect(b, true)
			}
			ctx, cancel := context.WithTimeout(context.Background(), 5*time.Millisecond)
			defer cancel()
			err := o.waitACK(ctx)
			want := name == "full" || name == "reset_after_ack"
			if (err == nil) != want {
				t.Fatalf("ACK outcome mismatch for %s", name)
			}
		})
	}
}

func TestDomainMemoryHTTPAndTLS(t *testing.T) {
	for _, tlsMode := range []bool{false, true} {
		t.Run(map[bool]string{false: "http", true: "tls"}[tlsMode], func(t *testing.T) {
			ctx, cancel := context.WithTimeout(context.Background(), 4*time.Second)
			defer cancel()
			watch := newObserver(nil)
			watch.domain = &domainObserver{tls: tlsMode}
			a, err := newMemoryTun(peerIP, watch)
			if err != nil {
				t.Fatal(err)
			}
			b, err := newMemoryTun(dutIP, nil)
			if err != nil {
				a.Close()
				t.Fatal(err)
			}
			var bridges sync.WaitGroup
			bridge := func(from, to *memoryTun) {
				defer bridges.Done()
				for {
					p := from.link.ReadContext(ctx)
					if p == nil {
						return
					}
					var raw []byte
					for _, part := range p.AsSlices() {
						raw = append(raw, part...)
					}
					p.DecRef()
					if from.watch != nil {
						from.watch.inspect(raw, false)
					}
					if _, err := to.Write([][]byte{raw}, 0); err != nil {
						return
					}
				}
			}
			bridges.Add(2)
			go bridge(a, b)
			go bridge(b, a)
			listener, err := gonet.ListenTCP(a.s, address(domainIP, watch.domain.port()), ipv4.ProtocolNumber)
			if err != nil {
				cancel()
				a.Close()
				b.Close()
				bridges.Wait()
				t.Fatal(err)
			}
			p := &livePeer{tun: a, listener: listener, watch: watch}
			result := make(chan domainResult, 1)
			p.workers.Add(1)
			go func() { defer p.workers.Done(); result <- serveDomain(ctx, p) }()
			defer func() {
				cancel()
				listener.Close()
				a.Close()
				b.Close()
				p.workers.Wait()
				bridges.Wait()
			}()
			client, err := gonet.DialContextTCP(ctx, b.s, address(domainIP, watch.domain.port()), ipv4.ProtocolNumber)
			if err != nil {
				t.Fatal(err)
			}
			defer client.Close()
			if tlsMode {
				tc := tls.Client(client, &tls.Config{ServerName: domainHost})
				if err := tc.HandshakeContext(ctx); err == nil {
					t.Fatal("TLS unexpectedly succeeded")
				}
			} else {
				if err := client.SetDeadline(deadline(ctx)); err != nil {
					t.Fatal(err)
				}
				if _, err := io.WriteString(client, "HEAD /task009-wg-domain HTTP/1.1\r\nHost: veyra.disign.me:18080\r\n\r\n"); err != nil {
					t.Fatal(err)
				}
				got := make([]byte, len(response))
				if _, err := io.ReadFull(client, got); err != nil || string(got) != response {
					t.Fatal("response mismatch", err)
				}
			}
			select {
			case got := <-result:
				if got.err != nil {
					t.Fatal("memory domain service failed", got.err)
				}
				if tlsMode && (got.bytes <= 0 || got.bytes > 16384) {
					t.Fatal("ClientHello bounds")
				}
			case <-ctx.Done():
				t.Fatal("domain service timed out")
			}
			if rx, tx, err := watch.result(); err != nil || rx == 0 || tx == 0 {
				t.Fatal("missing packet boundary evidence", err)
			}
		})
	}
}

type domainScriptConn struct {
	input      *bytes.Reader
	writeErr   error
	short      bool
	readErr    error
	writeDelay time.Duration
}

func (c *domainScriptConn) Read(b []byte) (int, error) {
	if c.readErr != nil {
		return 0, c.readErr
	}
	return c.input.Read(b)
}
func (c *domainScriptConn) Write(b []byte) (int, error) {
	if c.writeDelay != 0 {
		time.Sleep(c.writeDelay)
	}
	if c.writeErr != nil {
		return 0, c.writeErr
	}
	if c.short {
		return len(b) - 1, nil
	}
	return len(b), nil
}
func (*domainScriptConn) Close() error                     { return nil }
func (*domainScriptConn) LocalAddr() net.Addr              { return &net.TCPAddr{} }
func (*domainScriptConn) RemoteAddr() net.Addr             { return &net.TCPAddr{} }
func (*domainScriptConn) SetDeadline(time.Time) error      { return nil }
func (*domainScriptConn) SetReadDeadline(time.Time) error  { return nil }
func (*domainScriptConn) SetWriteDeadline(time.Time) error { return nil }

func domainClientHello(t *testing.T, host string) []byte {
	t.Helper()
	a, b := net.Pipe()
	defer b.Close()
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	done := make(chan struct{})
	go func() {
		defer close(done)
		defer a.Close()
		_ = tls.Client(a, &tls.Config{ServerName: host}).HandshakeContext(ctx)
	}()
	header := make([]byte, 5)
	if _, err := io.ReadFull(b, header); err != nil {
		t.Fatal(err)
	}
	payload := make([]byte, int(binary.BigEndian.Uint16(header[3:])))
	if _, err := io.ReadFull(b, payload); err != nil {
		t.Fatal(err)
	}
	b.Close()
	<-done
	return append(header, payload...)
}

func TestDomainTLSRejectsUnderlyingFailureDespiteSentinel(t *testing.T) {
	hello := domainClientHello(t, domainHost)
	for _, name := range []string{"match", "wrong_sni", "no_sni", "eof", "malformed", "short_alert", "alert_error", "alert_deadline", "read_error", "input_limit"} {
		t.Run(name, func(t *testing.T) {
			input := hello
			c := &domainScriptConn{}
			ctx, cancel := context.WithTimeout(context.Background(), time.Second)
			defer cancel()
			switch name {
			case "wrong_sni":
				input = domainClientHello(t, "wrong.example")
			case "no_sni":
				input = domainClientHello(t, "127.0.0.1")
			case "eof":
				input = nil
			case "malformed":
				input = []byte("not TLS")
			case "short_alert":
				c.short = true
			case "alert_error":
				c.writeErr = errors.New("write failure")
			case "alert_deadline":
				ctx, cancel = context.WithTimeout(context.Background(), 10*time.Millisecond)
				defer cancel()
				c.writeDelay = 20 * time.Millisecond
			case "read_error":
				c.readErr = io.ErrUnexpectedEOF
			case "input_limit":
				input = append([]byte{22, 3, 3, 0x40, 0}, make([]byte, 16384)...)
			}
			c.input = bytes.NewReader(input)
			n, err := observeDomainTLS(ctx, c)
			if (err == nil) != (name == "match") {
				t.Fatalf("TLS failure incorrectly classified: %s", name)
			}
			if n > 16384 {
				t.Fatal("TLS read cap exceeded")
			}
		})
	}
}

func TestDomainHTTPRejectsWrongOrExtraRequest(t *testing.T) {
	good := "HEAD /task009-wg-domain HTTP/1.1\r\nHost: veyra.disign.me:18080\r\n\r\n"
	for name, request := range map[string]string{
		"valid":             good,
		"host":              strings.Replace(good, domainHost, "wrong.example", 1),
		"port":              strings.Replace(good, "18080", "18443", 1),
		"path":              strings.Replace(good, "/task009-wg-domain", "/task009-dns-preflight", 1),
		"method":            strings.Replace(good, "HEAD", "GET", 1),
		"body":              strings.Replace(good, "\r\n\r\n", "\r\nContent-Length: 1\r\n\r\nx", 1),
		"transfer_encoding": strings.Replace(good, "\r\n\r\n", "\r\nTransfer-Encoding: chunked\r\n\r\n", 1),
		"duplicate_host":    strings.Replace(good, "\r\n\r\n", "\r\nHost: veyra.disign.me:18080\r\n\r\n", 1),
		"second_request":    good + good,
		"limit":             "HEAD /task009-wg-domain HTTP/1.1\r\nX: " + strings.Repeat("a", 16384),
	} {
		t.Run(name, func(t *testing.T) {
			c := &domainScriptConn{input: bytes.NewReader([]byte(request))}
			o := newObserver(nil)
			o.domain = &domainObserver{}
			o.acked = true
			ctx, cancel := context.WithTimeout(context.Background(), time.Second)
			defer cancel()
			err := serveDomainHTTP(ctx, c, o)
			if (err == nil) != (name == "valid") {
				t.Fatal("HTTP acceptance mismatch")
			}
		})
	}
}

func TestDomainTLSConnectionStickyLimits(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	for _, write := range []bool{false, true} {
		base := &domainScriptConn{input: bytes.NewReader(make([]byte, 16385))}
		c := &domainTLSConn{Conn: base, ctx: ctx}
		if write {
			if n, err := c.Write(make([]byte, 4096)); err != nil || n != 4096 {
				t.Fatal("output maximum rejected")
			}
			if n, err := c.Write([]byte{1}); err == nil || n != 0 {
				t.Fatal("output overflow accepted")
			}
		} else {
			if n, err := c.Read(make([]byte, 16385)); err != nil || n != 16384 {
				t.Fatal("input maximum not bounded")
			}
			if n, err := c.Read([]byte{0}); err == nil || n != 0 {
				t.Fatal("input overflow accepted")
			}
		}
		first := c.failure
		if _, err := c.Write([]byte{1}); err != first {
			t.Fatal("failure not sticky")
		}
		if _, err := c.Read([]byte{0}); err != first {
			t.Fatal("read cleared failure")
		}
	}
}

func TestDomainReadyCancellationIsCleanupNotSuccess(t *testing.T) {
	for _, op := range []string{"init_domain_http", "init_domain_tls"} {
		t.Run(op, func(t *testing.T) {
			ir, iw := io.Pipe()
			or, ow := io.Pipe()
			defer ir.Close()
			defer iw.Close()
			defer or.Close()
			done := make(chan int, 1)
			go func() { done <- run(ir, ow); ow.Close() }()
			initial := strings.Replace(string(initFrame()), `"op":"init"`, `"op":"`+op+`"`, 1)
			if _, err := io.WriteString(iw, initial); err != nil {
				t.Fatal(err)
			}
			decoder := json.NewDecoder(or)
			var frame map[string]json.RawMessage
			if err := decoder.Decode(&frame); err != nil || string(frame["event"]) != `"ready"` {
				t.Fatal("missing ready", err)
			}
			if _, err := io.WriteString(iw, `{"v":1,"op":"shutdown","run_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","dut_stopped":true}`+"\n"); err != nil {
				t.Fatal(err)
			}
			frame = nil
			if err := decoder.Decode(&frame); err != nil {
				t.Fatal(err)
			}
			if len(frame) != 5 || string(frame["event"]) != `"stopped"` || string(frame["resources_closed"]) != "true" || string(frame["mode"]) != `"`+strings.TrimPrefix(op, "init_")+`"` {
				t.Fatal("invalid stopped")
			}
			if _, ok := frame["connections"]; ok {
				t.Fatal("cleanup invented connection count")
			}
			if err := decoder.Decode(new(any)); err != io.EOF {
				t.Fatal("extra business frame", err)
			}
			select {
			case code := <-done:
				if code == 0 {
					t.Fatal("cancel reported business success")
				}
			case <-time.After(3 * time.Second):
				t.Fatal("cancel cleanup blocked")
			}
		})
	}
}
