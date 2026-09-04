package main

import (
	"bufio"
	"bytes"
	"encoding/hex"
	"encoding/json"
	"errors"
	"io"
	"time"
)

type command struct {
	V              int      `json:"v"`
	Op             string   `json:"op"`
	RunID          string   `json:"run_id"`
	DUT            []uint16 `json:"dut_private_key"`
	Peer           []uint16 `json:"peer_private_key"`
	Token          string   `json:"token"`
	Stopped        *bool    `json:"dut_stopped"`
	VirtualTCPPort *uint16  `json:"virtual_tcp_port,omitempty"`
	HostTCPPort    *uint16  `json:"host_tcp_port,omitempty"`
	VirtualUDPPort *uint16  `json:"virtual_udp_port,omitempty"`
	HostUDPPort    *uint16  `json:"host_udp_port,omitempty"`
	Phase          *int     `json:"phase,omitempty"`
}

func hex16(s string) bool {
	if len(s) != 32 {
		return false
	}
	for _, c := range s {
		if !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return false
		}
	}
	return true
}
func decodeCommand(line []byte) (command, error) {
	var c command
	// 首层字段不允许重复，防止两个初始化场景被不同解码端解释成不同操作。
	keys := json.NewDecoder(bytes.NewReader(line))
	first, err := keys.Token()
	if err != nil || first != json.Delim('{') {
		return c, errors.New("object required")
	}
	seen := make(map[string]bool)
	for keys.More() {
		key, err := keys.Token()
		if err != nil {
			return c, errors.New("object key")
		}
		name, ok := key.(string)
		if !ok || seen[name] {
			return c, errors.New("duplicate field")
		}
		seen[name] = true
		var value json.RawMessage
		if keys.Decode(&value) != nil {
			return c, errors.New("object value")
		}
	}
	decoder := json.NewDecoder(bytes.NewReader(line))
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(&c); err != nil {
		return c, errors.New("invalid input")
	}
	if decoder.Decode(new(any)) != io.EOF {
		return c, errors.New("trailing input")
	}
	if c.V != 1 || !hex16(c.RunID) {
		return c, errors.New("protocol identity")
	}
	switch c.Op {
	case "init", "init_udp", "init_reject", "init_dns_probe", "init_domain_http", "init_domain_tls":
		if !hex16(c.Token) || len(c.DUT) != 32 || len(c.Peer) != 32 || c.Stopped != nil {
			return c, errors.New("init fields")
		}
		for _, key := range [][]uint16{c.DUT, c.Peer} {
			for _, v := range key {
				if v > 255 {
					return c, errors.New("key value")
				}
			}
		}
		if bytes.Equal(keyBytes(c.DUT), keyBytes(c.Peer)) {
			return c, errors.New("duplicate key")
		}
	case "probe_icmp", "probe_local", "begin_phase":
		if c.Token != "" || c.DUT != nil || c.Peer != nil || c.Stopped != nil {
			return c, errors.New("probe fields")
		}
	case "shutdown", "finish_phase":
		if c.Token != "" || c.DUT != nil || c.Peer != nil || c.Stopped == nil || !*c.Stopped {
			return c, errors.New("shutdown fields")
		}
	default:
		return c, errors.New("operation")
	}
	if c.Op == "init_reject" {
		ports := []*uint16{c.VirtualTCPPort, c.HostTCPPort, c.VirtualUDPPort, c.HostUDPPort}
		for i, port := range ports {
			if port == nil || *port == 0 || *port == 9090 {
				return c, errors.New("target ports")
			}
			for _, previous := range ports[:i] {
				if *port == *previous {
					return c, errors.New("target ports")
				}
			}
		}
	} else if seen["virtual_tcp_port"] || seen["host_tcp_port"] || seen["virtual_udp_port"] || seen["host_udp_port"] {
		return c, errors.New("unexpected ports")
	}
	if c.Op == "begin_phase" {
		if c.Phase == nil || *c.Phase < 1 || *c.Phase > 3 {
			return c, errors.New("phase")
		}
	} else if seen["phase"] {
		return c, errors.New("unexpected phase")
	}
	allowed := map[string]bool{"v": true, "op": true, "run_id": true}
	switch c.Op {
	case "init", "init_udp", "init_reject", "init_dns_probe", "init_domain_http", "init_domain_tls":
		allowed["dut_private_key"] = true
		allowed["peer_private_key"] = true
		allowed["token"] = true
	case "shutdown", "finish_phase":
		allowed["dut_stopped"] = true
	case "begin_phase":
		allowed["phase"] = true
	}
	if c.Op == "init_reject" {
		allowed["virtual_tcp_port"] = true
		allowed["host_tcp_port"] = true
		allowed["virtual_udp_port"] = true
		allowed["host_udp_port"] = true
	}
	if len(allowed) != len(seen) {
		return c, errors.New("field set")
	}
	for key := range seen {
		if !allowed[key] {
			return c, errors.New("field set")
		}
	}
	return c, nil
}
func keyBytes(input []uint16) []byte {
	key := make([]byte, len(input))
	for i, v := range input {
		key[i] = byte(v)
	}
	if len(key) == 32 {
		key[0] &= 248
		key[31] &= 127
		key[31] |= 64
	}
	return key
}

type inputResult struct {
	command command
	err     error
}

func inputLoop(in io.Reader, out chan<- inputResult, done <-chan struct{}) {
	r := bufio.NewReaderSize(in, 4096)
	total := 0
	for {
		line, err := r.ReadSlice('\n')
		total += len(line)
		if len(line) > 4096 || total > 16384 || err == bufio.ErrBufferFull {
			err = errors.New("input limit")
		}
		var c command
		if err == nil {
			c, err = decodeCommand(line)
		}
		select {
		case out <- inputResult{c, err}:
		case <-done:
			return
		}
		if err != nil {
			return
		}
	}
}

type event map[string]any

func emit(out io.Writer, e event) error {
	b, err := json.Marshal(e)
	if err != nil {
		return err
	}
	b = append(b, '\n')
	if len(b) > 4096 {
		return errors.New("output limit")
	}
	n, err := out.Write(b)
	if err != nil {
		return err
	}
	if n != len(b) {
		return io.ErrShortWrite
	}
	return nil
}

type outputRequest struct {
	value  event
	result chan error
}

// 单一写者避免超时后的后续帧交错；停止读取的父进程不能阻塞状态机。
func boundedOutput(out io.Writer, done <-chan struct{}, hardDeadline time.Time) func(event) error {
	queue := make(chan outputRequest)
	go func() {
		for {
			select {
			case <-done:
				return
			case r := <-queue:
				r.result <- emit(out, r.value)
			}
		}
	}()
	failed := false
	total := 0
	return func(e event) error {
		if failed {
			return errors.New("output unavailable")
		}
		b, err := json.Marshal(e)
		if err != nil {
			return err
		}
		total += len(b) + 1
		if total > 16384 {
			failed = true
			return errors.New("output limit")
		}
		request := outputRequest{e, make(chan error, 1)}
		remaining := time.Until(hardDeadline)
		if remaining <= 0 {
			failed = true
			return errors.New("output deadline")
		}
		if remaining > 2*time.Second {
			remaining = 2 * time.Second
		}
		timer := time.NewTimer(remaining)
		defer timer.Stop()
		select {
		case queue <- request:
		case <-timer.C:
			failed = true
			return errors.New("output timeout")
		}
		select {
		case err := <-request.result:
			if err != nil {
				failed = true
			}
			return err
		case <-timer.C:
			failed = true
			return errors.New("output timeout")
		}
	}
}
func tokenBytes(s string) []byte { b, _ := hex.DecodeString(s); return b }
