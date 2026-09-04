package main

import (
	"bytes"
	"context"
	"errors"
	"io"
	"net"
	"syscall"

	"github.com/sagernet/gvisor/pkg/tcpip/adapters/gonet"
	"github.com/sagernet/gvisor/pkg/tcpip/network/ipv4"
)

type localCaseResult struct {
	CaseID    int    `json:"case_id"`
	Sent      bool   `json:"sent"`
	EqualEcho bool   `json:"equal_echo"`
	Error     string `json:"error"`
}

func localError(err error) (string, error) {
	if err == nil {
		return "none", nil
	}
	if errors.Is(err, syscall.ECONNREFUSED) {
		return "refused", nil
	}
	if errors.Is(err, syscall.ECONNRESET) {
		return "reset", nil
	}
	if errors.Is(err, io.EOF) {
		return "eof", nil
	}
	var n net.Error
	if errors.Is(err, context.DeadlineExceeded) || errors.Is(err, syscall.ETIMEDOUT) || (errors.As(err, &n) && n.Timeout()) {
		return "timeout", nil
	}
	return "", err
}

func probeLocal(ctx context.Context, p *livePeer) ([]localCaseResult, error) {
	results := make([]localCaseResult, 0, 4)
	for i := 0; i < 4; i++ {
		if err := ctx.Err(); err != nil {
			return nil, err
		}
		p.watch.mu.Lock()
		p.watch.reject.flows[i].started = true
		f := p.watch.reject.flows[i]
		p.watch.mu.Unlock()
		equal, probeErr := probeFlow(ctx, p.tun, f)
		if err := ctx.Err(); err != nil {
			return nil, err
		} // 阶段/全局截止不属于预期拒绝。
		category, err := localError(probeErr)
		if err != nil {
			return nil, err
		}
		sent, err := p.watch.localFlowResult(i, equal)
		if err != nil {
			return nil, err
		}
		if !equal && category == "none" {
			return nil, errors.New("missing echo without error")
		}
		results = append(results, localCaseResult{CaseID: i + 1, Sent: sent, EqualEcho: equal, Error: category})
	}
	p.watch.mu.Lock()
	health := p.watch.reject.health
	p.watch.mu.Unlock()
	if err := icmpProbe(ctx, p.tun, health.token, health); err != nil {
		return nil, err
	}
	return results, nil
}

func probeFlow(ctx context.Context, m *memoryTun, f localFlow) (bool, error) {
	var c net.Conn
	if f.proto == 6 {
		connectCtx, cancel := context.WithDeadline(ctx, deadline(ctx))
		conn, err := gonet.DialTCPWithBind(connectCtx, m.s, address(peerIP, f.source), address(f.destination, f.port), ipv4.ProtocolNumber)
		cancel()
		if err != nil {
			return false, err
		}
		c = conn
	} else {
		local, remote := address(peerIP, f.source), address(f.destination, f.port)
		conn, err := gonet.DialUDP(m.s, &local, &remote, ipv4.ProtocolNumber)
		if err != nil {
			return false, err
		}
		c = conn
	}
	defer c.Close()
	stop := context.AfterFunc(ctx, func() { c.Close() })
	defer stop()
	if err := c.SetWriteDeadline(deadline(ctx)); err != nil {
		return false, err
	}
	n, err := c.Write(f.payload[:])
	if err != nil {
		return false, err
	}
	if n != 20 {
		return false, io.ErrShortWrite
	}
	if err := c.SetReadDeadline(deadline(ctx)); err != nil {
		return false, err
	}
	var reply [21]byte
	if f.proto == 6 {
		n, err = io.ReadFull(c, reply[:20])
	} else {
		n, err = c.Read(reply[:])
	}
	if err != nil {
		return false, err
	}
	if n != 20 || !bytes.Equal(reply[:n], f.payload[:]) {
		return false, errors.New("invalid echo")
	}
	return true, nil
}
