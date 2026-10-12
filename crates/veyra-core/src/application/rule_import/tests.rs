use super::*;
use crate::{
    domain::AppState,
    storage::{JsonStateStore, StateStore},
};
use std::{
    collections::BTreeMap,
    io::Write,
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};
struct Fixture {
    address: std::net::SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    workspace: OwnedWorkspace,
    before: Vec<u8>,
}
impl Fixture {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let worker = thread::spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(v) => v,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(e) => panic!("owned HTTP fixture: {e}"),
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                stream.set_nonblocking(false).unwrap();
                let mut request = String::new();
                std::io::BufRead::read_line(
                    &mut std::io::BufReader::new(&mut stream),
                    &mut request,
                )
                .unwrap();
                let path = request.split_whitespace().nth(1).unwrap_or_default();
                let (status,body):(&str,Vec<u8>)=match path {
                    "/small.list"=>("200 OK",b"# read-only preview\nDOMAIN,exact.invalid\nDOMAIN-SUFFIX,suffix.invalid\nDOMAIN-KEYWORD,keyword\nIP-CIDR,192.0.2.0/24,no-resolve\n".to_vec()),
                    "/large.list"=>("200 OK",(0..201).map(|n|format!("DOMAIN,n{n}.invalid\n")).collect::<String>().into_bytes()),
                    "/invalid.list"=>("200 OK",b"IP-CIDR,invalid/999\n".to_vec()),
                    "/unsupported.list"=>("200 OK",b"DOMAIN,exact.invalid\nDOMAIN-REGEX,.*\\.invalid\n".to_vec()),
                    "/small.mrs"=>("200 OK",include_bytes!("fixtures/small.mrs").to_vec()),
                    "/large.mrs"=>("200 OK",include_bytes!("fixtures/large.mrs").to_vec()),
                    "/youtube.mrs"=>("200 OK",include_bytes!("fixtures/youtube.mrs").to_vec()),
                    "/private.mrs"=>("200 OK",include_bytes!("fixtures/private.mrs").to_vec()),
                    "/small.srs"=>("200 OK",include_bytes!("fixtures/small.srs").to_vec()),
                    "/large.srs"=>("200 OK",include_bytes!("fixtures/large.srs").to_vec()),
                    "/invalid.mrs"|"/invalid.srs"=>("200 OK",b"invalid binary".to_vec()),
                    "/slow.list"=>{thread::sleep(Duration::from_millis(200));("200 OK",b"DOMAIN,slow.invalid\n".to_vec())},
                    "/redirect.list"=>("302 Found",Vec::new()),
                    _=>("503 Service Unavailable",Vec::new()),
                };
                let location = if status.starts_with("302") {
                    "Location: /small.list\r\n"
                } else {
                    ""
                };
                let header = format!(
                    "HTTP/1.1 {status}\r\n{location}Content-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        let workspace = OwnedWorkspace(std::env::temp_dir().join(format!(
            "veyra-rule-preview-fixture-{:?}",
            crate::domain::StateEpoch::fresh().unwrap()
        )));
        std::fs::create_dir(&workspace.0).unwrap();
        let store = JsonStateStore::new(workspace.0.join("state.json")).unwrap();
        store.save(&AppState::empty()).unwrap();
        let before = std::fs::read(workspace.0.join("state.json")).unwrap();
        Self {
            address,
            stop,
            worker: Some(worker),
            workspace,
            before,
        }
    }
    fn request(&self, path: &str) -> RuleImportRequest {
        RuleImportRequest {
            url: format!("http://{}{path}", self.address),
            path: RequestedPath::Direct,
            timeout: Duration::from_secs(10),
        }
    }
    fn assert_unchanged(&self) {
        assert_eq!(
            std::fs::read(self.workspace.0.join("state.json")).unwrap(),
            self.before
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
    }
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}
fn kernel() -> PathBuf {
    std::env::var_os("VEYRA_RULE_IMPORT_TEST_KERNEL")
        .expect("directed SRS test needs existing fixed kernel fixture")
        .into()
}
fn preview(
    service: &RuleImportService,
    request: RuleImportRequest,
) -> Result<RuleImportPreview, RuleImportError> {
    let (_cancel, rx) = watch::channel(false);
    runtime().block_on(service.preview(request, rx))
}

// 保护 URL 导入只预览、redirect 复用同一路径、合法/非法/超 200 不写配置，list/mrs无需内核。
#[test]
fn policy104_rule_import_list_mrs_url_preview_preserves_state_and_reports_full_count() {
    let fixture = Fixture::new();
    let service = RuleImportService::new(ProxySource::default(), None);
    let list = preview(&service, fixture.request("/small.list")).unwrap();
    assert_eq!(list.total, 4);
    assert_eq!(list.entries.len(), 4);
    assert!(!list.truncated);
    assert_eq!(
        preview(&service, fixture.request("/redirect.list")).unwrap(),
        list
    );
    assert_eq!(
        preview(&service, fixture.request("/small.mrs"))
            .unwrap()
            .entries,
        vec![RuleImportEntry {
            kind: RuleImportKind::Domain,
            value: "example.invalid".into()
        }]
    );
    for path in ["/large.list", "/large.mrs"] {
        let result = preview(&service, fixture.request(path)).unwrap();
        assert_eq!(result.total, 201);
        assert_eq!(result.entries.len(), 200);
        assert!(result.truncated);
    }
    assert_eq!(
        preview(&service, fixture.request("/invalid.list")),
        Err(RuleImportError::InvalidFormat)
    );
    assert_eq!(
        preview(&service, fixture.request("/invalid.mrs")),
        Err(RuleImportError::InvalidFormat)
    );
    assert_eq!(
        preview(&service, fixture.request("/unsupported.list")),
        Err(RuleImportError::UnsupportedRule)
    );
    assert_eq!(
        preview(&service, fixture.request("/small.srs")),
        Err(RuleImportError::KernelUnavailable)
    );
    fixture.assert_unchanged();
}
// 保护实际官方 MRS DomainSet/IPRange binary 可提取；回放冻结原件不访问外部 URL。
#[test]
fn policy104_rule_import_primary_mrs_fixtures_decode_domains_and_ip_ranges() {
    let fixture = Fixture::new();
    let service = RuleImportService::new(ProxySource::default(), None);
    let domains = preview(&service, fixture.request("/youtube.mrs")).unwrap();
    assert!(
        domains
            .entries
            .iter()
            .any(|e| e.kind == RuleImportKind::DomainSuffix && e.value == "youtube.com")
    );
    let ranges = preview(&service, fixture.request("/private.mrs")).unwrap();
    assert!(
        ranges
            .entries
            .iter()
            .any(|e| e.kind == RuleImportKind::IpCidr && e.value == "127.0.0.0/8")
    );
    assert!(ranges.entries.iter().any(|e| e.value == "::/127"));
    fixture.assert_unchanged();
}
// 保护真实固定内核 decompile 后的 typed 明细/超 200/非法 binary、每次目录清理且不写配置。
#[test]
fn policy104_rule_import_fixed_srs_preview_and_invalid_cleanup_preserve_state() {
    let fixture = Fixture::new();
    let service = RuleImportService::new(ProxySource::default(), Some(kernel()));
    let existing = import_workspaces();
    let small = preview(&service, fixture.request("/small.srs")).unwrap();
    assert_eq!(small.total, 4);
    assert_eq!(
        small.entries.iter().map(|e| e.kind).collect::<Vec<_>>(),
        vec![
            RuleImportKind::Domain,
            RuleImportKind::DomainSuffix,
            RuleImportKind::DomainKeyword,
            RuleImportKind::IpCidr
        ]
    );
    let large = preview(&service, fixture.request("/large.srs")).unwrap();
    assert_eq!(large.total, 201);
    assert!(large.truncated);
    assert_eq!(large.entries.len(), 200);
    assert_eq!(
        preview(&service, fixture.request("/invalid.srs")),
        Err(RuleImportError::DecompileFailed)
    );
    assert_eq!(import_workspaces(), existing);
    fixture.assert_unchanged();
}
fn import_workspaces() -> BTreeMap<PathBuf, ()> {
    std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("veyra-rule-import-")
        })
        .map(|p| (p, ()))
        .collect()
}
// 保护取消/超时/HTTP错误和不可用代理不把部分结果变保存成功；没有 Direct fallback。
#[test]
fn policy104_rule_import_cancel_timeout_http_error_and_unavailable_proxy_preserve_state() {
    let fixture = Fixture::new();
    let service = RuleImportService::new(ProxySource::default(), None);
    runtime().block_on(async {
        let (tx, rx) = watch::channel(false);
        let request = fixture.request("/slow.list");
        let operation = service.preview(request, rx);
        let cancel = async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            tx.send(true).unwrap();
        };
        let (result, _) = tokio::join!(operation, cancel);
        assert_eq!(result, Err(RuleImportError::Cancelled));
    });
    let mut request = fixture.request("/slow.list");
    request.timeout = Duration::from_millis(20);
    assert_eq!(preview(&service, request), Err(RuleImportError::Timeout));
    assert_eq!(
        preview(&service, fixture.request("/http-error.list")),
        Err(RuleImportError::Fetch(FetchError::HttpStatus(503)))
    );
    let mut request = fixture.request("/small.list");
    request.path = RequestedPath::ViaRunningProxy;
    assert_eq!(
        preview(&service, request),
        Err(RuleImportError::ProxyUnavailable)
    );
    fixture.assert_unchanged();
}
// 保护 SRS 默认多字段 AND / invert /复杂 wildcard 不被错误转换为独立 OR 明细。
#[test]
fn policy104_rule_import_complex_rules_are_explicitly_rejected() {
    assert_eq!(
        parse_srs_json(br#"{"rules":[{"domain":["a.invalid"],"ip_cidr":["192.0.2.0/24"]}]}"#),
        Err(RuleImportError::UnsupportedRule)
    );
    assert_eq!(
        parse_srs_json(br#"{"rules":[{"domain":["a.invalid"],"invert":true}]}"#),
        Err(RuleImportError::UnsupportedRule)
    );
    assert_eq!(
        parse_srs_json(
            br#"{"rules":[{"type":"logical","mode":"and","rules":[{"domain":["a.invalid"]}]}]}"#
        ),
        Err(RuleImportError::UnsupportedRule)
    );
    let or=parse_srs_json(br#"{"rules":[{"type":"logical","mode":"or","rules":[{"domain":["a.invalid"]},{"domain_suffix":["b.invalid"]}]}]}"#).unwrap();
    assert_eq!(or.len(), 2);
}

// 保护 MRS zstd checksum、完整 frame 和声明长度不合法时明确拒绝，避免坏 binary 被当成明细。
#[test]
fn policy104_rule_import_mrs_checksum_trailing_and_truncation_are_rejected() {
    let mut broken = include_bytes!("fixtures/small.mrs").to_vec();
    let last = broken.len() - 1;
    broken[last] ^= 1;
    assert_eq!(mrs::parse(&broken), Err(RuleImportError::InvalidFormat));
    let mut trailing = include_bytes!("fixtures/small.mrs").to_vec();
    trailing.push(0);
    assert_eq!(mrs::parse(&trailing), Err(RuleImportError::InvalidFormat));
    let truncated = &include_bytes!("fixtures/small.mrs")[..10];
    assert_eq!(mrs::parse(truncated), Err(RuleImportError::InvalidFormat));
}
