package main

import (
	"bytes"
	"encoding/json"
	"io"
	"strings"
	"testing"
	"time"
)

func initFrame() []byte {
	dut, peer := make([]uint16, 32), make([]uint16, 32)
	for i := range dut {
		dut[i] = uint16(i + 1)
		peer[i] = uint16(i + 40)
	}
	b, _ := json.Marshal(command{V: 1, Op: "init", RunID: strings.Repeat("a", 32), DUT: dut, Peer: peer, Token: strings.Repeat("b", 32)})
	// 测试输入显式省略未使用字段，和父端协议一致。
	var fields map[string]any
	_ = json.Unmarshal(b, &fields)
	delete(fields, "dut_stopped")
	b, _ = json.Marshal(fields)
	return append(b, '\n')
}
func TestProtocolRejectsInvalidFrames(t *testing.T) {
	good := initFrame()
	if _, err := decodeCommand(good); err != nil {
		t.Fatal(err)
	}
	for _, bad := range [][]byte{append(bytes.TrimSpace(good), []byte(" {}")...), bytes.Replace(good, []byte(`"v":1`), []byte(`"v":2`), 1), bytes.Replace(good, []byte(`"op":"init"`), []byte(`"op":"unknown"`), 1), []byte(`{"v":1,"op":"shutdown","run_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","dut_stopped":false}`)} {
		if _, err := decodeCommand(bad); err == nil {
			t.Fatal("invalid frame accepted")
		}
	}
	var fields map[string]any
	json.Unmarshal(good, &fields)
	fields["extra"] = true
	unknown, _ := json.Marshal(fields)
	if _, err := decodeCommand(unknown); err == nil {
		t.Fatal("unknown field")
	}
}
func TestUDPInitRejectsDuplicateAndUnknownFields(t *testing.T) {
	good := bytes.Replace(initFrame(), []byte(`"op":"init"`), []byte(`"op":"init_udp"`), 1)
	c, err := decodeCommand(good)
	if err != nil || c.Op != "init_udp" {
		t.Fatal("UDP init rejected", err)
	}
	for _, field := range []string{`"op":"init",`, `"token":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",`, `"extra":true,`} {
		bad := append([]byte("{"+field), good[1:]...)
		if _, err := decodeCommand(bad); err == nil {
			t.Fatal("ambiguous or unknown UDP init accepted")
		}
	}
}

func TestDNSProbeInitHasExactFields(t *testing.T) {
	good := bytes.Replace(initFrame(), []byte(`"op":"init"`), []byte(`"op":"init_dns_probe"`), 1)
	c, err := decodeCommand(good)
	if err != nil || c.Op != "init_dns_probe" {
		t.Fatal("DNS probe init rejected", err)
	}
	for _, field := range []string{
		`"op":"init",`, `"token":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",`,
		`"phase":null,`, `"virtual_tcp_port":null,`, `"dut_stopped":null,`,
		`"discarded_packets":0,`, `"extra":true,`,
	} {
		bad := append([]byte("{"+field), good[1:]...)
		if _, err := decodeCommand(bad); err == nil {
			t.Fatal("ambiguous or extra DNS probe field accepted")
		}
	}
	for _, replacement := range []string{`"token":null`, `"token":123`, `"token":""`} {
		bad := bytes.Replace(good, []byte(`"token":"`+strings.Repeat("b", 32)+`"`), []byte(replacement), 1)
		if _, err := decodeCommand(bad); err == nil {
			t.Fatal("invalid DNS probe token accepted")
		}
	}
}
func TestInputBoundBeforeAllocation(t *testing.T) {
	out := make(chan inputResult, 1)
	done := make(chan struct{})
	defer close(done)
	go inputLoop(strings.NewReader(strings.Repeat("x", 4097)), out, done)
	select {
	case r := <-out:
		if r.err == nil {
			t.Fatal("oversize accepted")
		}
	case <-time.After(time.Second):
		t.Fatal("input did not terminate")
	}
}
func TestOutputFailureDoesNotEchoSecrets(t *testing.T) {
	var output bytes.Buffer
	if code := run(bytes.NewReader([]byte("secret-not-json\n")), &output); code == 0 || bytes.Contains(output.Bytes(), []byte("secret")) {
		t.Fatal("invalid input accepted or echoed")
	}
}
func TestBoundedOutputBlocksAtMostTwoSeconds(t *testing.T) {
	r, w := io.Pipe()
	done := make(chan struct{})
	output := boundedOutput(w, done, time.Now().Add(55*time.Second))
	start := time.Now()
	if err := output(event{"v": 1}); err == nil {
		t.Fatal("blocked output succeeded")
	}
	if elapsed := time.Since(start); elapsed < 1800*time.Millisecond || elapsed > 3*time.Second {
		t.Fatal("output deadline", elapsed)
	}
	r.Close()
	w.Close()
	close(done)
}
