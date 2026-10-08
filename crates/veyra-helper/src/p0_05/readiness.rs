//! child 启动日志契约：两个 pipe 各自保持字节和行边界，不扫描监听端口。
use super::*;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

const STREAM_BUDGET: usize = 32768; // 两流合计最多 65536 bytes，沿用原总预算。
const TEXT_JSON_BUDGET: usize = 768; // 包含 JSON 转义，给既有 4096-byte IPC 留出空间。
const STAGE: &str = "StartSystemProxyTest.child_readiness";

#[derive(Default)]
struct StreamOutput {
    bytes: Vec<u8>,
    parsed: usize,
    eof: bool,
    overflow: bool,
}
#[derive(Default)]
struct Output {
    stdout: StreamOutput,
    stderr: StreamOutput,
    mixed_port: Option<u16>,
    controller_port: Option<u16>,
    exit: Option<ExitStatus>,
}
struct Failure {
    outcome: &'static str,
    detail: String,
}
impl Failure {
    fn new(outcome: &'static str, detail: impl Into<String>) -> Self {
        Self {
            outcome,
            detail: detail.into(),
        }
    }
}

#[derive(Debug)]
pub(super) struct ReadinessError {
    pub(super) diagnostics: Value,
    pub(super) detail: String,
}
impl std::fmt::Display for ReadinessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            self.diagnostics["stage"].as_str().unwrap(),
            self.detail
        )
    }
}
impl std::error::Error for ReadinessError {}

// 同 manual 的固定关键词/loopback 地址契约；只接受紧随关键词的非零 u16 端口。
fn parse_line(line: &[u8], mixed: &mut Option<u16>, controller: &mut Option<u16>) {
    let Ok(line) = std::str::from_utf8(line) else {
        return;
    };
    for (marker, port) in [
        (
            "inbound/mixed[p005-mixed]: tcp server started at 127.0.0.1:",
            mixed,
        ),
        ("clash-api: restful api listening at 127.0.0.1:", controller),
    ] {
        if let Some((_, address)) = line.rsplit_once(marker) {
            let address = address.trim_end_matches('\r');
            if address.starts_with(['1', '2', '3', '4', '5', '6', '7', '8', '9'])
                && address.bytes().all(|b| b.is_ascii_digit())
                && let Ok(value) = address.parse::<u16>()
            {
                *port = Some(value);
            }
        }
    }
}
impl StreamOutput {
    fn parse(&mut self, mixed: &mut Option<u16>, controller: &mut Option<u16>) {
        while let Some(end) = self.bytes[self.parsed..].iter().position(|b| *b == b'\n') {
            let end = self.parsed + end;
            parse_line(&self.bytes[self.parsed..end], mixed, controller);
            self.parsed = end + 1;
        }
        // EOF 是最后一个无 newline 行的确定边界；未完成的 chunk 不能成为端口事实。
        if self.eof && !self.overflow && self.parsed < self.bytes.len() {
            parse_line(&self.bytes[self.parsed..], mixed, controller);
            self.parsed = self.bytes.len();
        }
    }
    fn drain(&mut self, pipe: &mut dyn Read, name: &str) -> std::result::Result<(), String> {
        if self.eof {
            return Ok(());
        }
        let mut bytes = [0; 2048];
        loop {
            match pipe.read(&mut bytes) {
                Ok(0) => {
                    self.eof = true;
                    return Ok(());
                }
                Ok(n) => {
                    let keep = n.min(STREAM_BUDGET - self.bytes.len());
                    self.bytes.extend_from_slice(&bytes[..keep]);
                    if keep < n {
                        self.overflow = true;
                        return Err(format!("{name}: child startup output exceeded budget"));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
                // EINTR 后重试这根 pipe，尤其不能在已退出 child 的最终 drain 中丢掉尾部。
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(format!("{name}.read: {error}")),
            }
        }
    }
    fn evidence(&self, secret: &str) -> Value {
        // 在截断/JSON 转义之前固定替换完整 secret，以及末尾可能尚未读全的 secret 前缀。
        let mut text = String::from_utf8_lossy(&self.bytes).replace(secret, "<secret-redacted>");
        for n in (1..secret.len()).rev() {
            if text.ends_with(&secret[..n]) {
                text.truncate(text.len() - n);
                text.push_str("<secret-redacted>");
                break;
            }
        }
        let mut encoded = 2; // JSON 引号
        let end = text
            .char_indices()
            .find_map(|(index, c)| {
                encoded += match c {
                    '"' | '\\' | '\n' | '\r' | '\t' | '\u{08}' | '\u{0c}' => 2,
                    c if c < '\u{20}' => 6,
                    c => c.len_utf8(),
                };
                (encoded > TEXT_JSON_BUDGET).then_some(index)
            })
            .unwrap_or(text.len());
        let truncated = self.overflow || end < text.len();
        text.truncate(end);
        json!({"text":text,"eof":self.eof,"captured_bytes":self.bytes.len(),"truncated":truncated,"overflow":self.overflow})
    }
}
impl Output {
    fn drain(
        &mut self,
        stdout: &mut dyn Read,
        stderr: &mut dyn Read,
    ) -> std::result::Result<(), String> {
        // 即使一个 pipe 报错，也读取另一个 pipe 的末尾证据；永不跨流拼接。
        let out = self.stdout.drain(stdout, "stdout");
        let err = self.stderr.drain(stderr, "stderr");
        self.stdout
            .parse(&mut self.mixed_port, &mut self.controller_port);
        self.stderr
            .parse(&mut self.mixed_port, &mut self.controller_port);
        match (out, err) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
            (Err(a), Err(b)) => Err(format!("{a}; {b}")),
        }
    }
    fn evidence(
        &self,
        child_pid: u32,
        identity: &Value,
        secret: &str,
        started: Instant,
        outcome: &str,
    ) -> Value {
        let stage = if outcome == "child_exited" {
            format!("{STAGE}.child_exited")
        } else {
            STAGE.to_owned()
        };
        json!({"stage":stage,"outcome":outcome,"child_pid":child_pid,"child_identity":identity,
            "stdout":self.stdout.evidence(secret),"stderr":self.stderr.evidence(secret),
            "mixed_port":self.mixed_port,"controller_port":self.controller_port,"elapsed_ms":started.elapsed().as_millis(),
            "exit_status":self.exit.map(|s| json!({"display":s.to_string(),"raw":s.into_raw(),"code":s.code(),"signal":s.signal(),"success":s.success()}))})
    }
}

// 此循环只依赖 pipe 和真实 child.try_wait；纯测试注入退出结果，不启动任何 child。
fn collect(
    stdout: &mut dyn Read,
    stderr: &mut dyn Read,
    mut try_wait: impl FnMut() -> std::io::Result<Option<ExitStatus>>,
    output: &mut Output,
    budget: Duration,
) -> std::result::Result<(u16, u16), Failure> {
    let deadline = Instant::now() + budget;
    loop {
        output.exit =
            try_wait().map_err(|e| Failure::new("wait_error", format!("child.try_wait: {e}")))?;
        let mut drained = output.drain(stdout, stderr);
        if output.exit.is_none() {
            output.exit = try_wait()
                .map_err(|e| Failure::new("wait_error", format!("child.try_wait: {e}")))?;
            if output.exit.is_some() && drained.is_ok() {
                drained = output.drain(stdout, stderr);
            }
        }
        // 检测退出后仍先 drain，再判失败；末尾完整日志和无 newline 行均保留。
        if let Some(exit) = output.exit {
            let mut detail = format!("child exited before readiness: {exit}; no proxy write");
            if let Err(e) = drained {
                detail.push_str(&format!("; drain: {e}"));
            }
            return Err(Failure::new("child_exited", detail));
        }
        drained.map_err(|e| Failure::new("pipe_error", e))?;
        if Instant::now() >= deadline {
            return Err(Failure::new(
                "timeout",
                "child startup timeout; no proxy write",
            ));
        }
        if let (Some(mixed), Some(controller)) = (output.mixed_port, output.controller_port) {
            return Ok((mixed, controller));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn wait(child: &mut Child, identity: &Value, secret: &str) -> Result<(u16, u16, Value)> {
    let started = Instant::now();
    let mut output = Output::default();
    let result = (|| -> std::result::Result<(u16, u16), Failure> {
        // 移出 pipe 后 child 可在每轮 try_wait；句柄在返回后放回，保持后续资源归属。
        for (name, fd) in [
            ("stdout", child.stdout.as_ref().unwrap().as_raw_fd()),
            ("stderr", child.stderr.as_ref().unwrap().as_raw_fd()),
        ] {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
            {
                return Err(Failure::new(
                    "pipe_error",
                    format!("{name}.nonblocking: {}", std::io::Error::last_os_error()),
                ));
            }
        }
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let ports = collect(
            &mut stdout,
            &mut stderr,
            || child.try_wait(),
            &mut output,
            Duration::from_secs(10),
        );
        child.stdout = Some(stdout);
        child.stderr = Some(stderr);
        let (port, controller) = ports?;
        // 固定鉴权契约保持不变；不保存请求、secret 或 HTTP response 原文。
        let authenticated = (|| -> Result<()> {
            let mut stream = std::net::TcpStream::connect_timeout(
                &format!("127.0.0.1:{controller}").parse()?,
                Duration::from_secs(1),
            )?;
            stream.set_read_timeout(Some(Duration::from_secs(2)))?;
            write!(
                stream,
                "GET /version HTTP/1.0\r\nHost: localhost\r\nAuthorization: Bearer {secret}\r\n\r\n"
            )?;
            let mut response = String::new();
            stream.take(4096).read_to_string(&mut response)?;
            if !response.starts_with("HTTP/1.0 200") && !response.starts_with("HTTP/1.1 200") {
                return Err("controller authentication/readiness failed".into());
            }
            Ok(())
        })();
        // 端口解析后/鉴权过程中退出也必须优先报告真实退出状态和最后输出。
        output.exit = child
            .try_wait()
            .map_err(|e| Failure::new("wait_error", format!("child.try_wait: {e}")))?;
        let drained = output.drain(
            child.stdout.as_mut().unwrap(),
            child.stderr.as_mut().unwrap(),
        );
        if let Some(exit) = output.exit {
            return Err(Failure::new(
                "child_exited",
                format!("child exited before readiness: {exit}; drain={drained:?}; no proxy write"),
            ));
        }
        drained.map_err(|e| Failure::new("pipe_error", e))?;
        authenticated.map_err(|e| Failure::new("controller_error", e.to_string()))?;
        Ok((port, controller))
    })();
    let outcome = match &result {
        Ok(_) => "ready",
        Err(e) => e.outcome,
    };
    let evidence = output.evidence(child.id(), identity, secret, started, outcome);
    match result {
        Ok((mixed, controller)) => Ok((mixed, controller, evidence)),
        Err(e) => Err(Box::new(ReadinessError {
            diagnostics: evidence,
            detail: e.detail,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io::{Cursor, Error, ErrorKind};

    const MIXED: &str = "INFO inbound/mixed[p005-mixed]: tcp server started at 127.0.0.1:12345\n";
    const CONTROLLER: &str = "INFO clash-api: restful api listening at 127.0.0.1:23456\n";
    const SECRET: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    struct Pipe {
        steps: VecDeque<std::io::Result<Vec<u8>>>,
        reads: usize,
    }
    impl Pipe {
        fn new(steps: Vec<std::io::Result<Vec<u8>>>) -> Self {
            Self {
                steps: steps.into(),
                reads: 0,
            }
        }
    }
    impl Read for Pipe {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            match self.steps.pop_front().unwrap_or_else(block) {
                Ok(bytes) => {
                    let n = bytes.len().min(out.len());
                    out[..n].copy_from_slice(&bytes[..n]);
                    if n < bytes.len() {
                        self.steps.push_front(Ok(bytes[n..].to_vec()));
                    }
                    Ok(n)
                }
                Err(e) => Err(e),
            }
        }
    }
    fn bytes(text: &str) -> std::io::Result<Vec<u8>> {
        Ok(text.as_bytes().to_vec())
    }
    fn block() -> std::io::Result<Vec<u8>> {
        Err(ErrorKind::WouldBlock.into())
    }
    fn identity() -> Value {
        json!({"pid":42,"uid":501,"ruid":501,"gid":20,"rgid":20,"pgid":42,"start_sec":100,"start_usec":200})
    }

    // 保护日志端口契约：交错 partial chunks 各自拼回原行，不能拼出另一个流的伪端口。
    #[test]
    fn interleaved_partial_chunks_preserve_stream_lines_and_split_ports_both_ways() {
        for reversed in [false, true] {
            let mut mixed = Pipe::new(vec![
                bytes(MIXED.trim_end_matches("12345\n")),
                block(),
                bytes("12345\n"),
                block(),
            ]);
            let mut controller = Pipe::new(vec![
                bytes("45678\n"),
                bytes(&CONTROLLER[..19]),
                block(),
                bytes(&CONTROLLER[19..]),
                block(),
            ]);
            let mut output = Output::default();
            let (stdout, stderr) = if reversed {
                (&mut controller, &mut mixed)
            } else {
                (&mut mixed, &mut controller)
            };
            output.drain(stdout, stderr).unwrap();
            assert_eq!(output.mixed_port, None);
            assert_eq!(output.controller_port, None);
            output.drain(stdout, stderr).unwrap();
            assert_eq!(
                (output.mixed_port, output.controller_port),
                (Some(12345), Some(23456))
            );
            let mixed_bytes = if reversed {
                &output.stderr.bytes
            } else {
                &output.stdout.bytes
            };
            assert_eq!(mixed_bytes, MIXED.as_bytes());
            let controller_bytes = if reversed {
                &output.stdout.bytes
            } else {
                &output.stderr.bytes
            };
            assert_eq!(controller_bytes, format!("45678\n{CONTROLLER}").as_bytes());
        }
    }

    // 保护成功诊断：两流独立提供端口，collect 只解析日志；此测试不发起鉴权网络请求。
    #[test]
    fn recognized_ports_and_ready_evidence_survive_both_stream_assignments() {
        for reversed in [false, true] {
            let mut out = Cursor::new(if reversed { CONTROLLER } else { MIXED });
            let mut err = Cursor::new(if reversed { MIXED } else { CONTROLLER });
            let mut output = Output::default();
            assert_eq!(
                collect(
                    &mut out,
                    &mut err,
                    || Ok(None),
                    &mut output,
                    Duration::from_secs(10)
                )
                .ok(),
                Some((12345, 23456))
            );
            let evidence = output.evidence(42, &identity(), SECRET, Instant::now(), "ready");
            assert_eq!(evidence["outcome"], "ready");
            assert_eq!(evidence["child_identity"], identity());
            assert_eq!(evidence["stdout"]["eof"], true);
            assert_eq!(evidence["stderr"]["eof"], true);
            assert_eq!(evidence["controller_port"], 23456);
        }
    }

    // 保护提前退出定位与最后日志：纯注入 ExitStatus，不启动 child、不等待 10s。
    #[test]
    fn early_exit_drains_last_output_and_retains_code_signal_and_identity() {
        for raw in [7 << 8, libc::SIGTERM] {
            let mut out = Cursor::new(MIXED);
            let mut err = Cursor::new("fatal: last message without newline");
            let mut output = Output::default();
            let started = Instant::now();
            let failure = collect(
                &mut out,
                &mut err,
                || Ok(Some(ExitStatus::from_raw(raw))),
                &mut output,
                Duration::from_secs(10),
            )
            .err()
            .unwrap();
            assert!(started.elapsed() < Duration::from_secs(1));
            assert_eq!(failure.outcome, "child_exited");
            let e = output.evidence(42, &identity(), SECRET, started, failure.outcome);
            assert_eq!(e["stage"], format!("{STAGE}.child_exited"));
            assert_eq!(e["child_pid"], 42);
            assert_eq!(e["child_identity"], identity());
            assert_eq!(e["mixed_port"], 12345);
            assert!(e["controller_port"].is_null());
            assert_eq!(e["stderr"]["text"], "fatal: last message without newline");
            assert_eq!(e["exit_status"]["raw"], raw);
            assert_eq!(
                e["exit_status"]["code"],
                if raw == 7 << 8 { json!(7) } else { Value::Null }
            );
            assert_eq!(
                e["exit_status"]["signal"],
                if raw == libc::SIGTERM {
                    json!(libc::SIGTERM)
                } else {
                    Value::Null
                }
            );
        }
    }

    // 保护 try_wait 与读取之间的退出竞态：第二次检查见 exit 后再 drain 两根 pipe。
    #[test]
    fn exit_during_read_drains_after_final_exit_check() {
        let mut out = Pipe::new(vec![block(), bytes(MIXED), bytes("")]);
        let mut err = Pipe::new(vec![block(), bytes("last error"), bytes("")]);
        let mut checks = 0;
        let mut output = Output::default();
        let failure = collect(
            &mut out,
            &mut err,
            || {
                checks += 1;
                Ok((checks == 2).then(|| ExitStatus::from_raw(9 << 8)))
            },
            &mut output,
            Duration::from_secs(10),
        )
        .err()
        .unwrap();
        assert_eq!(failure.outcome, "child_exited");
        assert_eq!(output.stderr.bytes, b"last error");
        assert_eq!(output.mixed_port, Some(12345));
        assert!(output.stdout.eof && output.stderr.eof);
    }

    // 保护单流 EOF 不截断另一根 pipe 的后续日志，EOF 的无 newline 最后一行也有效。
    #[test]
    fn one_stream_eof_does_not_stop_the_other_stream() {
        let mut out = Pipe::new(vec![bytes(MIXED.trim_end()), bytes("")]);
        let mut err = Pipe::new(vec![block(), bytes(CONTROLLER), block()]);
        let mut output = Output::default();
        output.drain(&mut out, &mut err).unwrap();
        assert!(output.stdout.eof && !output.stderr.eof);
        let out_reads = out.reads;
        output.drain(&mut out, &mut err).unwrap();
        assert_eq!(out.reads, out_reads);
        assert_eq!(
            (output.mixed_port, output.controller_port),
            (Some(12345), Some(23456))
        );
    }

    // 保护非阻塞读分类：WouldBlock/EINTR 不当作 EOF，真实错误明确指向 stream。
    #[test]
    fn transient_reads_continue_and_other_errors_name_the_stream() {
        let mut out = Pipe::new(vec![
            Err(Error::from_raw_os_error(libc::EINTR)),
            block(),
            block(),
            bytes(MIXED),
            bytes(""),
        ]);
        let mut err = Pipe::new(vec![block(), block(), bytes(CONTROLLER), bytes("")]);
        let mut output = Output::default();
        for _ in 0..2 {
            output.drain(&mut out, &mut err).unwrap();
            assert!(!output.stdout.eof && !output.stderr.eof);
        }
        output.drain(&mut out, &mut err).unwrap();
        assert_eq!(output.mixed_port, Some(12345));
        for stream in ["stdout", "stderr"] {
            let mut bad = Pipe::new(vec![Err(Error::from_raw_os_error(libc::EIO))]);
            let mut good = Cursor::new("other stream tail");
            let mut output = Output::default();
            let error = if stream == "stdout" {
                output.drain(&mut bad, &mut good)
            } else {
                output.drain(&mut good, &mut bad)
            }
            .unwrap_err();
            assert!(error.contains(&format!("{stream}.read:")));
            let good_output = if stream == "stdout" {
                output.stderr
            } else {
                output.stdout
            };
            assert_eq!(good_output.bytes, b"other stream tail");
        }
    }

    // 保护诊断内存/IPC 预算：任意两流输出最多 64KiB，转义和无效 UTF-8 不突破文本上限。
    #[test]
    fn output_and_json_diagnostics_have_bounded_budgets() {
        let mut output = Output::default();
        let mut out = Cursor::new(vec![0; STREAM_BUDGET + 2048]);
        let mut err = Cursor::new(vec![255; STREAM_BUDGET + 2048]);
        let error = output.drain(&mut out, &mut err).unwrap_err();
        assert!(error.contains("stdout") && error.contains("stderr"));
        assert_eq!(output.stdout.bytes.len(), STREAM_BUDGET);
        assert_eq!(output.stderr.bytes.len(), STREAM_BUDGET);
        let e = output.evidence(42, &identity(), SECRET, Instant::now(), "pipe_error");
        for stream in ["stdout", "stderr"] {
            assert!(serde_json::to_vec(&e[stream]["text"]).unwrap().len() <= TEXT_JSON_BUDGET);
            assert_eq!(e[stream]["overflow"], true);
            assert_eq!(e[stream]["truncated"], true);
        }
        // 结构化诊断之外仍为 peer/status/instance/last 或错误及 cleanup 留至少 1KiB。
        assert!(serde_json::to_vec(&e).unwrap().len() + 1024 < MAX);
    }

    // 保护 secret 不出现在错误/evidence：完整、分块、末尾 partial 和截断位置均先替换。
    #[test]
    fn secret_is_redacted_before_export_and_text_truncation() {
        for text in [
            format!("fatal secret={SECRET}\n"),
            format!("fatal secret={}", &SECRET[..32]),
            format!("{}secret={SECRET}\n", "x".repeat(740)),
        ] {
            let mut out = Pipe::new(vec![
                bytes(&text[..7]),
                block(),
                bytes(&text[7..]),
                bytes(""),
            ]);
            let mut err = Cursor::new("");
            let mut output = Output::default();
            output.drain(&mut out, &mut err).unwrap();
            output.drain(&mut out, &mut err).unwrap();
            let e = output.evidence(42, &identity(), SECRET, Instant::now(), "timeout");
            let serialized = e.to_string();
            assert!(!serialized.contains(SECRET));
            assert!(!serialized.contains(&SECRET[..16]));
        }
    }

    // 保护 timeout 与原始 UTF-8 分块证据，不将活着但无端口的 child 误报为 exited。
    #[test]
    fn timeout_keeps_independent_output_and_unknown_exit_state() {
        let mut out = Pipe::new(vec![Ok(vec![0xe4]), Ok(vec![0xb8, 0xad]), block()]);
        let mut err = Pipe::new(vec![bytes("stderr pending"), block()]);
        let mut output = Output::default();
        let failure = collect(&mut out, &mut err, || Ok(None), &mut output, Duration::ZERO)
            .err()
            .unwrap();
        let e = output.evidence(42, &identity(), SECRET, Instant::now(), failure.outcome);
        assert_eq!(e["outcome"], "timeout");
        assert_eq!(e["stdout"]["text"], "中");
        assert_eq!(e["stderr"]["text"], "stderr pending");
        assert!(e["exit_status"].is_null());
        assert!(!output.stdout.eof && !output.stderr.eof);
    }

    // 保护精确日志语义：错误行、伪拼接地址、零/越界/部分数字不能成为监听端口。
    #[test]
    fn parser_rejects_wrong_addresses_and_incomplete_or_forged_ports() {
        for line in [
            "inbound/mixed[p005-mixed]: tcp server started at 127.0.0.1:0",
            "inbound/mixed[p005-mixed]: tcp server started at 127.0.0.1:65536",
            "inbound/mixed[p005-mixed]: tcp server started at 127.0.0.1:12345extra",
            "inbound/mixed[p005-mixed]: tcp server started at 127.0.0.1:12345 clash-api: restful api listening at 127.0.0.1:23456extra",
            "inbound/mixed[p005-mixed]: tcp server started at 127.0.0.1:broken other 127.0.0.1:23456",
            "inbound/mixed[p005-other]: tcp server started at 127.0.0.1:12345",
            "clash-api: restful api listening at 0.0.0.0:23456",
            "clash-api: restful api listening at 127.0.0.1:+23456",
            "clash-api: restful api listening at 127.0.0.1:023456",
        ] {
            let (mut mixed, mut controller) = (None, None);
            parse_line(line.as_bytes(), &mut mixed, &mut controller);
            assert_eq!((mixed, controller), (None, None), "{line}");
        }
        let mut out = Pipe::new(vec![
            bytes(MIXED.trim_end()),
            block(),
            bytes("invalid suffix\n"),
            bytes(""),
        ]);
        let mut err = Cursor::new("");
        let mut output = Output::default();
        output.drain(&mut out, &mut err).unwrap();
        assert_eq!(output.mixed_port, None);
        output.drain(&mut out, &mut err).unwrap();
        assert_eq!(output.mixed_port, None);
    }
}
