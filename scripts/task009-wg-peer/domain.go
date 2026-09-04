package main

import (
	"bufio"
	"context"
	"crypto/tls"
	"encoding/binary"
	"errors"
	"io"
	"net"
	"net/http"
	"strings"
	"time"
)

var domainIP = [4]byte{198, 20, 0, 255}

const domainHost = "veyra.disign.me"

// 这些字段仅在两个封闭模式设置，全部报文状态由 observer.mu 保护。
type domainObserver struct {
	tls         bool
	haveSYN     bool
	clientStart uint32
	inputEnd    uint32
	headerBytes uint32
}

func (d *domainObserver) port() uint16 {
	if d.tls {
		return 18443
	}
	return 18080
}

func (d *domainObserver) mode() string {
	if d.tls {
		return "domain_tls"
	}
	return "domain_http"
}

func (d *domainObserver) inspect(o *observer, p packet, raw []byte, incoming bool) {
	if len(raw) > 1280 || p.proto != 6 || (incoming && (p.src != dutIP || p.dst != domainIP)) || (!incoming && (p.src != domainIP || p.dst != dutIP)) {
		o.err = errors.New("domain packet address/protocol")
		return
	}
	b := p.body
	source, dest := binary.BigEndian.Uint16(b), binary.BigEndian.Uint16(b[2:])
	seq, flags := binary.BigEndian.Uint32(b[4:8]), b[13]
	if incoming {
		if !d.haveSYN {
			if source == 0 || dest != d.port() || flags != 2 || len(b) != int(b[12]>>4)*4 {
				o.err = errors.New("domain first SYN")
				return
			}
			d.haveSYN, d.clientStart, o.clientPort = true, seq+1, source
		}
		if source != o.clientPort || dest != d.port() || (flags&2 != 0 && (flags&16 != 0 || seq+1 != d.clientStart)) {
			o.err = errors.New("domain second connection")
			return
		}
		payload := b[int(b[12]>>4)*4:]
		if !d.tls && len(payload) > 0 {
			end := uint64(uint32(seq-d.clientStart)) + uint64(len(payload))
			if end > 16384 || (d.headerBytes != 0 && end > uint64(d.headerBytes)) {
				o.err = errors.New("domain extra request")
				return
			}
			if uint32(end) > d.inputEnd {
				d.inputEnd = uint32(end)
			}
		}
	} else if !d.haveSYN || source != d.port() || dest != o.clientPort {
		o.err = errors.New("domain reply tuple")
		return
	}
	if (incoming && o.rx >= 1024) || (!incoming && o.tx >= 1024) {
		o.err = errors.New("domain packet limit")
		return
	}
	// HTTP 的既有覆盖算法使用相同的18080端口。RST仅结束连接，不能自行满足ACK。
	if !d.tls && flags&4 == 0 {
		o.tcp(p, incoming)
	} else if incoming {
		o.rx++
	} else {
		o.tx++
	}
}

type domainResult struct {
	bytes int
	err   error
}

func serveDomain(ctx context.Context, p *livePeer) (result domainResult) {
	c, err := p.listener.Accept()
	if err != nil {
		return domainResult{err: err}
	}
	stop := context.AfterFunc(ctx, func() { c.Close() })
	defer stop()
	defer func() { result.err = errors.Join(result.err, c.Close()) }()
	// 从首连接开始即观察额外连接，成功事件后直到关闭仍保留该观察。
	p.workers.Add(1)
	go func() {
		defer p.workers.Done()
		extra, err := p.listener.Accept()
		if err == nil {
			closeErr := extra.Close()
			p.watch.fail(errors.Join(errors.New("domain extra connection"), closeErr))
		} else if ctx.Err() == nil {
			p.watch.fail(errors.New("domain accept error"))
		}
	}()
	if p.watch.domain.tls {
		result.bytes, result.err = observeDomainTLS(ctx, c)
	} else {
		result.err = serveDomainHTTP(ctx, c, p.watch)
	}
	return result
}

func serveDomainHTTP(ctx context.Context, c net.Conn, watch *observer) error {
	if err := ctx.Err(); err != nil {
		return err
	}
	if err := c.SetReadDeadline(deadline(ctx)); err != nil {
		return err
	}
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
	if !strings.HasSuffix(header.String(), "\r\n\r\n") {
		return errors.New("domain header limit")
	}
	req, err := http.ReadRequest(bufio.NewReader(strings.NewReader(header.String())))
	if err != nil {
		return err
	}
	defer req.Body.Close()
	if req.Method != "HEAD" || req.RequestURI != "/task009-wg-domain" || req.Host != domainHost+":18080" || req.ContentLength > 0 || len(req.TransferEncoding) != 0 || r.Buffered() != 0 {
		return errors.New("domain HTTP request")
	}
	watch.mu.Lock()
	watch.domain.headerBytes = uint32(header.Len())
	if watch.domain.inputEnd > watch.domain.headerBytes {
		watch.err = errors.New("domain extra request")
	}
	err = watch.err
	watch.mu.Unlock()
	if err != nil {
		return err
	}
	if err := c.SetWriteDeadline(deadline(ctx)); err != nil {
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

var errDomainSNIObserved = errors.New("domain SNI observed")

// tls.Server忽略回调失败路径的alert写错误；此包装独立锁存该错误，不能凭哨兵判成功。
// 握手在单一goroutine执行；取消只关闭底层连接，不读取或修改计数。
type domainTLSConn struct {
	net.Conn
	ctx                   context.Context
	readBytes, writeBytes int
	failure               error
	lastDeadline          time.Time
}

func (c *domainTLSConn) latch(err error) error {
	if c.failure == nil {
		c.failure = err
	}
	return c.failure
}

func (c *domainTLSConn) Read(b []byte) (int, error) {
	if c.failure != nil {
		return 0, c.failure
	}
	if err := c.ctx.Err(); err != nil {
		return 0, c.latch(err)
	}
	remaining := 16384 - c.readBytes
	if remaining == 0 {
		return 0, c.latch(errors.New("domain TLS read limit"))
	}
	if len(b) > remaining {
		b = b[:remaining]
	}
	c.lastDeadline = deadline(c.ctx)
	if err := c.Conn.SetReadDeadline(c.lastDeadline); err != nil {
		return 0, c.latch(err)
	}
	n, err := c.Conn.Read(b)
	c.readBytes += n
	if err != nil {
		c.latch(err)
	}
	if !time.Now().Before(c.lastDeadline) {
		c.latch(context.DeadlineExceeded)
	}
	return n, c.failure
}

func (c *domainTLSConn) Write(b []byte) (int, error) {
	if c.failure != nil {
		return 0, c.failure
	}
	if err := c.ctx.Err(); err != nil {
		return 0, c.latch(err)
	}
	if len(b) > 4096-c.writeBytes {
		return 0, c.latch(errors.New("domain TLS write limit"))
	}
	c.lastDeadline = deadline(c.ctx)
	if err := c.Conn.SetWriteDeadline(c.lastDeadline); err != nil {
		return 0, c.latch(err)
	}
	n, err := c.Conn.Write(b)
	c.writeBytes += n
	if err != nil {
		c.latch(err)
	} else if n != len(b) {
		c.latch(io.ErrShortWrite)
	}
	if !time.Now().Before(c.lastDeadline) {
		c.latch(context.DeadlineExceeded)
	}
	return n, c.failure
}

func observeDomainTLS(ctx context.Context, conn net.Conn) (int, error) {
	bounded := &domainTLSConn{Conn: conn, ctx: ctx}
	matched := false
	server := tls.Server(bounded, &tls.Config{GetConfigForClient: func(info *tls.ClientHelloInfo) (*tls.Config, error) {
		if info.ServerName != domainHost {
			return nil, errors.New("domain SNI mismatch")
		}
		matched = true
		return nil, errDomainSNIObserved
	}})
	err := server.HandshakeContext(ctx)
	if !matched || !errors.Is(err, errDomainSNIObserved) {
		return bounded.readBytes, errors.New("domain TLS handshake")
	}
	if bounded.failure != nil {
		return bounded.readBytes, bounded.failure
	}
	if ctx.Err() != nil || !time.Now().Before(bounded.lastDeadline) {
		return bounded.readBytes, context.DeadlineExceeded
	}
	return bounded.readBytes, nil
}
