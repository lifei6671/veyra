use std::time::Duration;

use reqwest::{
    Client, StatusCode, Url,
    header::{
        CONTENT_DISPOSITION, ETAG, HeaderMap, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED,
    },
};

use crate::domain::{SubscriptionHttpMetadata, SubscriptionTraffic};

const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const TOTAL_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn default_user_agent() -> String {
    "clash-verge/v2.5".to_owned()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FetchClientOptions {
    pub user_agent: Option<String>,
    pub timeout: Duration,
    pub proxy_url: Option<String>,
    pub verify_tls: bool,
}

impl Default for FetchClientOptions {
    fn default() -> Self {
        Self {
            user_agent: None,
            timeout: TOTAL_TIMEOUT,
            proxy_url: None,
            verify_tls: true,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ConditionalHeaders {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FetchResult {
    Modified {
        body: String,
        metadata: SubscriptionHttpMetadata,
        profile_update_interval_minutes: Option<u32>,
        validators_match_source: bool,
    },
    NotModified {
        metadata: SubscriptionHttpMetadata,
        profile_update_interval_minutes: Option<u32>,
        validators_match_source: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FetchError {
    InvalidUrl,
    RequestFailed,
    InvalidStatus,
    BodyTooLarge,
    InvalidBody,
}

#[cfg(test)]
pub(crate) fn build_client() -> Result<Client, FetchError> {
    build_client_with_options(&FetchClientOptions::default())
}

pub(crate) fn build_client_with_options(
    options: &FetchClientOptions,
) -> Result<Client, FetchError> {
    let connect_timeout = options.timeout.min(Duration::from_secs(10));
    let mut builder = Client::builder()
        .user_agent(
            options
                .user_agent
                .clone()
                .unwrap_or_else(default_user_agent),
        )
        .connect_timeout(connect_timeout)
        .danger_accept_invalid_certs(!options.verify_tls)
        .redirect(reqwest::redirect::Policy::none());
    if let Some(proxy_url) = &options.proxy_url {
        builder =
            builder.proxy(reqwest::Proxy::all(proxy_url).map_err(|_| FetchError::InvalidUrl)?);
    } else {
        builder = builder.no_proxy();
    }
    builder.build().map_err(|_| FetchError::RequestFailed)
}

#[cfg(test)]
pub(crate) async fn fetch_subscription(
    client: &Client,
    source: &str,
    conditional: ConditionalHeaders,
) -> Result<FetchResult, FetchError> {
    fetch_subscription_with_timeout(client, source, conditional, TOTAL_TIMEOUT).await
}

pub(crate) async fn fetch_subscription_with_options(
    source: &str,
    conditional: ConditionalHeaders,
    options: &FetchClientOptions,
) -> Result<FetchResult, FetchError> {
    let client = build_client_with_options(options)?;
    fetch_subscription_with_timeout(&client, source, conditional, options.timeout).await
}

async fn fetch_subscription_with_timeout(
    client: &Client,
    source: &str,
    conditional: ConditionalHeaders,
    total_timeout: Duration,
) -> Result<FetchResult, FetchError> {
    let initial = Url::parse(source).map_err(|_| FetchError::InvalidUrl)?;
    validate_url(&initial, None)?;
    tokio::time::timeout(
        total_timeout,
        fetch_with_redirects(client, initial, conditional),
    )
    .await
    .map_err(|_| FetchError::RequestFailed)?
}

async fn fetch_with_redirects(
    client: &Client,
    initial: Url,
    conditional: ConditionalHeaders,
) -> Result<FetchResult, FetchError> {
    let initial_origin = origin(&initial);
    let initial_scheme = initial.scheme().to_owned();
    let mut current = initial;
    for redirects in 0..=MAX_REDIRECTS {
        let mut request = client.get(current.clone());
        if origin(&current) == initial_origin {
            if let Some(etag) = conditional.etag.as_deref() {
                request = request.header(IF_NONE_MATCH, etag);
            }
            if let Some(last_modified) = conditional.last_modified.as_deref() {
                request = request.header(IF_MODIFIED_SINCE, last_modified);
            }
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| FetchError::RequestFailed)?;
        let validators_match_source = origin(&current) == initial_origin;
        if response.status() == StatusCode::NOT_MODIFIED {
            let mut metadata = response_metadata(response.headers());
            if !validators_match_source {
                metadata.etag = None;
                metadata.last_modified = None;
            }
            return Ok(FetchResult::NotModified {
                metadata,
                profile_update_interval_minutes: profile_update_interval_minutes(
                    response.headers(),
                ),
                validators_match_source,
            });
        }
        if response.status().is_redirection() {
            if redirects == MAX_REDIRECTS {
                return Err(FetchError::RequestFailed);
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(FetchError::RequestFailed)?;
            let next = current.join(location).map_err(|_| FetchError::InvalidUrl)?;
            validate_url(&next, Some(&initial_scheme))?;
            current = next;
            continue;
        }

        let mut metadata = response_metadata(response.headers());
        let profile_update_interval_minutes = profile_update_interval_minutes(response.headers());
        if !validators_match_source {
            metadata.etag = None;
            metadata.last_modified = None;
        }
        if response.status() != StatusCode::OK {
            return Err(FetchError::InvalidStatus);
        }

        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| FetchError::RequestFailed)?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
                return Err(FetchError::BodyTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        let body = String::from_utf8(bytes).map_err(|_| FetchError::InvalidBody)?;
        return Ok(FetchResult::Modified {
            body,
            metadata,
            profile_update_interval_minutes,
            validators_match_source,
        });
    }
    Err(FetchError::RequestFailed)
}

pub(crate) fn validate_source_url(source: &str) -> Result<(), FetchError> {
    if source.len() > 8_192 || source.trim() != source {
        return Err(FetchError::InvalidUrl);
    }
    let url = Url::parse(source).map_err(|_| FetchError::InvalidUrl)?;
    validate_url(&url, None)
}

fn validate_url(url: &Url, initial_scheme: Option<&str>) -> Result<(), FetchError> {
    if url.username() != "" || url.password().is_some() || url.host_str().is_none() {
        return Err(FetchError::InvalidUrl);
    }
    if initial_scheme == Some("https") && url.scheme() != "https" {
        return Err(FetchError::InvalidUrl);
    }
    match url.scheme() {
        "https" => Ok(()),
        "http"
            if matches!(url.host_str(), Some("127.0.0.1" | "::1" | "[::1]"))
                && url.query().is_none()
                && url.fragment().is_none() =>
        {
            Ok(())
        }
        _ => Err(FetchError::InvalidUrl),
    }
}

fn origin(url: &Url) -> (String, Option<String>, Option<u16>) {
    (
        url.scheme().to_owned(),
        url.host_str().map(ToOwned::to_owned),
        url.port_or_known_default(),
    )
}

fn response_metadata(headers: &HeaderMap) -> SubscriptionHttpMetadata {
    SubscriptionHttpMetadata {
        etag: bounded_header(headers, ETAG, 1_024),
        last_modified: bounded_header(headers, LAST_MODIFIED, 128),
        subscription_userinfo: headers
            .get("subscription-userinfo")
            .and_then(|value| value.to_str().ok())
            .filter(|value| value.len() <= 1_024)
            .and_then(parse_userinfo),
        content_disposition: headers
            .get(CONTENT_DISPOSITION)
            .and_then(|value| value.to_str().ok())
            .filter(|value| value.len() <= 1_024)
            .and_then(parse_filename),
    }
}

fn profile_update_interval_minutes(headers: &HeaderMap) -> Option<u32> {
    let value = headers
        .get("profile-update-interval")?
        .to_str()
        .ok()?
        .trim();
    if value.is_empty() || value.len() > 20 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let hours = value.parse::<u64>().ok()?;
    let minutes = hours.checked_mul(60)?;
    let minutes = u32::try_from(minutes).ok()?;
    (minutes >= 1_440).then_some(minutes)
}

fn bounded_header(
    headers: &HeaderMap,
    name: reqwest::header::HeaderName,
    max: usize,
) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= max
                && value.chars().all(|character| !character.is_control())
        })
        .map(ToOwned::to_owned)
}

fn parse_userinfo(value: &str) -> Option<SubscriptionTraffic> {
    let mut traffic = SubscriptionTraffic::default();
    let mut recognized = false;
    for part in value.split(';') {
        let Some((key, value)) = part.trim().split_once('=') else {
            continue;
        };
        let parsed = value.trim().parse::<u64>().ok()?;
        if parsed > MAX_SAFE_INTEGER {
            return None;
        }
        match key.trim().to_ascii_lowercase().as_str() {
            "upload" if traffic.upload.is_none() => traffic.upload = Some(parsed),
            "download" if traffic.download.is_none() => traffic.download = Some(parsed),
            "total" if traffic.total.is_none() => traffic.total = Some(parsed),
            "expire" if traffic.expire_at_ms.is_none() => {
                let expire_at_ms = parsed.checked_mul(1_000)?;
                if expire_at_ms > MAX_SAFE_INTEGER {
                    return None;
                }
                traffic.expire_at_ms = Some(expire_at_ms)
            }
            "upload" | "download" | "total" | "expire" => return None,
            _ => continue,
        }
        recognized = true;
    }
    recognized.then_some(traffic)
}

fn parse_filename(value: &str) -> Option<String> {
    let filename = value.split(';').skip(1).find_map(|parameter| {
        let (key, value) = parameter.trim().split_once('=')?;
        key.eq_ignore_ascii_case("filename")
            .then(|| value.trim().trim_matches('"'))
    })?;
    let filename = filename.rsplit(['/', '\\']).next()?.trim();
    (!filename.is_empty()
        && filename.len() <= 255
        && filename.chars().all(|character| !character.is_control()))
    .then(|| filename.to_owned())
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
    };

    use super::*;

    #[test]
    fn compatibility_005_request_identifies_client_and_parses_clash_response() {
        let body = "proxies: [{name: Remote Clash, type: vmess, server: example.invalid, port: 443, uuid: test-uuid, alterId: 0, cipher: auto, tls: true, servername: tls.example.invalid}]\n";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/yaml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let (url, request, server) = serve_once(response);
        let result = tauri::async_runtime::block_on(fetch_subscription(
            &build_client().expect("client"),
            &url,
            ConditionalHeaders::default(),
        ))
        .expect("fetch");
        server.join().expect("fixture exit");
        let request = request.lock().expect("captured request");
        assert!(request.contains("user-agent: clash-verge/v2.5\r\n"));
        let FetchResult::Modified { body, .. } = result else {
            panic!("expected response body")
        };
        let parsed = crate::subscription::parse_subscription(&body).expect("parse fetched Clash");
        assert!(parsed.skipped.is_empty());
        let nodes = crate::subscription::normalize_nodes(
            crate::domain::ProviderId("fixture-provider".to_owned()),
            parsed.nodes,
        )
        .expect("normalize fetched nodes");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "Remote Clash");
        assert_eq!(nodes[0].provider_id.0, "fixture-provider");
    }

    #[test]
    fn custom_user_agent_is_sent_without_changing_the_bounded_fetch_contract() {
        for user_agent in ["sing-box/1.14.0", "Clash.Meta", "custom-client/1"] {
            let (url, request, server) = serve_once(
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_owned(),
            );
            let result = tauri::async_runtime::block_on(fetch_subscription_with_options(
                &url,
                ConditionalHeaders::default(),
                &FetchClientOptions {
                    user_agent: Some(user_agent.to_owned()),
                    ..FetchClientOptions::default()
                },
            ));
            server.join().expect("fixture exit");

            assert!(matches!(result, Ok(FetchResult::Modified { .. })));
            assert!(
                request
                    .lock()
                    .expect("request lock")
                    .contains(&format!("user-agent: {user_agent}\r\n"))
            );
        }
    }

    #[test]
    fn accepts_only_https_or_credential_free_literal_loopback_http() {
        assert_eq!(
            validate_source_url("https://example.invalid/sub?token=x"),
            Ok(())
        );
        assert_eq!(validate_source_url("http://127.0.0.1:8080/sub"), Ok(()));
        assert_eq!(validate_source_url("http://[::1]:8080/sub"), Ok(()));
        assert_eq!(
            validate_source_url("http://example.invalid/sub"),
            Err(FetchError::InvalidUrl)
        );
        assert_eq!(
            validate_source_url("https://user:secret@example.invalid/sub"),
            Err(FetchError::InvalidUrl)
        );
        assert_eq!(
            validate_source_url("http://127.0.0.1/sub?token=x"),
            Err(FetchError::InvalidUrl)
        );
    }

    #[test]
    fn userinfo_is_typed_and_invalid_or_duplicate_numbers_are_discarded() {
        assert_eq!(
            parse_userinfo("upload=1; download=2; total=10; expire=3"),
            Some(SubscriptionTraffic {
                upload: Some(1),
                download: Some(2),
                total: Some(10),
                expire_at_ms: Some(3_000),
            })
        );
        assert_eq!(parse_userinfo("upload=secret"), None);
        assert_eq!(parse_userinfo("upload=1; upload=2"), None);
    }

    #[test]
    fn content_disposition_keeps_only_a_bounded_basename() {
        assert_eq!(
            parse_filename("attachment; filename=../fixture.yaml"),
            Some("fixture.yaml".to_owned())
        );
        assert_eq!(parse_filename("attachment"), None);
    }

    #[test]
    fn profile_update_interval_accepts_only_whole_hours_at_or_above_one_day() {
        let mut headers = HeaderMap::new();
        headers.insert("profile-update-interval", "24".parse().expect("header"));
        assert_eq!(profile_update_interval_minutes(&headers), Some(1_440));
        headers.insert("profile-update-interval", "48".parse().expect("header"));
        assert_eq!(profile_update_interval_minutes(&headers), Some(2_880));
        for invalid in ["23", "1.5", "-24", "99999999999999999999"] {
            headers.insert(
                "profile-update-interval",
                invalid.parse().expect("valid header syntax"),
            );
            assert_eq!(profile_update_interval_minutes(&headers), None);
        }
    }

    #[test]
    fn loopback_fetch_reads_a_bounded_200_body_and_typed_headers() {
        let (url, request, server) = serve_once(
            "HTTP/1.1 200 OK\r\nETag: \"v1\"\r\nSubscription-Userinfo: upload=1; download=2; total=10; expire=3\r\nContent-Disposition: attachment; filename=fixture.yaml\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_owned(),
        );
        let result = tauri::async_runtime::block_on(fetch_subscription(
            &build_client().expect("client"),
            &url,
            ConditionalHeaders::default(),
        ))
        .expect("loopback fetch");
        server.join().expect("server exits");

        assert_eq!(
            result,
            FetchResult::Modified {
                body: "{}".to_owned(),
                metadata: SubscriptionHttpMetadata {
                    etag: Some("\"v1\"".to_owned()),
                    last_modified: None,
                    subscription_userinfo: Some(SubscriptionTraffic {
                        upload: Some(1),
                        download: Some(2),
                        total: Some(10),
                        expire_at_ms: Some(3_000),
                    }),
                    content_disposition: Some("fixture.yaml".to_owned()),
                },
                profile_update_interval_minutes: None,
                validators_match_source: true,
            }
        );
        assert!(
            !request
                .lock()
                .expect("request lock")
                .contains("If-None-Match")
        );
    }

    #[test]
    fn conditional_304_returns_no_body_and_sends_the_current_validator() {
        let (url, request, server) = serve_once(
            "HTTP/1.1 304 Not Modified\r\nETag: \"v1\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_owned(),
        );
        let result = tauri::async_runtime::block_on(fetch_subscription(
            &build_client().expect("client"),
            &url,
            ConditionalHeaders {
                etag: Some("\"v1\"".to_owned()),
                last_modified: None,
            },
        ))
        .expect("304 response");
        server.join().expect("server exits");

        assert!(matches!(result, FetchResult::NotModified { .. }));
        assert!(
            request
                .lock()
                .expect("request lock")
                .contains("if-none-match: \"v1\"")
        );
    }

    #[test]
    fn redirects_are_revalidated_before_a_second_request() {
        let (url, _, server) = serve_once(
            "HTTP/1.1 302 Found\r\nLocation: http://example.invalid/sub\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_owned(),
        );
        let result = tauri::async_runtime::block_on(fetch_subscription(
            &build_client().expect("client"),
            &url,
            ConditionalHeaders::default(),
        ));
        server.join().expect("server exits");

        assert_eq!(result, Err(FetchError::InvalidUrl));
        let downgrade = Url::parse("http://127.0.0.1/sub").expect("URL");
        assert_eq!(
            validate_url(&downgrade, Some("https")),
            Err(FetchError::InvalidUrl)
        );
    }

    #[test]
    fn accumulated_body_limit_does_not_trust_content_length() {
        let body = "x".repeat(MAX_BODY_BYTES + 1);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let (url, _, server) = serve_once(response);
        let result = tauri::async_runtime::block_on(fetch_subscription(
            &build_client().expect("client"),
            &url,
            ConditionalHeaders::default(),
        ));
        server.join().expect("server exits");

        assert_eq!(result, Err(FetchError::BodyTooLarge));
    }

    #[test]
    fn total_timeout_cancels_a_stalled_loopback_response() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind stalled fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut bytes = [0_u8; 8_192];
            let _ = stream.read(&mut bytes).expect("read request");
            thread::sleep(Duration::from_millis(150));
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
        });

        let result = tauri::async_runtime::block_on(fetch_subscription_with_timeout(
            &build_client().expect("client"),
            &format!("http://{address}/subscription"),
            ConditionalHeaders::default(),
            Duration::from_millis(40),
        ));
        server.join().expect("server exits");

        assert_eq!(result, Err(FetchError::RequestFailed));
    }

    #[test]
    fn cross_origin_validators_are_neither_forwarded_nor_returned_for_the_next_update() {
        let source_listener = TcpListener::bind("127.0.0.1:0").expect("bind redirect source");
        let source_address = source_listener.local_addr().expect("source address");
        let target_listener = TcpListener::bind("127.0.0.1:0").expect("bind redirect target");
        let target_address = target_listener.local_addr().expect("target address");
        let source_requests = Arc::new(Mutex::new(Vec::new()));
        let captured_source = Arc::clone(&source_requests);
        let source_server = thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = source_listener.accept().expect("accept redirect source");
                let mut bytes = [0_u8; 8_192];
                let count = stream.read(&mut bytes).expect("read source request");
                captured_source
                    .lock()
                    .expect("source request lock")
                    .push(String::from_utf8_lossy(&bytes[..count]).into_owned());
                let response = format!(
                    "HTTP/1.1 302 Found\r\nLocation: http://{target_address}/target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write source response");
            }
        });
        let target_requests = Arc::new(Mutex::new(Vec::new()));
        let captured_target = Arc::clone(&target_requests);
        let target_server = thread::spawn(move || {
            for response in [
                "HTTP/1.1 200 OK\r\nETag: \"target-v1\"\r\nLast-Modified: Wed, 21 Oct 2015 07:28:00 GMT\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                "HTTP/1.1 304 Not Modified\r\nETag: \"target-v2\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ] {
                let (mut stream, _) = target_listener.accept().expect("accept redirect target");
                let mut bytes = [0_u8; 8_192];
                let count = stream.read(&mut bytes).expect("read target request");
                captured_target
                    .lock()
                    .expect("target request lock")
                    .push(String::from_utf8_lossy(&bytes[..count]).into_owned());
                stream
                    .write_all(response.as_bytes())
                    .expect("write target response");
            }
        });

        let source_url = format!("http://{source_address}/subscription");
        let first = tauri::async_runtime::block_on(fetch_subscription(
            &build_client().expect("client"),
            &source_url,
            ConditionalHeaders {
                etag: Some("\"source-v1\"".to_owned()),
                last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_owned()),
            },
        ))
        .expect("first redirect fetch");
        let FetchResult::Modified {
            metadata,
            validators_match_source,
            ..
        } = first
        else {
            panic!("expected first body")
        };
        assert!(!validators_match_source);
        assert_eq!(metadata.etag, None);
        assert_eq!(metadata.last_modified, None);

        let second = tauri::async_runtime::block_on(fetch_subscription(
            &build_client().expect("client"),
            &source_url,
            ConditionalHeaders {
                etag: metadata.etag,
                last_modified: metadata.last_modified,
            },
        ))
        .expect("second redirect fetch");
        source_server.join().expect("source server exits");
        target_server.join().expect("target server exits");

        let FetchResult::NotModified {
            metadata,
            validators_match_source,
            ..
        } = second
        else {
            panic!("expected second 304")
        };
        assert!(!validators_match_source);
        assert_eq!(metadata.etag, None);
        let source_requests = source_requests.lock().expect("source requests lock");
        assert!(source_requests[0].contains("if-none-match: \"source-v1\""));
        assert!(source_requests[0].contains("if-modified-since:"));
        assert!(!source_requests[1].contains("if-none-match"));
        assert!(!source_requests[1].contains("if-modified-since"));
        let target_requests = target_requests.lock().expect("target requests lock");
        for request in &*target_requests {
            assert!(!request.contains("if-none-match"));
            assert!(!request.contains("if-modified-since"));
        }
    }

    fn serve_once(response: String) -> (String, Arc<Mutex<String>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback fixture");
        let address = listener.local_addr().expect("fixture address");
        let captured = Arc::new(Mutex::new(String::new()));
        let request = Arc::clone(&captured);
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut bytes = [0_u8; 8_192];
            let count = stream.read(&mut bytes).expect("read request");
            *request.lock().expect("request lock") =
                String::from_utf8_lossy(&bytes[..count]).into_owned();
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });
        (format!("http://{address}/subscription"), captured, server)
    }
}
