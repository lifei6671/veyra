package main

import (
	"bytes"
	"context"
	"encoding/binary"
	"errors"
	"io"
	"net"

	"github.com/sagernet/gvisor/pkg/tcpip"
	"github.com/sagernet/gvisor/pkg/tcpip/adapters/gonet"
	"github.com/sagernet/gvisor/pkg/tcpip/network/ipv4"
	"github.com/sagernet/gvisor/pkg/tcpip/transport/udp"
	"github.com/sagernet/gvisor/pkg/waiter"
)

const udpSize = 20

func udpPayloadValid(payload, token []byte, seq int) bool {
	return len(payload) == udpSize && len(token) == 16 && seq >= 1 && seq <= 3 && bytes.Equal(payload[:16], token) && binary.BigEndian.Uint32(payload[16:]) == uint32(seq)
}

func (o *observer) udp(p packet, incoming bool) {
	source, dest := binary.BigEndian.Uint16(p.body[:2]), binary.BigEndian.Uint16(p.body[2:4])
	if incoming {
		if source == 0 || dest != 18081 || !udpPayloadValid(p.body[8:], o.token, o.udpReceived+1) || o.udpReceived != o.udpReplied {
			o.err = errors.New("UDP request")
			return
		}
		if o.udpPort == 0 {
			o.udpPort = source
		}
		if source != o.udpPort {
			o.err = errors.New("UDP request tuple")
			return
		}
		o.udpReceived++
	} else {
		if source != 18081 || dest != o.udpPort || !udpPayloadValid(p.body[8:], o.token, o.udpReplied+1) || o.udpReplied >= o.udpReceived {
			o.err = errors.New("UDP response")
			return
		}
		o.udpReplied++
	}
}
func (o *observer) waitUDP(ctx context.Context) error {
	for {
		if err := ctx.Err(); err != nil {
			return err
		}
		o.mu.Lock()
		err, rx, tx, changed := o.err, o.udpReceived, o.udpReplied, o.changed
		o.mu.Unlock()
		if err != nil {
			return err
		}
		if rx == 3 && tx == 3 {
			return nil
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-changed:
		}
	}
}

type udpService struct {
	conn     *gonet.UDPConn
	endpoint tcpip.Endpoint
	queue    waiter.Queue
	entry    waiter.Entry
	readable chan struct{}
}

func newUDPService(m *memoryTun) (*udpService, error) {
	u := &udpService{}
	e, err := m.s.NewEndpoint(udp.ProtocolNumber, ipv4.ProtocolNumber, &u.queue)
	if err != nil {
		return nil, errors.New(err.String())
	}
	u.endpoint = e
	if err = e.Bind(address(peerIP, 18081)); err != nil {
		e.Close()
		return nil, errors.New(err.String())
	}
	u.entry, u.readable = waiter.NewChannelEntry(waiter.ReadableEvents)
	u.queue.EventRegister(&u.entry)
	u.conn = gonet.NewUDPConn(&u.queue, e)
	return u, nil
}
func (u *udpService) close() error {
	u.queue.EventUnregister(&u.entry)
	return u.conn.Close()
}
func (u *udpService) waitReadable(ctx context.Context) error {
	for {
		if err := ctx.Err(); err != nil {
			return err
		}
		if u.endpoint.Readiness(waiter.ReadableEvents) != 0 {
			return nil
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-u.readable:
		}
	}
}

func serveUDP(ctx context.Context, p *livePeer) error {
	var port int
	for seq := 1; seq <= 3; seq++ {
		if err := p.udp.waitReadable(ctx); err != nil {
			return err
		}
		if err := p.udp.conn.SetReadDeadline(deadline(ctx)); err != nil {
			return err
		}
		var payload [udpSize + 1]byte // 多一字节使过长/截断数据报不能伪装成正确20字节。
		n, source, err := p.udp.conn.ReadFrom(payload[:])
		if err != nil {
			return err
		}
		remote, ok := source.(*net.UDPAddr)
		if !ok || !remote.IP.Equal(net.IP(dutIP[:])) || remote.Port <= 0 || !udpPayloadValid(payload[:n], p.watch.token, seq) {
			return errors.New("UDP service request")
		}
		if port == 0 {
			port = remote.Port
		}
		if remote.Port != port {
			return errors.New("UDP service tuple")
		}
		if err := ctx.Err(); err != nil {
			return err
		}
		if err := p.udp.conn.SetWriteDeadline(deadline(ctx)); err != nil {
			return err
		}
		written, err := p.udp.conn.WriteTo(payload[:n], remote)
		if err != nil {
			return err
		}
		if written != udpSize {
			return io.ErrShortWrite
		}
	}
	boundaryCtx, cancel := context.WithDeadline(ctx, deadline(ctx))
	defer cancel()
	return p.watch.waitUDP(boundaryCtx)
}
