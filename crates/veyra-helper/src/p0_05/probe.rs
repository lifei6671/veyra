//! Fixed root test harness, invoked once via the standard macOS authorization UI.
//! No arbitrary paths/commands/PIDs are accepted by this harness or daily IPC.
use super::*;

pub fn reject_probe() -> Result<Value> {
    let before = request(Operation::Status)?;
    let mut results = Vec::new();
    for payload in [
        r#"{"command":"Status","pid":1}"#.to_owned(),
        r#"{"command":"StartSystemProxyTest","path":"/bin/sh"}"#.to_owned(),
        r#"{"command":"RunShell"}"#.to_owned(),
        "x".repeat(MAX + 1),
    ] {
        let mut stream = connect()?;
        writeln!(stream, "{payload}")?;
        let response: Value =
            serde_json::from_slice(&read_frame(&mut stream, Duration::from_secs(2))?)?;
        if response["ok"] != false {
            return Err("invalid request was accepted".into());
        }
        results.push(response);
    }
    let after = request(Operation::Status)?;
    if before["response"]["result"]["status"] != after["response"]["result"]["status"] {
        return Err("invalid request changed state".into());
    }
    Ok(json!({"rejections":results,"no_side_effect":true}))
}
fn client(uid: u32, gid: u32, operation: &str) -> Command {
    let mut command = Command::new(HELPER);
    command.arg(operation);
    unsafe {
        command.pre_exec(move || drop_credentials(uid, gid));
    }
    command
}
fn capture(mut command: Command, budget: Duration) -> Result<Value> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| spawn_error("harness.client_spawn", e))?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    for fd in [
        child.stdout.as_ref().unwrap().as_raw_fd(),
        child.stderr.as_ref().unwrap().as_raw_fd(),
    ] {
        unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };
    }
    let deadline = Instant::now() + budget;
    let exit = loop {
        let mut bytes = [0; 8192];
        for (pipe, buffer) in [
            (
                &mut *child.stdout.as_mut().unwrap() as &mut dyn Read,
                &mut stdout,
            ),
            (
                &mut *child.stderr.as_mut().unwrap() as &mut dyn Read,
                &mut stderr,
            ),
        ] {
            loop {
                match pipe.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(n) => buffer.extend_from_slice(&bytes[..n]),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) => return Err(e.into()),
                }
            }
        }
        if stdout.len() + stderr.len() > 1_000_000 {
            child.kill()?;
            child.wait()?;
            return Err("fixed probe output too large".into());
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() > deadline {
            child.kill()?;
            child.wait()?;
            // 超时也保留已经 flush 的 cycle 分步输出，不能再次退化为 stdout 空。
            return Ok(
                json!({"code":null,"stdout":String::from_utf8_lossy(&stdout),
                "stderr":String::from_utf8_lossy(&stderr),
                "capture_error":"fixed client operation timeout; owner cleanup must be verified"}),
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    child.stdout.take().unwrap().read_to_end(&mut stdout)?;
    child.stderr.take().unwrap().read_to_end(&mut stderr)?;
    Ok(
        json!({"code":exit.code(),"stdout":String::from_utf8_lossy(&stdout),"stderr":String::from_utf8_lossy(&stderr)}),
    )
}
fn cycle_capture(captured: &Value) -> Value {
    // daily cycle 现在是 NDJSON 分步行 + 成功聚合行。非零退出仍保留前序行和 raw capture。
    let mut steps = Vec::new();
    let mut summary = Value::Null;
    let mut parse_error = Value::Null;
    for (index, line) in captured["stdout"]
        .as_str()
        .unwrap_or("")
        .lines()
        .enumerate()
    {
        match serde_json::from_str::<Value>(line) {
            Ok(value) if value["event"] == "cycle_step" => steps.push(value),
            Ok(value) if value["cycles"].is_array() => summary = value,
            Ok(_) => {
                parse_error = json!({"line_index":index + 1,"error":"unexpected cycle output"});
                break;
            }
            Err(error) => {
                parse_error = json!({"line_index":index + 1,"error":error.to_string()});
                break;
            }
        }
    }
    json!({"capture":captured,"steps":steps,"summary":summary,"parse_error":parse_error})
}
fn success(value: Value) -> Result<Value> {
    if value["code"] != 0 {
        return Err(value.to_string().into());
    }
    Ok(serde_json::from_str(
        value["stdout"].as_str().ok_or("missing stdout")?,
    )?)
}
fn status(uid: u32, gid: u32) -> Result<Value> {
    let mut command = client(uid, gid, "request");
    command.arg("Status");
    success(capture(command, Duration::from_secs(40))?)
}
fn spawn_owner(uid: u32, gid: u32) -> Result<Child> {
    client(uid, gid, "owner")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| spawn_error("harness.owner_spawn", e))
}
fn owner_lines(owner: &mut Child) -> Result<Vec<Value>> {
    let pipe = owner.stdout.as_mut().unwrap();
    unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) };
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut output = String::new();
    while output.lines().count() < 5 {
        let mut bytes = [0; 8192];
        match pipe.read(&mut bytes) {
            Ok(n) if n > 0 => output.push_str(&String::from_utf8_lossy(&bytes[..n])),
            Err(e) if e.kind() != std::io::ErrorKind::WouldBlock => return Err(e.into()),
            _ => {}
        }
        if output.len() > 65536 {
            return Err("owner output exceeded budget".into());
        }
        if Instant::now() > deadline {
            return Err("owner transient-disconnect probe timeout".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(output
        .lines()
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?)
}
fn wait_cleanup(uid: u32, gid: u32) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let status = status(uid, gid)?;
        let s = &status["response"]["result"]["status"];
        if s["recovery_required"] == true {
            return Err("RecoveryRequired after owner NOTE_EXIT".into());
        }
        if s["instance"].is_null() {
            if s["last"]["event"] != "owner_NOTE_EXIT" || s["last"]["group_empty"] != true {
                return Err("cleanup was not confirmed by owner NOTE_EXIT".into());
            }
            return Ok(status);
        }
        if Instant::now() > deadline {
            return Err("owner exit cleanup timeout".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
fn launchd_snapshot() -> Value {
    // 只允许读取这个固定 label；查询有自己的预算，不阻塞整个 socket 等待。
    let mut command = Command::new("/bin/launchctl");
    command.args(["print", &format!("system/{LABEL}")]);
    match capture(command, Duration::from_secs(1)) {
        Ok(value) => value,
        Err(e) => json!({"query_error":e.to_string()}),
    }
}
fn launchd_field<'a>(snapshot: &'a Value, name: &str) -> Option<&'a str> {
    let prefix = format!("\t{name} = ");
    // print 的顶层属性只有一个 tab；嵌套 endpoint/job 的 state 不代表本 daemon。
    snapshot["stdout"]
        .as_str()?
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
}
fn startup_failure(snapshot: &Value, was_running: bool) -> Option<&'static str> {
    if snapshot["code"].as_i64().is_some_and(|code| code != 0) {
        return Some("launchd fixed label unavailable");
    }
    let state = launchd_field(snapshot, "state");
    let attempted = was_running
        || launchd_field(snapshot, "runs")
            .and_then(|s| s.parse::<u64>().ok())
            .is_some_and(|n| n > 0)
        || launchd_field(snapshot, "last exit code")
            .and_then(|s| s.parse::<i32>().ok())
            .is_some()
        || launchd_field(snapshot, "last terminating signal").is_some();
    match state {
        Some("inactive" | "exited") => Some("launchd helper inactive before socket readiness"),
        Some("not running") if attempted => Some("launchd helper exited before socket readiness"),
        _ => None,
    }
}
fn startup_diagnostics(launchd: Value) -> Value {
    // 日志可能为空或未创建；它只能提供错误上下文，不能证明 daemon 已启动。
    let log = (|| -> Result<Value> {
        let file = path("daemon.log");
        if !resource_exists(&file)? {
            return Ok(json!({"state":"missing"}));
        }
        protected_root()?;
        protected(&file, false)?;
        let mut bytes = Vec::new();
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(file)?
            .take(8193)
            .read_to_end(&mut bytes)?;
        let truncated = bytes.len() > 8192;
        bytes.truncate(8192);
        Ok(
            json!({"state":if bytes.is_empty() {"empty"} else {"present"},
            "text":String::from_utf8_lossy(&bytes),"truncated":truncated}),
        )
    })()
    .unwrap_or_else(|e| json!({"read_error":e.to_string()}));
    let installed = (|| -> Result<Value> {
        protected_root()?;
        protected(HELPER, false)?;
        protected(PLIST, false)?;
        Ok(
            json!({"helper":HELPER,"helper_sha256":hash(Path::new(HELPER))?,
            "plist":PLIST,"plist_sha256":hash(Path::new(PLIST))?}),
        )
    })()
    .unwrap_or_else(|e| json!({"identity_error":e.to_string()}));
    json!({"launchd":launchd,"daemon_log":log,"installed":installed})
}
fn wait_for_socket(evidence: &mut Value) -> Result<()> {
    use std::os::unix::fs::FileTypeExt;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut was_running = false;
    loop {
        let snapshot = launchd_snapshot();
        let failure = startup_failure(&snapshot, was_running);
        was_running |= launchd_field(&snapshot, "state") == Some("running");
        if let Some(reason) = failure {
            evidence["startup_diagnostics"] = startup_diagnostics(snapshot);
            return Err(reason.into());
        }
        if resource_exists(&path("control.sock"))? {
            let meta = fs::symlink_metadata(path("control.sock"))?;
            if !meta.file_type().is_socket() {
                return Err("helper readiness path is not a Unix socket".into());
            }
            return Ok(());
        }
        if Instant::now() >= deadline {
            evidence["startup_diagnostics"] = startup_diagnostics(snapshot);
            return Err("launchd helper socket startup timeout; fixed diagnostics captured".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub fn privileged_probe(uid: u32, helper_digest: &str) -> Result<Value> {
    root()?;
    for file in [ROOT, HELPER, PLIST] {
        if resource_exists(file)? {
            return Err(
                "prototype resources already exist; no mutation or cleanup of an earlier run"
                    .into(),
            );
        }
    }
    let mut owner: Option<Child> = None;
    let mut evidence = json!({});
    let work = (|| -> Result<()> {
        evidence["install"] = install(uid, helper_digest)?;
        let (_, gid) = load_public_settings()?;
        wait_for_socket(&mut evidence)?;
        let mut permissions = Vec::new();
        for file in [
            ROOT,
            HELPER,
            PLIST,
            &path("veyra-sing-box"),
            &path("runtime"),
            &path("control.sock"),
        ] {
            let m = fs::symlink_metadata(file)?;
            permissions.push(json!({"path":file,"uid":m.uid(),"gid":m.gid(),"mode":format!("{:o}",m.mode()&0o777)}));
            if file != path("control.sock") {
                protected(file, m.is_dir())?;
            }
        }
        evidence["permissions"] = json!(permissions);
        let service = fs::read_to_string(path("service-id"))?;
        let snapshot = network("read", &service, &json!({}))?;
        evidence["snapshot"] = snapshot.clone();
        let captured = capture(client(uid, gid, "cycle"), Duration::from_secs(40))?;
        let report = cycle_capture(&captured);
        // 先登记再判断成功，FAIL 证据不能丢掉 daemon error 或 transport phase。
        evidence["cycles_diagnostics"] = report.clone();
        if captured["code"] != 0 || !report["parse_error"].is_null() || report["summary"].is_null()
        {
            return Err(format!("daily cycle failed: {report}").into());
        }
        evidence["cycles"] = report["summary"].clone();
        if network("read", &service, &json!({}))? != snapshot {
            return Err("two daily cycles did not restore exact snapshot".into());
        }
        evidence["two_cycle_exact_restore"] = json!(true);
        evidence["invalid_requests"] = success(capture(
            client(uid, gid, "reject-probe"),
            Duration::from_secs(40),
        )?)?;
        let nobody = unsafe { libc::getpwnam(c"nobody".as_ptr()) };
        if nobody.is_null() {
            return Err("nobody system UID unavailable".into());
        }
        let (other_uid, other_gid) = unsafe { ((*nobody).pw_uid, (*nobody).pw_gid) };
        if other_uid == uid {
            return Err("unauthorized UID equals authorized UID".into());
        }
        let mut denied = client(other_uid, other_gid, "request");
        denied.arg("Status");
        let denied = capture(denied, Duration::from_secs(40))?;
        if denied["code"] == 0 {
            return Err("different UID was not refused by socket ACL".into());
        }
        evidence["unauthorized_uid"] = json!({"uid":other_uid,"gid":other_gid,"result":denied});
        let mut root_request = Command::new(HELPER);
        root_request.args(["request", "Status"]);
        let refused = success(capture(root_request, Duration::from_secs(40))?)?;
        if refused["response"]["ok"] != false || refused["response"]["error"] != "UnauthorizedPeer"
        {
            return Err("server peer check did not reject root non-business caller".into());
        }
        evidence["server_peer_check"] = refused;
        owner = Some(spawn_owner(uid, gid)?);
        let child = owner.as_mut().unwrap();
        let lines = owner_lines(child)?;
        let first = &lines[1]["response"]["result"]["status"]["instance"];
        if first.is_null() {
            return Err("owner did not start instance".into());
        }
        for value in &lines[2..5] {
            if &value["response"]["result"]["status"]["instance"] != first {
                return Err("reconnect created or cleaned a different instance".into());
            }
        }
        let owner_pid = child.id() as i32;
        if first["owner"]["pid"] != owner_pid {
            return Err("owner PID did not come from the connected OS process".into());
        }
        let child_pid = first["child"]["pid"]
            .as_i64()
            .ok_or("missing owned child")? as i32;
        let mixed_port = first["mixed_port"].as_u64().ok_or("missing port")? as u16;
        let controller_port = first["controller_port"]
            .as_u64()
            .ok_or("missing controller")? as u16;
        let readback = network("read", &service, &json!({}))?;
        if readback["proxies"] != load("recovery.json")?["managed"] {
            return Err("independent active proxy readback failed".into());
        }
        evidence["active_readback"] = readback;
        let m = fs::metadata(path("runtime/config.json"))?;
        if m.uid() != 0 || m.mode() & 0o777 != 0o600 {
            return Err("config not root-only 0600".into());
        }
        evidence["config_permissions"] = json!({"uid":m.uid(),"gid":m.gid(),"mode":"600","child_input":"/dev/fd/3","controller_auth_200":true});
        evidence["owner_lifecycle"] = json!({"repeated_start_same_instance":true,"two_disconnects_each_seconds":2,"owner_alive_before_kill":process(owner_pid)?,"requests":lines});
        if unsafe { libc::kill(owner_pid, libc::SIGKILL) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        child.wait()?;
        evidence["owner_lifecycle"]["after_SIGKILL"] = wait_cleanup(uid, gid)?;
        let after = network("read", &service, &json!({}))?;
        if after != snapshot {
            return Err("GUI crash restore does not equal exact snapshot".into());
        }
        if process(child_pid).is_ok() {
            return Err("owned child remains after cleanup".into());
        }
        for port in [mixed_port, controller_port] {
            if std::net::TcpStream::connect_timeout(
                &format!("127.0.0.1:{port}").parse()?,
                Duration::from_millis(200),
            )
            .is_ok()
            {
                return Err("owned listener remains after cleanup".into());
            }
        }
        evidence["owner_lifecycle"]["independent_restored"] = after;
        evidence["owner_lifecycle"]["child_absent_and_listeners_closed"] = json!(true);
        // Separate external-write probe of the same dedicated disabled service.
        owner = Some(spawn_owner(uid, gid)?);
        owner_lines(owner.as_mut().unwrap())?;
        let observed = network("read", &service, &json!({}))?["proxies"].clone();
        let mut changed = observed.clone();
        changed["ProxyAutoConfigURLString"] = json!("http://127.0.0.1:9/p005-external.pac");
        network(
            "write",
            &service,
            &json!({"desired":changed,"expected":observed}),
        )?;
        let child = owner.as_mut().unwrap();
        child.kill()?;
        child.wait()?;
        let conflict_status = wait_cleanup(uid, gid)?;
        let conflict_read = network("read", &service, &json!({}))?;
        if conflict_read["proxies"]["ProxyAutoConfigURLString"]
            != "http://127.0.0.1:9/p005-external.pac"
        {
            return Err("external PAC field was overwritten".into());
        }
        if conflict_status["response"]["result"]["status"]["last"]["conflicts"]
            .as_array()
            .is_none_or(Vec::is_empty)
        {
            return Err("external modification conflict was not reported".into());
        }
        evidence["external_conflict"] = json!({"readback":conflict_read,"status":conflict_status});
        // Explicit harness reset of its own disabled, unassociated service.
        network(
            "write",
            &service,
            &json!({"desired":snapshot["proxies"],"expected":conflict_read["proxies"]}),
        )?;
        if network("read", &service, &json!({}))? != snapshot {
            return Err("test harness final snapshot restoration failed".into());
        }
        Ok(())
    })();
    if let Some(child) = owner.as_mut()
        && child.try_wait()?.is_none()
    {
        child.kill()?;
        child.wait()?;
    }
    // Finally: wait for OS owner exit; preserve recovery state on failed restore.
    let deadline = Instant::now() + Duration::from_secs(10);
    while Path::new(&path("recovery.json")).exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    // bootstrap 自身失败时也在卸载前保留固定状态/产物；后续 IPC 失败不冒称启动失败。
    if work.is_err()
        && evidence["permissions"].is_null()
        && evidence["startup_diagnostics"].is_null()
    {
        evidence["startup_diagnostics"] = startup_diagnostics(launchd_snapshot());
    }
    let cleanup = uninstall();
    evidence["result"] = match work {
        Ok(()) => json!({"status":"PASS"}),
        Err(e) => json!({"status":"FAIL","error":e.to_string()}),
    };
    evidence["cleanup"] = match cleanup {
        Ok(v) => v,
        Err(e) => json!({"status":"RecoveryRequired","error":e.to_string()}),
    };
    Ok(evidence)
}

// Ordinary-user API feasibility only; this never masquerades as a root daemon.
pub fn local_client() -> Result<Value> {
    let parent = unsafe { libc::getppid() };
    let mut stream = UnixStream::connect(format!(
        "/private/tmp/veyra-p005-local-{parent}/control.sock"
    ))?;
    let server = peer(&stream)?;
    if server.uid != unsafe { libc::geteuid() } || server.pid != parent {
        return Err("local probe server OS credential mismatch".into());
    }
    writeln!(
        stream,
        "{}",
        serde_json::to_string(&Request {
            command: Operation::Hello
        })?
    )?;
    read_frame(&mut stream, Duration::from_secs(2))?;
    drop(stream);
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
pub fn local_platform() -> Result<Value> {
    let directory = format!("/private/tmp/veyra-p005-local-{}", std::process::id());
    fs::create_dir(&directory)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    let mut child: Option<Child> = None;
    let work = (|| -> Result<Value> {
        let listener = UnixListener::bind(format!("{directory}/control.sock"))?;
        listener.set_nonblocking(true)?;
        child = Some(
            Command::new(std::env::current_exe()?)
                .arg("local-peer-client")
                .stdout(Stdio::null())
                .spawn()?,
        );
        let child = child.as_mut().unwrap();
        let watch = watch(child.id() as i32)?;
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.into()),
            }
            if Instant::now() > deadline {
                return Err("local peer connection timeout".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        stream.set_nonblocking(false)?;
        let actual = peer(&stream)?;
        if actual.pid != child.id() as i32
            || actual.uid != unsafe { libc::geteuid() }
            || actual.gid != unsafe { libc::getegid() }
        {
            return Err("local peer OS credentials mismatch".into());
        }
        let bytes = read_frame(&mut stream, Duration::from_millis(500))?;
        let _: Request = serde_json::from_slice(&bytes)?;
        writeln!(stream, "{}", json!({"ready":true}))?;
        let mut eof = [0];
        if read_with_deadline(&stream, &mut eof, Instant::now() + Duration::from_secs(1))? != 0 {
            return Err("expected local IPC EOF".into());
        }
        drop(stream);
        let wait_started = Instant::now();
        while wait_started.elapsed() < Duration::from_secs(2) {
            if exited(&watch)? {
                return Err("IPC EOF was misreported as process exit".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let identity = process(child.id() as i32)?;
        child.kill()?;
        child.wait()?;
        let deadline = Instant::now() + Duration::from_secs(2);
        while !exited(&watch)? {
            if Instant::now() > deadline {
                return Err("kqueue NOTE_EXIT not received".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Ok(
            json!({"status":"PASS","scope":"ordinary-user OS API feasibility; root helper acceptance NOT_RUN",
            "peer":actual,"actual_peer_pid_matches_held_child":true,"server_peer_checked_by_client":true,
            "IPC_EOF_owner_alive_seconds":2,"owner_identity_before_SIGKILL":identity,"exit_api":"kqueue EVFILT_PROC NOTE_EXIT","SIGKILL_NOTE_EXIT":true,"child_reaped":true}),
        )
    })();
    if let Some(child) = child.as_mut()
        && child.try_wait()?.is_none()
    {
        child.kill()?;
        child.wait()?;
    }
    fs::remove_dir_all(directory)?;
    work
}

#[cfg(test)]
mod startup_tests {
    use super::*;

    // 保护 root harness：中途 daemon 失败时保留成功前序、失败 step 和原始 stderr/退出码。
    #[test]
    fn cycle_capture_keeps_partial_steps_and_daemon_error() {
        let mut stdout = Vec::new();
        let mut count = 0;
        let error = daily_cycle(|_| {
            count += 1;
            Ok(json!({"response":{"ok":count < 3,"error":if count < 3 {Value::Null} else {json!("restore-stage")}}}))
        }, &mut stdout).unwrap_err();
        let captured = json!({"code":1,"stdout":String::from_utf8(stdout).unwrap(),"stderr":error.to_string()});
        let report = cycle_capture(&captured);
        assert_eq!(report["capture"], captured);
        let steps = report["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0]["outcome"], "ok");
        assert_eq!(steps[1]["outcome"], "ok");
        assert_eq!(steps[2]["operation"], "Restore");
        assert_eq!(steps[2]["daemon_error"], "restore-stage");
        assert!(report["summary"].is_null());
        assert!(report["parse_error"].is_null());
    }
    // 保护成功兼容：分步 NDJSON 与 main 的最后聚合行可以共同被 harness 捕获。
    #[test]
    fn cycle_capture_recognizes_streamed_success_summary() {
        let mut stdout = Vec::new();
        let summary = daily_cycle(|_| Ok(json!({"response":{"ok":true}})), &mut stdout).unwrap();
        writeln!(stdout, "{summary}").unwrap();
        let captured = json!({"code":0,"stdout":String::from_utf8(stdout).unwrap(),"stderr":""});
        let report = cycle_capture(&captured);
        assert_eq!(report["summary"], summary);
        assert_eq!(report["steps"].as_array().unwrap().len(), 8);
        assert!(report["parse_error"].is_null());
    }
    // 保护 transport 与超时/截断捕获：前序行仍可读，不能因缺少最后聚合行丢失。
    #[test]
    fn cycle_capture_preserves_transport_phase_and_truncated_tail() {
        let mut stdout = Vec::new();
        daily_cycle(
            |_| {
                Err(ClientError::new(
                    "server_peer",
                    std::io::Error::from_raw_os_error(libc::EINVAL),
                ))
            },
            &mut stdout,
        )
        .unwrap_err();
        stdout.extend_from_slice(b"{truncated");
        let captured = json!({"code":null,"stdout":String::from_utf8(stdout).unwrap(),"stderr":"","capture_error":"timeout"});
        let report = cycle_capture(&captured);
        assert_eq!(report["capture"], captured);
        assert_eq!(report["steps"][0]["client_phase"], "server_peer");
        assert_eq!(report["steps"][0]["outcome"], "client_error");
        assert_eq!(report["parse_error"]["line_index"], 2);
    }

    // 保护启动失败反馈：已运行后退出/信号退出应早报，不等 generic timeout。
    #[test]
    fn reports_exit_inactive_and_missing_label() {
        for stdout in [
            "job = {\n\tstate = not running\n\truns = 1\n\tlast exit code = 1\n}",
            "job = {\n\tstate = not running\n\tlast terminating signal = Killed: 9\n}",
            "job = {\n\tstate = inactive\n}",
        ] {
            assert!(startup_failure(&json!({"code":0,"stdout":stdout}), false).is_some());
        }
        assert!(
            startup_failure(
                &json!({"code":113,"stderr":"Could not find service"}),
                false
            )
            .is_some()
        );
        assert!(
            startup_failure(&json!({"code":0,"stdout":"\tstate = not running"}), true).is_some()
        );
    }
    // 保护启动竞争：尚未调度或状态查询超时不能被当作已退出，也不能被当作就绪。
    #[test]
    fn waits_for_pending_launch_and_unknown_status() {
        for snapshot in [
            json!({"code":0,"stdout":"\tstate = not running\n\truns = 0"}),
            json!({"code":0,"stdout":"\tstate = not running\n\truns = 0\n\tlast exit code = (never exited)"}),
            json!({"code":0,"stdout":"\tstate = spawn scheduled\n\truns = 0"}),
            json!({"query_error":"bounded query timeout"}),
        ] {
            assert!(startup_failure(&snapshot, false).is_none());
        }
    }
    // 保护 launchctl 文本解析：嵌套 endpoint 状态不能覆盖主 daemon 的 running。
    #[test]
    fn ignores_nested_launchd_states() {
        let snapshot = json!({"code":0,"stdout":"job = {\n\tstate = running\n\truns = 1\n\tendpoints = {\n\t\tstate = inactive\n\t}\n}"});
        assert_eq!(launchd_field(&snapshot, "state"), Some("running"));
        assert!(startup_failure(&snapshot, false).is_none());
    }
    // 保护旧失败场景：缺失或空 daemon.log 不会掩盖 launchd 退出事实。
    #[test]
    fn empty_log_does_not_override_exit() {
        for log in [
            json!({"state":"missing"}),
            json!({"state":"empty","text":""}),
        ] {
            let snapshot =
                json!({"code":0,"stdout":"\tstate = not running\n\truns = 1", "daemon_log":log});
            assert!(startup_failure(&snapshot, false).is_some());
        }
    }
}
