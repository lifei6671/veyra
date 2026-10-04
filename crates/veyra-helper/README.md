# P0-05 helper prototype

This task-card path contains only the isolated `p0-05-prototype` entry, named
`veyra-helper-prototype`. It is not the production P2 helper, exports no library,
and is not a dependency of `veyra-core`.

macOS arm64 only; builds require `MACOSX_DEPLOYMENT_TARGET=15.0`:

```sh
MACOSX_DEPLOYMENT_TARGET=15.0 cargo check -p veyra-helper-prototype --features p0-05-prototype
MACOSX_DEPLOYMENT_TARGET=15.0 cargo build -p veyra-helper-prototype --features p0-05-prototype
MACOSX_DEPLOYMENT_TARGET=15.0 cargo test -p veyra-helper-prototype --features p0-05-prototype
MACOSX_DEPLOYMENT_TARGET=15.0 cargo clippy -p veyra-helper-prototype --features p0-05-prototype --all-targets -- -D warnings
cargo fmt -p veyra-helper-prototype -- --check
```

The five pure tests protect closed requests, bounded framing/deadlines and exact
or conflict-aware proxy restoration. They do not install a daemon, launch a kernel
or modify SystemConfiguration. `local-platform-probe` separately tests ordinary
user OS peer credentials and process exit events in an exclusive temporary socket.

The [fixed orchestrator](../../tools/p0-05-helper-probe.py) downloads/checks the
official archive as the ordinary user, demonstrates manual loopback/FD input,
then invokes the standard macOS administrator UI. Do not launch it with sudo;
do not provide passwords in arguments, files, environment or chat.

```sh
python3 tools/p0-05-helper-probe.py --output /private/tmp/veyra-p005-replay-evidence
```

Use a new empty output directory. Existing P005 protected resources cause refusal
without mutation or cleanup. `--no-authorize` records ordinary-user evidence only.
The administrator explicitly authorizes a fixed `privileged-probe <business UID> <verified helper SHA256>`
command; its root harness installs only the verified local helper/kernel,
creates one disabled service outside every Network Set, drops credentials for
clients, exercises fixed operations and finally restores/stops/bootouts/deletes.
Daily IPC has no payload UID/PID/path/service/config fields.

Protected prototype resources are `com.lifei6671.veyra.p005` under the traditional
PrivilegedHelperTools/LaunchDaemons paths and `/Library/Application Support/VeyraP005`.
The daemon uses `getpeereid`, `LOCAL_PEERPID`, `proc_pidinfo` and kqueue `NOTE_EXIT`.
The root-only 0600 config is passed through inherited FD 3 after clearing groups,
setting gid/uid and creating an owned process group. Only that fixed installed
sing-box may execute. HTTP/HTTPS/SOCKS/PAC/discovery/bypass restoration compares
related field groups; native writes recheck the observed dictionary under the
SCPreferences lock. Root service writes require the recorded service to remain
disabled, P005-named and absent from all Network Sets.

If restore fails, the daemon retains the endpoint and recovery record, and the
uninstaller refuses removal. A prior recovery record prevents a new writer.
This P0 entry deliberately does not implement production restart recovery,
multi-service support, TUN, updates or GUI. Never treat successful compilation
or ordinary-user API tests as proof of privileged platform acceptance.

Current outcome and exact limitations: [P0-05 evidence](../../docs/openbox-rust-gpui-tasks/evidence/p0-05/README.md).
