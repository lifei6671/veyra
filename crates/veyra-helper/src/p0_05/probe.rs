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
        command.pre_exec(move || {
            if libc::setgroups(0, std::ptr::null()) != 0
                || libc::setgid(gid) != 0
                || libc::setuid(uid) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command
}
fn capture(mut command: Command) -> Result<Value> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    for fd in [
        child.stdout.as_ref().unwrap().as_raw_fd(),
        child.stderr.as_ref().unwrap().as_raw_fd(),
    ] {
        unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };
    }
    let deadline = Instant::now() + Duration::from_secs(40);
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
            return Err("fixed client operation timeout; owner cleanup must be verified".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    child.stdout.take().unwrap().read_to_end(&mut stdout)?;
    child.stderr.take().unwrap().read_to_end(&mut stderr)?;
    Ok(
        json!({"code":exit.code(),"stdout":String::from_utf8_lossy(&stdout),"stderr":String::from_utf8_lossy(&stderr)}),
    )
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
    success(capture(command)?)
}
fn spawn_owner(uid: u32, gid: u32) -> Result<Child> {
    Ok(client(uid, gid, "owner")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?)
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
pub fn privileged_probe(uid: u32, helper_digest: &str) -> Result<Value> {
    root()?;
    for file in [ROOT, HELPER, PLIST] {
        if Path::new(file).exists() {
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
        let deadline = Instant::now() + Duration::from_secs(10);
        while !Path::new(&path("control.sock")).exists() {
            if Instant::now() > deadline {
                return Err("launchd helper socket startup timeout".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let mut permissions = Vec::new();
        for file in [
            ROOT,
            HELPER,
            PLIST,
            &path("sing-box"),
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
        evidence["cycles"] = success(capture(client(uid, gid, "cycle"))?)?;
        if network("read", &service, &json!({}))? != snapshot {
            return Err("two daily cycles did not restore exact snapshot".into());
        }
        evidence["two_cycle_exact_restore"] = json!(true);
        evidence["invalid_requests"] = success(capture(client(uid, gid, "reject-probe"))?)?;
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
        let denied = capture(denied)?;
        if denied["code"] == 0 {
            return Err("different UID was not refused by socket ACL".into());
        }
        evidence["unauthorized_uid"] = json!({"uid":other_uid,"gid":other_gid,"result":denied});
        let mut root_request = Command::new(HELPER);
        root_request.args(["request", "Status"]);
        let refused = success(capture(root_request)?)?;
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
        stream.set_read_timeout(Some(Duration::from_secs(1)))?;
        if stream.read(&mut eof)? != 0 {
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
