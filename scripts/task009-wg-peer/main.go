package main

import (
	"bufio"
	"context"
	"crypto/ecdh"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"io"
	"net"
	"net/http"
	"os"
	"strings"
	"sync"
	"time"

	"github.com/sagernet/gvisor/pkg/tcpip/adapters/gonet"
	"github.com/sagernet/gvisor/pkg/tcpip/network/ipv4"
	"github.com/sagernet/wireguard-go/device"
)

type livePeer struct {
	tun      *memoryTun
	bind     *loopBind
	wg       *device.Device
	listener net.Listener
	udp      *udpService
	watch    *observer
	workers  sync.WaitGroup
}

func newLive(c command) (*livePeer, string, string, error) {
	dut, err := ecdh.X25519().NewPrivateKey(keyBytes(c.DUT))
	if err != nil {
		return nil, "", "", err
	}
	peer, err := ecdh.X25519().NewPrivateKey(keyBytes(c.Peer))
	if err != nil {
		return nil, "", "", err
	}
	p := &livePeer{watch: newObserver(tokenBytes(c.Token))}
	p.watch.udpMode = c.Op == "init_udp"
	if c.Op == "init_domain_http" || c.Op == "init_domain_tls" {
		p.watch.domain = &domainObserver{tls: c.Op == "init_domain_tls"}
	}
	if c.Op == "init_reject" {
		p.watch.reject = &rejectObserver{ports: [4]uint16{*c.VirtualTCPPort, *c.HostTCPPort, *c.VirtualUDPPort, *c.HostUDPPort}}
	}
	p.tun, err = newMemoryTun(peerIP, p.watch)
	if err != nil {
		return nil, "", "", err
	}
	if c.Op == "init_dns_probe" {
		p.tun.dnsProbe = &dnsProbeSink{fail: p.watch.fail}
	} else if p.watch.udpMode {
		p.udp, err = newUDPService(p.tun)
	} else if p.watch.domain != nil {
		p.listener, err = gonet.ListenTCP(p.tun.s, address(domainIP, p.watch.domain.port()), ipv4.ProtocolNumber)
	} else {
		p.listener, err = gonet.ListenTCP(p.tun.s, address(peerIP, 18080), ipv4.ProtocolNumber)
	}
	if err != nil {
		p.tun.Close()
		return nil, "", "", err
	}
	p.bind = &loopBind{fault: p.watch.fail}
	// 库日志不进入 stdout/stderr；固定错误只通过观察器传播。
	logger := &device.Logger{Verbosef: func(string, ...any) {}, Errorf: func(string, ...any) { p.watch.fail(errors.New("WG error")) }}
	p.wg = device.NewDevice(context.Background(), p.tun, p.bind, logger, 1)
	config := "private_key=" + hex.EncodeToString(peer.Bytes()) + "\npublic_key=" + hex.EncodeToString(dut.PublicKey().Bytes()) + "\nallowed_ip=198.18.0.1/32\n"
	if c.Op == "init_reject" {
		config += "allowed_ip=172.26.192.1/32\n"
	}
	if err = p.wg.IpcSet(config); err == nil {
		err = p.wg.Up()
	}
	if err != nil {
		p.close()
		return nil, "", "", errors.New("WG setup")
	}
	return p, base64.StdEncoding.EncodeToString(peer.PublicKey().Bytes()), base64.StdEncoding.EncodeToString(dut.PublicKey().Bytes()), nil
}
func (p *livePeer) close() error {
	var err error
	if p.udp != nil {
		err = p.udp.close()
	} else if p.listener != nil {
		err = p.listener.Close()
	}
	p.wg.Close()
	p.workers.Wait()
	return err
}
func serveHTTP(ctx context.Context, p *livePeer, token string) error {
	return serveHTTPURI(ctx, p, "/task009-wg?token="+token, p.watch)
}
func serveHTTPURI(ctx context.Context, p *livePeer, uri string, watch *observer) error {
	c, err := p.listener.Accept()
	if err != nil {
		return err
	}
	defer c.Close()
	stopCancel := context.AfterFunc(ctx, func() { c.Close() })
	defer stopCancel()
	if err := ctx.Err(); err != nil {
		return err
	}
	if err = c.SetDeadline(deadline(ctx)); err != nil {
		return err
	}
	// 限制头字节，避免 ReadRequest 无界读取；只解析已经取得的完整头。
	r := bufio.NewReader(c)
	var header strings.Builder
	for header.Len() < 16384 {
		b, err := r.ReadByte()
		if err != nil {
			return err
		}
		header.WriteByte(b)
		if strings.HasSuffix(header.String(), "\r\n\r\n") {
			break
		}
	}
	if header.Len() >= 16384 {
		return errors.New("HTTP header limit")
	}
	request, err := http.ReadRequest(bufio.NewReader(strings.NewReader(header.String())))
	if err != nil {
		return err
	}
	if request.Method != "HEAD" || request.RequestURI != uri || request.Host != "198.18.0.2:18080" || request.ContentLength > 0 || len(request.TransferEncoding) != 0 {
		return errors.New("HTTP request")
	}
	if err = c.SetWriteDeadline(deadline(ctx)); err != nil {
		return err
	}
	if err := ctx.Err(); err != nil {
		return err
	}
	if n, err := io.WriteString(c, response); err != nil || n != len(response) {
		return errors.Join(err, io.ErrShortWrite)
	}
	ackCtx, cancel := context.WithDeadline(ctx, deadline(ctx))
	defer cancel()
	return watch.waitACK(ackCtx)
}

func run(in io.Reader, out io.Writer) int {
	born := time.Now()
	hard := time.NewTimer(55 * time.Second)
	defer hard.Stop()
	inputs := make(chan inputResult, 4)
	done := make(chan struct{})
	defer close(done)
	go inputLoop(in, inputs, done)
	output := boundedOutput(out, done, born.Add(55*time.Second))
	var initial command
	select {
	case input := <-inputs:
		if input.err != nil || (input.command.Op != "init" && input.command.Op != "init_udp" && input.command.Op != "init_reject" && input.command.Op != "init_dns_probe" && input.command.Op != "init_domain_http" && input.command.Op != "init_domain_tls") {
			return 1
		}
		initial = input.command
	case <-time.After(5 * time.Second):
		return 1
	}
	if initial.Op == "init_reject" {
		return runReject(initial, born, inputs, out, done)
	}
	base := func(kind string) event { return event{"v": 1, "event": kind, "run_id": initial.RunID} }
	failed := false
	cancelBusiness := func() {}
	failure := func(stage, code string) {
		if !failed {
			failed = true
			cancelBusiness()
			e := base("failed")
			e["stage"] = stage
			e["code"] = code
			if output(e) != nil {
				failed = true
			}
		}
	}
	selfCtx, selfCancel := context.WithDeadline(context.Background(), born.Add(10*time.Second))
	selfDone := make(chan error, 1)
	go func() {
		ctx, cancel := context.WithTimeout(selfCtx, 5*time.Second)
		defer cancel()
		selfDone <- selftest(ctx)
	}()
	select {
	case err := <-selfDone:
		if err != nil {
			selfCancel()
			failure("selftest", "io_error")
			return 1
		}
	case <-selfCtx.Done():
		selfCancel()
		failure("selftest", "timeout")
		return 1
	}
	selfCancel()
	p, peerPublic, dutPublic, err := newLive(initial)
	if err != nil {
		failure("bind", "resource_error")
		return 1
	}
	ready := base("ready")
	ready["udp_port"] = p.bind.port
	ready["peer_public_key"] = peerPublic
	ready["dut_public_key"] = dutPublic
	ready["selftest"] = map[string]bool{"tcp": true, "udp": true, "icmp": true}
	if err = output(ready); err != nil {
		p.close()
		return 1
	}
	workCtx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	cancelBusiness = cancel
	defer cancel()
	var httpDone, udpDone chan error
	var domainDone chan domainResult
	domainOK := false
	dnsProbe := p.tun.dnsProbe != nil
	if p.watch.domain != nil {
		p.workers.Add(1)
		domainDone = make(chan domainResult, 1)
		go func() { defer p.workers.Done(); domainDone <- serveDomain(workCtx, p) }()
	} else if p.watch.udpMode {
		p.workers.Add(1)
		udpDone = make(chan error, 1)
		go func() { defer p.workers.Done(); udpDone <- serveUDP(workCtx, p) }()
	} else if !dnsProbe {
		p.workers.Add(1)
		httpDone = make(chan error, 1)
		go func() { defer p.workers.Done(); httpDone <- serveHTTP(workCtx, p, initial.Token) }()
	}
	icmpDone := make(chan error, 1)
	tcpOK, icmpStarted, icmpOK := false, false, false
	udpOK := false
	workDone := workCtx.Done()
	for {
		select {
		case result := <-domainDone:
			domainDone = nil
			rx, tx, observeErr := p.watch.result()
			if result.err != nil || observeErr != nil || rx == 0 || tx == 0 || workCtx.Err() != nil {
				failure("protocol", "unexpected_packet")
				continue
			}
			if failed {
				continue
			}
			domainOK = true
			e := base(p.watch.domain.mode())
			e["destination_matches"], e["authenticated"] = true, true
			e["rx_tcp_packets"], e["tx_tcp_packets"] = rx, tx
			if p.watch.domain.tls {
				e["connections"], e["sni_matches"], e["https_success"] = 1, true, false
				e["client_hello_bytes"] = result.bytes
			} else {
				e["requests"], e["host_matches"] = 1, true
				e["response_status"], e["response_acked"] = 204, true
			}
			if output(e) != nil {
				failure("protocol", "io_error")
			}
		case uErr := <-udpDone:
			udpDone = nil
			if uErr != nil {
				if errors.Is(workCtx.Err(), context.DeadlineExceeded) {
					failure("deadline", "timeout")
				} else {
					failure("protocol", "unexpected_packet")
				}
				continue
			}
			if failed {
				continue
			}
			udpOK = true
			e := base("udp")
			e["received"] = 3
			e["replied"] = 3
			e["sequences"] = []int{1, 2, 3}
			e["rx_udp_packets"] = 3
			e["tx_udp_packets"] = 3
			e["payloads_valid"] = true
			e["addresses_valid"] = true
			e["authenticated"] = true
			if output(e) != nil {
				failure("protocol", "io_error")
			}
			p.workers.Add(1)
			go func() {
				defer p.workers.Done()
				if p.udp.waitReadable(workCtx) == nil {
					p.watch.fail(errors.New("extra UDP datagram"))
				}
			}()
		case hErr := <-httpDone:
			httpDone = nil
			if hErr != nil {
				failure("tcp", "io_error")
				continue
			}
			if failed {
				continue
			}
			rx, tx, observeErr := p.watch.result()
			if observeErr != nil || rx == 0 || tx == 0 {
				failure("tcp", "unexpected_packet")
				continue
			}
			tcpOK = true
			e := base("tcp")
			e["requests"] = 1
			e["response_status"] = 204
			e["rx_tcp_packets"] = rx
			e["tx_tcp_packets"] = tx
			e["authenticated"] = true
			e["response_acked"] = true
			if output(e) != nil {
				failure("protocol", "io_error")
			}
			p.workers.Add(1)
			go func() {
				defer p.workers.Done()
				c, e := p.listener.Accept()
				if e == nil {
					if c.Close() != nil {
						p.watch.fail(errors.New("extra close"))
					}
					p.watch.fail(errors.New("extra connection"))
				}
			}()
		case iErr := <-icmpDone:
			icmpDone = nil
			if iErr != nil {
				failure("icmp", "unexpected_packet")
				continue
			}
			if failed {
				continue
			}
			icmpOK = true
			e := base("icmp")
			e["sent"] = 3
			e["received"] = 3
			e["id"] = 9
			e["sequences"] = []int{1, 2, 3}
			e["payloads_valid"] = true
			e["addresses_valid"] = true
			if output(e) != nil {
				failure("protocol", "io_error")
			}
		case input := <-inputs:
			if input.err != nil {
				failure("protocol", "invalid_input")
				inputs = nil
				continue
			}
			c := input.command
			if c.RunID != initial.RunID {
				failure("protocol", "invalid_input")
				continue
			}
			if c.Op == "shutdown" {
				if (dnsProbe || p.watch.domain != nil) && workCtx.Err() != nil {
					failure("deadline", "timeout")
				}
				cancel()
				closed := make(chan error, 1)
				go func() { closed <- p.close() }()
				select {
				case err := <-closed:
					if err != nil {
						failure("cleanup", "resource_error")
						return 1
					}
				case <-time.After(2 * time.Second):
					failure("cleanup", "timeout")
					return 1
				}
				_, _, watchErr := p.watch.result()
				if watchErr != nil {
					failure("cleanup", "unexpected_packet")
				}
				e := base("stopped")
				e["resources_closed"] = true
				if dnsProbe {
					e["discarded_packets"], e["discarded_bytes"] = p.tun.dnsProbe.result()
				}
				if p.watch.domain != nil {
					e["mode"] = p.watch.domain.mode()
				}
				if output(e) != nil {
					return 1
				}
				if failed || (p.watch.domain != nil && !domainOK) || (p.watch.domain == nil && !dnsProbe && ((p.watch.udpMode && !udpOK) || (!p.watch.udpMode && (!tcpOK || !icmpOK)))) {
					return 1
				}
				return 0
			}
			if c.Op != "probe_icmp" || dnsProbe || p.watch.domain != nil || p.watch.udpMode || failed || !tcpOK || icmpStarted {
				failure("protocol", "invalid_input")
				continue
			}
			icmpStarted = true
			p.workers.Add(1)
			go func() { defer p.workers.Done(); icmpDone <- icmpProbe(workCtx, p.tun, p.watch.token, p.watch) }()
		case <-p.watch.wake:
			_, _, err := p.watch.result()
			if err != nil {
				if p.watch.udpMode || dnsProbe {
					failure("protocol", "unexpected_packet")
				} else {
					failure("tcp", "unexpected_packet")
				}
			}
		case <-workDone:
			failure("deadline", "timeout")
			workDone = nil
		case <-hard.C:
			failure("deadline", "timeout")
			return 1 // 进程硬退出释放自有资源，父应已先停 DUT。
		}
	}
}
func main() { os.Exit(run(os.Stdin, os.Stdout)) }
