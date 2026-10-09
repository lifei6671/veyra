//! 日志页面只持有已脱敏记录；原始 payload 不进入 Debug、队列或导出。
use super::controller::Identity;
use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use std::{
    collections::VecDeque,
    sync::{Arc, OnceLock},
};

pub const MAX_BODY_BYTES: usize = 4096;
pub const UI_CAPACITY: usize = 1000;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
    Panic,
    Silent,
    Unknown,
}
impl Level {
    pub const ALL: [Self; 8] = [
        Self::Trace,
        Self::Debug,
        Self::Info,
        Self::Warning,
        Self::Error,
        Self::Fatal,
        Self::Panic,
        Self::Silent,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Fatal => "fatal",
            Self::Panic => "panic",
            Self::Silent => "silent",
            Self::Unknown => "unknown",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "trace" => Self::Trace,
            "debug" => Self::Debug,
            "info" => Self::Info,
            "warn" | "warning" => Self::Warning,
            "error" => Self::Error,
            "fatal" => Self::Fatal,
            "panic" => Self::Panic,
            "silent" => Self::Silent,
            _ => Self::Unknown,
        }
    }
    fn rank(self) -> u8 {
        match self {
            Self::Trace => 0,
            Self::Debug => 1,
            Self::Info => 2,
            Self::Warning => 3,
            Self::Error => 4,
            Self::Fatal => 5,
            Self::Panic => 6,
            Self::Silent => 7,
            Self::Unknown => 8,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Category {
    Inbound,
    Outbound,
    Dns,
    Router,
    Runtime,
    Unknown,
}
impl Category {
    pub const ALL: [Self; 6] = [
        Self::Inbound,
        Self::Outbound,
        Self::Dns,
        Self::Router,
        Self::Runtime,
        Self::Unknown,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
            Self::Dns => "dns",
            Self::Router => "router",
            Self::Runtime => "runtime",
            Self::Unknown => "unknown",
        }
    }
    fn parse(s: &str) -> Self {
        let s = if s.starts_with('[') {
            s.split_once("] ").map_or(s, |(_, tail)| tail)
        } else {
            s
        };
        match s.split(['/', ':', ' ', '[']).next().unwrap_or("") {
            "inbound" => Self::Inbound,
            "outbound" => Self::Outbound,
            "dns" => Self::Dns,
            "router" => Self::Router,
            "runtime" => Self::Runtime,
            _ => Self::Unknown,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    pub identity: Identity,
    pub stream_generation: u64,
    pub sequence: u64,
    pub received_at_ms: i64,
    pub level: Level,
    pub category: Category,
    pub truncated: bool,
    body: String,
}
impl Record {
    pub fn body(&self) -> &str {
        &self.body
    }
}
// 含认证/订阅线索时整行省略，避免格式不标准或多词密码只遮掉一部分。
// 普通网络日志保留域名/端口与连接正文；URL（含路径/query/userinfo）整体遮掉。
fn redact(raw: &str) -> String {
    static SENSITIVE: OnceLock<Regex> = OnceLock::new();
    static URL: OnceLock<Regex> = OnceLock::new();
    static OPAQUE: OnceLock<Regex> = OnceLock::new();
    let sensitive=SENSITIVE.get_or_init(||Regex::new(r"(?i)authorization|proxy-authorization|bearer\s|basic\s|password|passwd|token|secret|credential|subscription|subscribe|api[_-]?key|private[_-]?key|access[_-]?key|uuid").expect("fixed sensitive pattern"));
    if sensitive.is_match(raw) {
        return "[REDACTED]".into();
    }
    let url = URL.get_or_init(|| {
        Regex::new(r"(?i)[a-z][a-z0-9+.-]*://[^\s]+|//[^\s]+@[^\s]+").expect("fixed URL pattern")
    });
    let opaque=OPAQUE.get_or_init(||Regex::new(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b|[a-z0-9_+/=-]{32,}").expect("fixed opaque pattern"));
    let text = url.replace_all(raw, "[REDACTED URL]");
    opaque
        .replace_all(&text, "[REDACTED]")
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
pub(crate) fn parse(
    text: &str,
    identity: Identity,
    generation: u64,
    sequence: u64,
    received_at_ms: i64,
) -> Result<Record, ()> {
    // 借用解析/固定错误，解析失败不包含 serde 的原文上下文。
    #[derive(Deserialize)]
    struct Wire<'a> {
        #[serde(rename = "type", borrow)]
        kind: &'a str,
        payload: serde_json::Value,
    }
    let frame: Wire<'_> = serde_json::from_str(text).map_err(|_| ())?;
    let level = Level::parse(frame.kind);
    let mut body = frame
        .payload
        .as_str()
        .map(redact)
        .unwrap_or_else(|| "[REDACTED]".into());
    let category = Category::parse(&body);
    let truncated = body.len() > MAX_BODY_BYTES;
    if truncated {
        let mut end = MAX_BODY_BYTES - 3;
        while !body.is_char_boundary(end) {
            end -= 1;
        }
        body.truncate(end);
        body.push('…');
    }
    Ok(Record {
        identity,
        stream_generation: generation,
        sequence,
        received_at_ms,
        level,
        category,
        truncated,
        body,
    })
}
/// 页面会话缓冲；与 Runtime 诊断独立，暂停/离页由消费者弃消息而非停采集。
pub struct Buffer {
    pub identity: Option<Identity>,
    pub paused: bool,
    pub dropped: u64,
    pub minimum: Level,
    pub category: Option<Category>,
    records: VecDeque<Arc<Record>>,
    regex: Option<Regex>,
    pub invalid_regex: bool,
}
impl Default for Buffer {
    fn default() -> Self {
        Self {
            identity: None,
            paused: false,
            dropped: 0,
            minimum: Level::Info,
            category: None,
            records: VecDeque::new(),
            regex: None,
            invalid_regex: false,
        }
    }
}
impl Buffer {
    pub fn set_identity(&mut self, identity: Option<Identity>) -> bool {
        if self.identity == identity {
            return false;
        }
        self.identity = identity;
        self.records.clear();
        self.dropped = 0;
        true
    }
    pub fn push(&mut self, record: Arc<Record>) -> bool {
        if self.paused || self.identity.as_ref() != Some(&record.identity) {
            return false;
        }
        if self.records.back().is_some_and(|r| {
            r.stream_generation > record.stream_generation
                || (r.stream_generation == record.stream_generation
                    && r.sequence >= record.sequence)
        }) {
            return false;
        }
        if self.records.len() == UI_CAPACITY {
            self.records.pop_front();
        }
        self.records.push_back(record);
        true
    }
    pub fn clear(&mut self) {
        self.records.clear();
        self.dropped = 0;
    }
    pub fn len(&self) -> usize {
        self.records.len()
    }
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    pub fn query(&mut self, query: &str) {
        let result = if query.is_empty() {
            Ok(None)
        } else {
            RegexBuilder::new(query)
                .case_insensitive(true)
                .size_limit(256 * 1024)
                .build()
                .map(Some)
        };
        self.invalid_regex = result.is_err();
        self.regex = result.ok().flatten();
    }
    pub fn visible(&self) -> Vec<Arc<Record>> {
        self.records
            .iter()
            .rev()
            .filter(|r| {
                !self.invalid_regex
                    && self.minimum != Level::Silent
                    && (r.level == Level::Unknown || r.level.rank() >= self.minimum.rank())
                    && self.category.is_none_or(|c| c == r.category)
                    && self.regex.as_ref().is_none_or(|re| {
                        re.is_match(r.body())
                            || re.is_match(r.level.label())
                            || re.is_match(&time_label(r.received_at_ms))
                    })
            })
            .cloned()
            .collect()
    }
    pub fn export(&self) -> String {
        self.visible()
            .iter()
            .map(|r| {
                format!(
                    "{}\t{}\t{}\t{}",
                    r.sequence,
                    time_label(r.received_at_ms),
                    r.level.label(),
                    r.body()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
#[cfg(unix)]
pub fn time_label(ms: i64) -> String {
    // 与页面本机时钟一致；时间来自采集入口，非渲染时刻。
    let seconds = ms.div_euclid(1000) as libc::time_t;
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    if unsafe { libc::localtime_r(&seconds, local.as_mut_ptr()) }.is_null() {
        return "—".into();
    }
    let local = unsafe { local.assume_init() };
    format!(
        "{:02}:{:02}:{:02}",
        local.tm_hour, local.tm_min, local.tm_sec
    )
}
pub fn normalize_query(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    let candidate = if value.contains("://") {
        value.to_owned()
    } else if value.starts_with("//") {
        format!("http:{value}")
    } else {
        format!("http://{value}")
    };
    reqwest::Url::parse(&candidate)
        .ok()
        .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
        .unwrap_or_else(|| {
            value
                .rsplit_once(':')
                .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
                .map_or(value, |(host, _)| host)
                .to_owned()
        })
}

#[cfg(not(unix))]
pub fn time_label(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d| d.format("%H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "—".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::runtime_snapshot::InstanceId;
    fn id() -> Identity {
        Identity {
            instance_id: InstanceId("logs-owned".into()),
            generation: 1,
        }
    }
    fn record(seq: u64, level: &str, body: &str) -> Arc<Record> {
        Arc::new(
            parse(
                &serde_json::json!({"type":level,"payload":body}).to_string(),
                id(),
                1,
                seq,
                1_000,
            )
            .unwrap(),
        )
    }
    // 用户正文搜索/导出仍能看到真实安全网络正文，所有认证材料在入队前省略。
    #[test]
    fn safe_body_redacts_credentials_urls_and_structured_payload_without_raw_debug() {
        for body in [
            "password = a multi word value",
            "Authorization: Basic YTpi",
            "https://a:b@example.org/feed?q=value",
            "subscription failed with opaque",
            "12345678-1234-1234-1234-123456789abc",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890",
        ] {
            let r = record(1, "info", body);
            assert!(!r.body().contains(body));
            assert!(!format!("{r:?}").contains(body));
        }
        let r = record(
            2,
            "info",
            "[7 0ms] outbound/direct[direct]: outbound connection to example.org:443",
        );
        assert_eq!(r.category, Category::Outbound);
        assert!(r.body().contains("example.org:443"));
        let structured = parse(
            r#"{"type":"trace","payload":{"password":"do not display"}}"#,
            id(),
            1,
            3,
            1000,
        )
        .unwrap();
        assert_eq!(structured.body(), "[REDACTED]");
        assert_eq!(structured.level, Level::Trace);
    }
    // 保护UTF8截断、不同级别与未知类别不造默认身份。
    #[test]
    fn body_is_utf8_byte_bounded_and_original_levels_are_typed() {
        let r = record(1, "debug", &"中文日志 ".repeat(1000));
        assert!(r.truncated);
        assert!(r.body().len() <= MAX_BODY_BYTES);
        assert!(r.body().ends_with('…'));
        assert_eq!(r.level, Level::Debug);
        assert_eq!(r.category, Category::Unknown);
        for l in Level::ALL {
            assert_eq!(record(2, l.label(), "dns query example.org").level, l);
        }
        assert_eq!(
            record(3, "unrecognized-sensitive-type", "router match").level,
            Level::Unknown
        );
    }
    // 保护UI1000条上界、暂停弃消息、换源拒绝晚到帧、重连序号去重。
    #[test]
    fn bounded_page_pause_clear_and_instance_fence() {
        let mut b = Buffer::default();
        b.set_identity(Some(id()));
        for seq in 1..=1005 {
            assert!(b.push(record(seq, "info", "dns query example.org")));
        }
        assert_eq!(b.len(), 1000);
        assert_eq!(b.visible().last().unwrap().sequence, 6);
        b.paused = true;
        assert!(!b.push(record(1006, "info", "paused")));
        b.paused = false;
        assert!(b.push(record(1007, "info", "resumed")));
        assert!(!b.push(record(1007, "info", "duplicate")));
        assert!(b.export().contains("resumed"));
        assert!(!b.export().contains("paused"));
        b.clear();
        assert!(b.is_empty());
        assert_eq!(b.identity, Some(id()));
        b.set_identity(Some(Identity {
            instance_id: InstanceId("replacement".into()),
            generation: 2,
        }));
        assert!(!b.push(record(1008, "info", "old")));
    }
    // 保护等级/真实类别/正文及时间正则、非法表达式修复不重进页面、导出实际筛选结果。
    #[test]
    fn filters_regex_repair_and_export_are_the_same_projection() {
        let mut b = Buffer::default();
        b.set_identity(Some(id()));
        b.push(record(1, "debug", "dns query other.org"));
        b.push(record(
            2,
            "info",
            "outbound/direct[owned]: connection to example.org:443",
        ));
        b.push(record(3, "error", "dns query example.net"));
        assert_eq!(b.visible().len(), 2);
        b.minimum = Level::Trace;
        assert_eq!(b.visible().len(), 3);
        b.category = Some(Category::Dns);
        b.query("example\\.(net|org)");
        assert_eq!(b.visible().len(), 1);
        assert!(b.export().contains("example.net"));
        assert!(!b.export().contains("other.org"));
        b.query("[");
        assert!(b.invalid_regex);
        assert!(b.export().is_empty());
        b.query("dns");
        assert!(!b.invalid_regex);
        assert_eq!(b.visible().len(), 2);
        b.minimum = Level::Silent;
        assert!(b.visible().is_empty());
        b.minimum = Level::Fatal;
        assert!(b.visible().is_empty());
    }
    // 保护格式化行为只变搜索文本；不发请求或从URL泄露认证正文。
    #[test]
    fn query_normalization_matches_reference_host_semantics() {
        assert_eq!(
            normalize_query(" HTTPS://Example.COM:8443/path?q=1 "),
            "example.com"
        );
        assert_eq!(normalize_query("example.com:443"), "example.com");
        assert_eq!(normalize_query("//Example.NET/path"), "example.net");
        assert_eq!(normalize_query("["), "[");
    }
}
