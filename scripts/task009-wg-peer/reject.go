package main

import (
	"context"
	"io"
	"strconv"
	"time"
)

type rejectWork struct {
	phase int
	kind  string
	cases []localCaseResult
	err   error
}

func runReject(initial command, born time.Time, inputs <-chan inputResult, out io.Writer, done <-chan struct{}) int {
	hardDeadline := born.Add(150 * time.Second)
	hard := time.NewTimer(time.Until(hardDeadline))
	defer hard.Stop()
	output := boundedOutput(out, done, hardDeadline)
	base := func(kind string) event { return event{"v": 1, "event": kind, "run_id": initial.RunID} }
	global, cancelGlobal := context.WithDeadline(context.Background(), born.Add(135*time.Second))
	defer cancelGlobal()
	cancelPhase := func() {}
	failed := false
	failure := func(stage, code string) {
		if !failed {
			failed = true
			cancelPhase()
			e := base("failed")
			e["stage"] = stage
			e["code"] = code
			_ = output(e)
		}
	}
	selfCtx, selfCancel := context.WithDeadline(global, born.Add(10*time.Second))
	testCtx, testCancel := context.WithTimeout(selfCtx, 5*time.Second)
	selfErr := selftest(testCtx)
	testCancel()
	selfCancel()
	if selfErr != nil {
		failure("selftest", "io_error")
		return 1
	}
	p, peerPublic, dutPublic, err := newLive(initial)
	if err != nil {
		failure("bind", "resource_error")
		return 1
	}
	for _, port := range p.watch.reject.ports {
		if p.bind.port == port {
			p.close()
			failure("bind", "resource_error")
			return 1
		}
	}
	ready := base("ready")
	ready["udp_port"] = p.bind.port
	ready["peer_public_key"] = peerPublic
	ready["dut_public_key"] = dutPublic
	ready["selftest"] = map[string]bool{"tcp": true, "udp": true, "icmp": true}
	if output(ready) != nil {
		p.close()
		return 1
	}
	completed, phase := 0, 0
	var phaseCtx context.Context
	var phaseDone <-chan struct{}
	globalDone := global.Done()
	bootstrapped, probed, probing := false, false, false
	work := make(chan rejectWork, 1)
	emitPhase := func(kind string) bool {
		e := base(kind)
		e["phase"] = phase
		if output(e) != nil {
			failure("protocol", "io_error")
			return false
		}
		return true
	}
	for {
		select {
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
				cancelPhase()
				cancelGlobal()
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
				if output(e) != nil {
					return 1
				}
				if failed || completed != 3 {
					return 1
				}
				return 0
			}
			if failed {
				continue
			}
			if global.Err() != nil || (phase != completed && phaseCtx.Err() != nil) {
				failure("deadline", "timeout")
				continue
			}
			switch c.Op {
			case "begin_phase":
				if phase != completed || completed >= 3 || *c.Phase != completed+1 {
					failure("protocol", "invalid_input")
					continue
				}
				if err := p.watch.beginRejectPhase(*c.Phase); err != nil {
					failure("protocol", "invalid_input")
					continue
				}
				phase = *c.Phase
				phaseCtx, cancelPhase = context.WithTimeout(global, 40*time.Second)
				defer cancelPhase()
				phaseDone = phaseCtx.Done()
				bootstrapped, probed, probing = false, false, false
				p.watch.mu.Lock()
				bootstrap := p.watch.reject.bootstrap
				p.watch.mu.Unlock()
				if !emitPhase("phase_ready") {
					continue
				}
				currentPhase, currentCtx := phase, phaseCtx
				p.workers.Add(1)
				go func() {
					defer p.workers.Done()
					err := serveHTTPURI(currentCtx, p, "/task009-wg?token="+initial.Token+"&phase="+strconv.Itoa(currentPhase), bootstrap)
					work <- rejectWork{phase: currentPhase, kind: "bootstrap", err: err}
				}()
			case "probe_local":
				if phase == completed || !bootstrapped || probed || probing {
					failure("protocol", "invalid_input")
					continue
				}
				probing = true
				currentPhase, currentCtx := phase, phaseCtx
				p.workers.Add(1)
				go func() {
					defer p.workers.Done()
					cases, err := probeLocal(currentCtx, p)
					work <- rejectWork{phase: currentPhase, kind: "local_probe", cases: cases, err: err}
				}()
			case "finish_phase":
				if phase == completed || !bootstrapped || !probed || probing {
					failure("protocol", "invalid_input")
					continue
				}
				cancelPhase()
				phaseDone = nil
				p.watch.mu.Lock()
				p.watch.reject.active = false
				p.watch.reject.health.mu.Lock()
				p.watch.reject.health.icmpActive = false
				p.watch.reject.health.mu.Unlock()
				p.watch.mu.Unlock()
				completed = phase
				emitPhase("phase_stopped")
			default:
				failure("protocol", "invalid_input")
			}
		case result := <-work:
			if failed {
				continue
			}
			if result.phase != phase || phase == completed {
				failure("protocol", "invalid_input")
				continue
			}
			if phaseCtx.Err() != nil {
				failure("deadline", "timeout")
				continue
			}
			if result.err != nil {
				failure("protocol", "unexpected_packet")
				continue
			}
			_, _, err := p.watch.result()
			if err != nil {
				failure("protocol", "unexpected_packet")
				continue
			}
			e := base(result.kind)
			e["phase"] = phase
			if result.kind == "bootstrap" {
				if bootstrapped {
					failure("protocol", "invalid_input")
					continue
				}
				p.watch.mu.Lock()
				bootstrap := p.watch.reject.bootstrap
				p.watch.mu.Unlock()
				rx, tx, err := bootstrap.result()
				if err != nil || rx == 0 || tx == 0 {
					failure("protocol", "unexpected_packet")
					continue
				}
				bootstrapped = true
				e["requests"] = 1
				e["response_status"] = 204
				e["rx_tcp_packets"] = rx
				e["tx_tcp_packets"] = tx
				e["authenticated"] = true
				e["response_acked"] = true
			} else {
				if !probing || probed {
					failure("protocol", "invalid_input")
					continue
				}
				probing = false
				probed = true
				e["cases"] = result.cases
				e["icmp"] = event{"sent": 3, "received": 3, "id": 9, "sequences": []int{1, 2, 3}, "payloads_valid": true, "addresses_valid": true}
			}
			if output(e) != nil {
				failure("protocol", "io_error")
			}
		case <-p.watch.wake:
			_, _, err := p.watch.result()
			if err != nil {
				failure("protocol", "unexpected_packet")
			}
		case <-phaseDone:
			failure("deadline", "timeout")
			phaseDone = nil
		case <-globalDone:
			failure("deadline", "timeout")
			globalDone = nil
		case <-hard.C:
			failure("deadline", "timeout")
			return 1 // main 的硬退出释放自有资源，父应已停止 DUT。
		}
	}
}
