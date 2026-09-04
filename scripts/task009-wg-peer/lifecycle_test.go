package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"io"
	"net"
	"os"
	"os/exec"
	"strings"
	"testing"
	"time"
)

func TestDNSProbeInputFailureChild(t *testing.T) {
	if os.Getenv("VEYRA_TEST_DNS_INPUT_CHILD") != "1" {
		t.Skip("isolated helper subprocess only")
	}
	os.Exit(run(os.Stdin, os.Stdout))
}

func TestDNSProbeInputFailureHoldsUntilOwnedProcessTermination(t *testing.T) {
	for _, scenario := range []string{"eof", "unconfirmed_shutdown"} {
		t.Run(scenario, func(t *testing.T) {
			executable, err := os.Executable()
			if err != nil {
				t.Fatal(err)
			}
			ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
			defer cancel()
			cmd := exec.CommandContext(ctx, executable, "-test.run=^TestDNSProbeInputFailureChild$")
			cmd.Env = append(os.Environ(), "VEYRA_TEST_DNS_INPUT_CHILD=1")
			stdin, err := cmd.StdinPipe()
			if err != nil {
				t.Fatal(err)
			}
			stdout, err := cmd.StdoutPipe()
			if err != nil {
				t.Fatal(err)
			}
			var stderr bytes.Buffer
			cmd.Stderr = &stderr
			if err := cmd.Start(); err != nil {
				t.Fatal(err)
			}
			defer func() { cmd.Process.Kill(); cmd.Wait() }()
			initial := strings.Replace(string(initFrame()), `"op":"init"`, `"op":"init_dns_probe"`, 1)
			if _, err := io.WriteString(stdin, initial); err != nil {
				t.Fatal(err)
			}
			decoder := json.NewDecoder(stdout)
			var ready map[string]json.RawMessage
			if err := decoder.Decode(&ready); err != nil || string(ready["event"]) != `"ready"` {
				t.Fatal("DNS probe child did not become ready", err)
			}
			if scenario == "unconfirmed_shutdown" {
				if _, err := io.WriteString(stdin, `{"v":1,"op":"shutdown","run_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","dut_stopped":false}`+"\n"); err != nil {
					t.Fatal(err)
				}
			}
			stdin.Close()
			var failure map[string]json.RawMessage
			if err := decoder.Decode(&failure); err != nil || string(failure["event"]) != `"failed"` || string(failure["code"]) != `"invalid_input"` {
				t.Fatal("input failure not propagated", err)
			}
			var port int
			if err := json.Unmarshal(ready["udp_port"], &port); err != nil {
				t.Fatal(err)
			}
			u, err := net.ListenUDP("udp4", &net.UDPAddr{IP: net.IPv4(127, 0, 0, 1), Port: port})
			if err == nil {
				u.Close()
				t.Fatal("input failure released peer ownership")
			}
			// 父输入已终止，不能再发有效 shutdown；仅终止此测试拥有的子进程。
			if err := cmd.Process.Kill(); err != nil {
				t.Fatal("helper exited instead of holding", err)
			}
			if err := decoder.Decode(new(any)); err != io.EOF {
				t.Fatal("failed helper emitted stopped or extra output", err)
			}
			if err := cmd.Wait(); err == nil {
				t.Fatal("abnormal helper termination reported exit zero")
			}
			if stderr.Len() != 0 {
				t.Fatal("helper emitted stderr")
			}
		})
	}
}

// peer 单独运行的取消自测，不创建或连接 sing-box DUT。
func TestProtocolFailureHoldsPortUntilShutdown(t *testing.T) {
	for _, scenario := range []struct{ name, initial, invalid string }{
		{"tcp_early_icmp", "init", "probe_icmp"},
		{"udp_icmp", "init_udp", "probe_icmp"},
		{"udp_reinit", "init_udp", "init_udp"},
		{"udp_to_tcp", "init_udp", "init"},
		{"tcp_to_udp", "init", "init_udp"},
		{"dns_icmp", "init_dns_probe", "probe_icmp"},
		{"dns_reinit", "init_dns_probe", "init_dns_probe"},
		{"dns_to_tcp", "init_dns_probe", "init"},
		{"dns_to_udp", "init_dns_probe", "init_udp"},
		{"tcp_to_dns", "init", "init_dns_probe"},
		{"udp_to_dns", "init_udp", "init_dns_probe"},
		{"domain_http_icmp", "init_domain_http", "probe_icmp"},
		{"domain_tls_icmp", "init_domain_tls", "probe_icmp"},
		{"domain_http_reinit", "init_domain_http", "init_domain_http"},
		{"domain_tls_reinit", "init_domain_tls", "init_domain_tls"},
		{"domain_http_to_tls", "init_domain_http", "init_domain_tls"},
		{"domain_tls_to_dns", "init_domain_tls", "init_dns_probe"},
		{"dns_to_domain", "init_dns_probe", "init_domain_http"},
		{"tcp_to_domain", "init", "init_domain_tls"},
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
			if scenario.initial == "init_dns_probe" {
				if len(e) != 6 || string(e["discarded_packets"]) != "0" || string(e["discarded_bytes"]) != "0" {
					t.Fatal("DNS probe cleanup counters")
				}
			} else if strings.HasPrefix(scenario.initial, "init_domain_") {
				if len(e) != 5 || string(e["mode"]) != `"`+strings.TrimPrefix(scenario.initial, "init_")+`"` {
					t.Fatal("domain cleanup shape")
				}
			} else if len(e) != 4 {
				t.Fatal("old mode acquired DNS probe fields")
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

func TestDNSProbeReadyShutdownWithoutBusiness(t *testing.T) {
	inReader, inWriter := io.Pipe()
	outReader, outWriter := io.Pipe()
	defer inReader.Close()
	defer inWriter.Close()
	defer outReader.Close()
	done := make(chan int, 1)
	go func() { done <- run(inReader, outWriter); outWriter.Close() }()
	initial := strings.Replace(string(initFrame()), `"op":"init"`, `"op":"init_dns_probe"`, 1)
	if _, err := io.WriteString(inWriter, initial); err != nil {
		t.Fatal(err)
	}
	decoder := json.NewDecoder(outReader)
	var ready map[string]json.RawMessage
	if err := decoder.Decode(&ready); err != nil {
		t.Fatal(err)
	}
	if len(ready) != 7 || string(ready["event"]) != `"ready"` || string(ready["run_id"]) != `"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"` {
		t.Fatal("DNS probe ready shape")
	}
	shutdown := `{"v":1,"op":"shutdown","run_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","dut_stopped":true}` + "\n"
	if _, err := io.WriteString(inWriter, shutdown); err != nil {
		t.Fatal(err)
	}
	var stopped map[string]json.RawMessage
	if err := decoder.Decode(&stopped); err != nil {
		t.Fatal(err)
	}
	if len(stopped) != 6 || string(stopped["event"]) != `"stopped"` || string(stopped["resources_closed"]) != "true" || string(stopped["discarded_packets"]) != "0" || string(stopped["discarded_bytes"]) != "0" {
		t.Fatal("DNS probe did not finish with exact zero counters")
	}
	if err := decoder.Decode(new(any)); err != io.EOF {
		t.Fatal("trailing DNS probe event or missing EOF", err)
	}
	select {
	case code := <-done:
		if code != 0 {
			t.Fatal("DNS probe without business did not exit zero", code)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("DNS probe cleanup blocked")
	}
}

func TestDNSProbeLiveHasNoBusinessServicesAndReportsFault(t *testing.T) {
	c, err := decodeCommand([]byte(strings.Replace(string(initFrame()), `"op":"init"`, `"op":"init_dns_probe"`, 1)))
	if err != nil {
		t.Fatal(err)
	}
	p, _, _, err := newLive(c)
	if err != nil {
		t.Fatal(err)
	}
	defer p.close()
	if p.listener != nil || p.udp != nil || p.tun.dnsProbe == nil {
		p.close()
		t.Fatal("DNS probe created business service")
	}
	if _, err := p.tun.Write([][]byte{make([]byte, 1281)}, 0); err == nil {
		p.close()
		t.Fatal("oversize DNS probe packet accepted")
	}
	select {
	case <-p.watch.wake:
		if _, _, err := p.watch.result(); err == nil {
			t.Fatal("sink failure did not reach main observer")
		}
	default:
		t.Fatal("sink failure did not wake main loop")
	}
	if err := p.close(); err != nil {
		t.Fatal(err)
	}
	packets, size := p.tun.dnsProbe.result()
	if packets != 0 || size != 0 {
		t.Fatal("rejected packet changed final counters")
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
