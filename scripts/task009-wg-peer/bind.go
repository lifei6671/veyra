package main

import (
	"errors"
	"net"
	"net/netip"
	"sync"
	"time"

	"github.com/sagernet/wireguard-go/conn"
)

type loopEndpoint struct{ address netip.AddrPort }

func (e *loopEndpoint) ClearSrc()           {}
func (e *loopEndpoint) SrcToString() string { return "" }
func (e *loopEndpoint) DstToString() string { return e.address.String() }
func (e *loopEndpoint) DstIP() netip.Addr   { return e.address.Addr() }
func (e *loopEndpoint) SrcIP() netip.Addr   { return netip.Addr{} }
func (e *loopEndpoint) DstToBytes() []byte {
	b, _ := e.address.MarshalBinary() // 地址仅由成功的 ParseEndpoint/ReadFrom 构造。
	return b
}

type loopBind struct {
	mu    sync.Mutex
	udp   *net.UDPConn
	port  uint16
	fault func(error)
}

func (b *loopBind) Open(port uint16) ([]conn.ReceiveFunc, uint16, error) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.udp != nil || port != 0 {
		return nil, 0, errors.New("bind state")
	}
	u, err := net.ListenUDP("udp4", &net.UDPAddr{IP: net.IPv4(127, 0, 0, 1)})
	if err != nil {
		return nil, 0, err
	}
	p := uint16(u.LocalAddr().(*net.UDPAddr).Port)
	if p == 9090 {
		_ = u.Close()
		return nil, 0, errors.New("reserved port")
	}
	b.udp, b.port = u, p
	receive := func(packets [][]byte, sizes []int, endpoints []conn.Endpoint) (int, error) {
		if len(packets) < 1 || len(sizes) < 1 || len(endpoints) < 1 {
			return 0, errors.New("receive batch")
		}
		n, source, readErr := u.ReadFromUDPAddrPort(packets[0])
		if readErr != nil {
			return 0, readErr
		}
		if !source.Addr().Is4() || !source.Addr().IsLoopback() {
			return 0, errors.New("nonloopback source")
		}
		sizes[0], endpoints[0] = n, &loopEndpoint{source}
		return 1, nil
	}
	return []conn.ReceiveFunc{receive}, p, nil
}

func (b *loopBind) Close() error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.udp == nil {
		return nil
	}
	err := b.udp.Close()
	b.udp = nil
	return err
}
func (b *loopBind) SetMark(mark uint32) error {
	if mark != 0 {
		return errors.New("mark prohibited")
	}
	return nil
}
func (b *loopBind) BatchSize() int { return 1 }
func (b *loopBind) ParseEndpoint(value string) (conn.Endpoint, error) {
	address, err := netip.ParseAddrPort(value)
	if err != nil || !address.Addr().Is4() || !address.Addr().IsLoopback() || address.Port() == 0 {
		return nil, errors.New("endpoint prohibited")
	}
	return &loopEndpoint{address}, nil
}
func (b *loopBind) Send(packets [][]byte, endpoint conn.Endpoint, offset int) error {
	e, ok := endpoint.(*loopEndpoint)
	if !ok || !e.address.Addr().Is4() || !e.address.Addr().IsLoopback() || len(packets) != 1 || offset < 0 || offset > len(packets[0]) {
		return errors.New("send arguments")
	}
	b.mu.Lock()
	u := b.udp
	b.mu.Unlock()
	if u == nil {
		return net.ErrClosed
	}
	if err := u.SetWriteDeadline(time.Now().Add(2 * time.Second)); err != nil {
		return err
	}
	_, err := u.WriteToUDPAddrPort(packets[0][offset:], e.address)
	return err
}
func (b *loopBind) SetReservedForEndpoint(_ netip.AddrPort, reserved [3]byte) {
	if reserved != [3]byte{} {
		b.fault(errors.New("reserved prohibited"))
	}
}
