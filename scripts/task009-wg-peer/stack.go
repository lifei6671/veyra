package main

import (
	"bytes"
	"context"
	"encoding/binary"
	"errors"
	"io"
	"net"
	"os"
	"sync"
	"time"

	"github.com/sagernet/gvisor/pkg/buffer"
	"github.com/sagernet/gvisor/pkg/tcpip"
	"github.com/sagernet/gvisor/pkg/tcpip/adapters/gonet"
	"github.com/sagernet/gvisor/pkg/tcpip/header"
	"github.com/sagernet/gvisor/pkg/tcpip/link/channel"
	"github.com/sagernet/gvisor/pkg/tcpip/network/ipv4"
	"github.com/sagernet/gvisor/pkg/tcpip/stack"
	"github.com/sagernet/gvisor/pkg/tcpip/transport/icmp"
	"github.com/sagernet/gvisor/pkg/tcpip/transport/tcp"
	"github.com/sagernet/gvisor/pkg/tcpip/transport/udp"
	"github.com/sagernet/gvisor/pkg/waiter"
	"github.com/sagernet/wireguard-go/tun"
)

type memoryTun struct {
	s      *stack.Stack
	link   *channel.Endpoint
	events chan tun.Event
	ctx    context.Context
	cancel context.CancelFunc
	once   sync.Once
	watch  *observer
}

func newMemoryTun(address [4]byte, watch *observer) (*memoryTun, error) {
	ctx, cancel := context.WithCancel(context.Background())
	m := &memoryTun{link: channel.New(64, 1280, ""), events: make(chan tun.Event, 1), ctx: ctx, cancel: cancel, watch: watch}
	ipv4Factory := ipv4.NewProtocol
	if watch != nil && watch.reject != nil {
		ipv4Factory = ipv4.NewProtocolWithOptions(ipv4.Options{AllowExternalLoopbackTraffic: true})
	}
	m.s = stack.New(stack.Options{NetworkProtocols: []stack.NetworkProtocolFactory{ipv4Factory}, TransportProtocols: []stack.TransportProtocolFactory{tcp.NewProtocol, udp.NewProtocol, icmp.NewProtocol4}})
	if err := m.s.CreateNIC(1, m.link); err != nil {
		m.Close()
		return nil, errors.New(err.String())
	}
	if err := m.s.AddProtocolAddress(1, tcpip.ProtocolAddress{Protocol: ipv4.ProtocolNumber, AddressWithPrefix: tcpip.AddrFrom4(address).WithPrefix()}, stack.AddressProperties{}); err != nil {
		m.Close()
		return nil, errors.New(err.String())
	}
	m.s.SetRouteTable([]tcpip.Route{{Destination: header.IPv4EmptySubnet, NIC: 1}})
	m.events <- tun.EventUp
	return m, nil
}
func (m *memoryTun) File() *os.File           { return nil }
func (m *memoryTun) MTU() (int, error)        { return 1280, nil }
func (m *memoryTun) Name() (string, error)    { return "task009-memory", nil }
func (m *memoryTun) Events() <-chan tun.Event { return m.events }
func (m *memoryTun) BatchSize() int           { return 1 }
func (m *memoryTun) Read(bufs [][]byte, sizes []int, offset int) (int, error) {
	if len(bufs) < 1 || len(sizes) < 1 || offset < 0 || offset > len(bufs[0]) {
		return 0, errors.New("tun read arguments")
	}
	p := m.link.ReadContext(m.ctx)
	if p == nil {
		return 0, os.ErrClosed
	}
	defer p.DecRef()
	n := 0
	for _, segment := range p.AsSlices() {
		if len(segment) > len(bufs[0])-offset-n {
			return 0, io.ErrShortBuffer
		}
		n += copy(bufs[0][offset+n:], segment)
	}
	if m.watch != nil {
		m.watch.inspect(bufs[0][offset:offset+n], false)
	}
	sizes[0] = n
	return 1, nil
}
func (m *memoryTun) Write(bufs [][]byte, offset int) (int, error) {
	select {
	case <-m.ctx.Done():
		return 0, os.ErrClosed
	default:
	}
	for i, b := range bufs {
		if offset < 0 || offset >= len(b) {
			return i, errors.New("tun write arguments")
		}
		b = b[offset:]
		if m.watch != nil {
			m.watch.inspect(b, true)
		}
		p := stack.NewPacketBuffer(stack.PacketBufferOptions{Payload: buffer.MakeWithData(bytes.Clone(b))})
		m.link.InjectInbound(ipv4.ProtocolNumber, p)
		p.DecRef()
	}
	return len(bufs), nil
}
func (m *memoryTun) Close() error {
	m.once.Do(func() {
		m.cancel()
		close(m.events)
		m.s.Close()
		for _, e := range m.s.CleanupEndpoints() {
			e.Abort()
		}
		m.link.Close()
		m.s.Wait()
	})
	return nil
}
func address(ip [4]byte, port uint16) tcpip.FullAddress {
	return tcpip.FullAddress{NIC: 1, Addr: tcpip.AddrFrom4(ip), Port: port}
}
func deadline(ctx context.Context) time.Time {
	d := time.Now().Add(2 * time.Second)
	if global, ok := ctx.Deadline(); ok && global.Before(d) {
		d = global
	}
	return d
}
func icmpProbe(ctx context.Context, m *memoryTun, token []byte, watch *observer) error {
	var queue waiter.Queue
	e, err := m.s.NewEndpoint(icmp.ProtocolNumber4, ipv4.ProtocolNumber, &queue)
	if err != nil {
		return errors.New(err.String())
	}
	if err = e.Bind(address(peerIP, 9)); err != nil {
		e.Close()
		return errors.New(err.String())
	}
	if err = e.Connect(address(dutIP, 0)); err != nil {
		e.Close()
		return errors.New(err.String())
	}
	c := gonet.NewUDPConn(&queue, e)
	defer c.Close()
	if watch != nil {
		watch.mu.Lock()
		watch.icmpActive = true
		watch.mu.Unlock()
	}
	for seq := 1; seq <= 3; seq++ {
		if err := ctx.Err(); err != nil {
			return err
		}
		if err := c.SetDeadline(deadline(ctx)); err != nil {
			return err
		}
		request := make([]byte, 8+len(token)+1)
		request[0] = 8
		binary.BigEndian.PutUint16(request[4:6], 9)
		binary.BigEndian.PutUint16(request[6:8], uint16(seq))
		copy(request[8:], token)
		request[len(request)-1] = byte(seq)
		if n, err := c.Write(request); err != nil || n != len(request) {
			return errors.Join(err, io.ErrShortWrite)
		}
		var reply [128]byte
		n, err := c.Read(reply[:])
		if err != nil {
			return err
		}
		if n != len(request) || reply[0] != 0 || reply[1] != 0 || binary.BigEndian.Uint16(reply[4:6]) != 9 || binary.BigEndian.Uint16(reply[6:8]) != uint16(seq) || !bytes.Equal(reply[8:n], request[8:]) {
			return errors.New("ICMP reply payload")
		}
	}
	if watch != nil {
		watch.mu.Lock()
		defer watch.mu.Unlock()
		if watch.err != nil {
			return watch.err
		}
		if watch.icmpSent != 3 || watch.icmpReceived != 3 {
			return errors.New("ICMP count")
		}
	}
	return nil
}

// 自测把两个内存设备相连，不产生操作系统 TCP/UDP listener。
func selftest(ctx context.Context) error {
	watch := newObserver([]byte("0123456789abcdef"))
	watch.allowUDP = true
	a, err := newMemoryTun(peerIP, watch)
	if err != nil {
		return err
	}
	b, err := newMemoryTun(dutIP, nil)
	if err != nil {
		a.Close()
		return err
	}
	bridgeCtx, cancel := context.WithCancel(ctx)
	var bridges sync.WaitGroup
	bridge := func(from, to *memoryTun) {
		defer bridges.Done()
		for {
			p := from.link.ReadContext(bridgeCtx)
			if p == nil {
				return
			}
			var raw []byte
			for _, s := range p.AsSlices() {
				raw = append(raw, s...)
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
	defer func() { cancel(); a.Close(); b.Close(); bridges.Wait() }()
	listener, te := gonet.ListenTCP(a.s, address(peerIP, 18080), ipv4.ProtocolNumber)
	if te != nil {
		return te
	}
	defer listener.Close()
	serverDone := make(chan error, 1)
	go func() {
		c, err := listener.Accept()
		if err != nil {
			serverDone <- err
			return
		}
		defer c.Close()
		if err = c.SetDeadline(deadline(ctx)); err != nil {
			serverDone <- err
			return
		}
		var request [4]byte
		if _, err = io.ReadFull(c, request[:]); err == nil && string(request[:]) != "HEAD" {
			err = errors.New("selftest TCP request")
		}
		if err == nil {
			_, err = io.WriteString(c, response)
		}
		serverDone <- err
	}()
	client, err := gonet.DialContextTCP(ctx, b.s, address(peerIP, 18080), ipv4.ProtocolNumber)
	if err != nil {
		listener.Close()
		<-serverDone
		return err
	}
	if err = client.SetDeadline(deadline(ctx)); err == nil {
		_, err = client.Write([]byte("HEAD"))
	}
	buf := make([]byte, len(response))
	if err == nil {
		_, err = io.ReadFull(client, buf)
	}
	if err == nil && string(buf) != response {
		err = errors.New("selftest TCP response")
	}
	client.Close()
	select {
	case se := <-serverDone:
		err = errors.Join(err, se)
	case <-ctx.Done():
		err = ctx.Err()
	}
	if err != nil {
		return err
	}
	ackCtx, ackCancel := context.WithDeadline(ctx, deadline(ctx))
	err = watch.waitACK(ackCtx)
	ackCancel()
	if err != nil {
		return err
	}
	// UDP 使用同一内存链，但不混入实际 WG/TCP 观察器。
	local, remote := address(peerIP, 18081), address(dutIP, 18082)
	u1, err := gonet.DialUDP(a.s, &local, &remote, ipv4.ProtocolNumber)
	if err != nil {
		return err
	}
	defer u1.Close()
	u2, err := gonet.DialUDP(b.s, &remote, &local, ipv4.ProtocolNumber)
	if err != nil {
		return err
	}
	defer u2.Close()
	for _, c := range []net.Conn{u1, u2} {
		if err := c.SetDeadline(deadline(ctx)); err != nil {
			return err
		}
	}
	payload := []byte("task009-udp-selftest")
	if _, err = u1.Write(payload); err != nil {
		return err
	}
	recv := make([]byte, 64)
	n, err := u2.Read(recv)
	if err != nil || !bytes.Equal(recv[:n], payload) {
		return errors.Join(err, errors.New("selftest UDP request"))
	}
	if _, err = u2.Write(recv[:n]); err != nil {
		return err
	}
	n, err = u1.Read(recv)
	if err != nil || !bytes.Equal(recv[:n], payload) {
		return errors.Join(err, errors.New("selftest UDP response"))
	}
	return icmpProbe(ctx, a, watch.token, watch)
}
