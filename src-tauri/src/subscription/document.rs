use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crate::domain::{MAX_SUBSCRIPTION_DOCUMENT_BYTES, SubscriptionDocumentFormat};

use super::{ParseResult, SubscriptionFormat, parse_subscription};

const DOCUMENT_WORK_TIMEOUT: Duration = Duration::from_secs(5);
static DOCUMENT_WORK_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DocumentErrorCode {
    InvalidInput,
    ContentTooLarge,
    FormatFailed,
    ParseFailed,
    UnsupportedNodes,
    UnsupportedClashProviders,
    Busy,
    OperationTimedOut,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentErrorLocation {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DocumentError {
    pub(crate) code: DocumentErrorCode,
    pub(crate) location: Option<DocumentErrorLocation>,
}

impl DocumentError {
    fn new(code: DocumentErrorCode) -> Self {
        Self {
            code,
            location: None,
        }
    }

    fn at(code: DocumentErrorCode, line: usize, column: usize) -> Self {
        Self {
            code,
            location: Some(DocumentErrorLocation { line, column }),
        }
    }
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self.code {
            DocumentErrorCode::InvalidInput => "document input is invalid",
            DocumentErrorCode::ContentTooLarge => "document content is too large",
            DocumentErrorCode::FormatFailed => "document formatting failed",
            DocumentErrorCode::ParseFailed => "document parsing failed",
            DocumentErrorCode::UnsupportedNodes => "document contains unsupported nodes",
            DocumentErrorCode::UnsupportedClashProviders => {
                "provider-only Clash documents are unsupported"
            }
            DocumentErrorCode::Busy => "document formatter is busy",
            DocumentErrorCode::OperationTimedOut => "document operation timed out",
        })
    }
}

impl std::error::Error for DocumentError {}

pub(crate) struct ExactDocumentParse {
    pub(crate) content: String,
    pub(crate) parsed: ParseResult,
}

struct DocumentWorkGuard;

impl Drop for DocumentWorkGuard {
    fn drop(&mut self) {
        DOCUMENT_WORK_IN_PROGRESS.store(false, Ordering::Release);
    }
}

pub(crate) fn format_document(
    content: String,
    format: SubscriptionDocumentFormat,
) -> Result<String, DocumentError> {
    validate_input_size(&content)?;
    run_bounded(move || {
        let formatted = match format {
            SubscriptionDocumentFormat::Json => {
                let value =
                    serde_json::from_str::<serde_json::Value>(&content).map_err(|error| {
                        DocumentError::at(
                            DocumentErrorCode::FormatFailed,
                            error.line(),
                            error.column(),
                        )
                    })?;
                serde_json::to_string_pretty(&value)
                    .map_err(|_| DocumentError::new(DocumentErrorCode::FormatFailed))?
            }
            SubscriptionDocumentFormat::Yaml => {
                let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&content)
                    .map_err(yaml_format_error)?;
                serde_yaml_ng::to_string(&value)
                    .map_err(|_| DocumentError::new(DocumentErrorCode::FormatFailed))?
            }
        };
        validate_input_size(&formatted)?;
        Ok(formatted)
    })?
}

pub(crate) fn parse_exact_document(
    content: String,
    format: SubscriptionDocumentFormat,
) -> Result<ExactDocumentParse, DocumentError> {
    validate_input_size(&content)?;
    run_bounded(move || {
        let syntax = match format {
            SubscriptionDocumentFormat::Json => serde_json::from_str::<serde_json::Value>(&content)
                .map_err(|error| {
                    DocumentError::at(
                        DocumentErrorCode::FormatFailed,
                        error.line(),
                        error.column(),
                    )
                })?,
            SubscriptionDocumentFormat::Yaml => {
                let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&content)
                    .map_err(yaml_format_error)?;
                serde_json::to_value(value)
                    .map_err(|_| DocumentError::new(DocumentErrorCode::FormatFailed))?
            }
        };
        if is_provider_only_clash(&syntax) {
            return Err(DocumentError::new(
                DocumentErrorCode::UnsupportedClashProviders,
            ));
        }
        let parsed = parse_subscription(&content)
            .map_err(|_| DocumentError::new(DocumentErrorCode::ParseFailed))?;
        let format_matches = matches!(
            (format, parsed.format),
            (SubscriptionDocumentFormat::Json, SubscriptionFormat::Json)
                | (
                    SubscriptionDocumentFormat::Yaml,
                    SubscriptionFormat::ClashYaml
                )
        );
        if !format_matches || parsed.nodes.is_empty() {
            return Err(DocumentError::new(DocumentErrorCode::ParseFailed));
        }
        if !parsed.skipped.is_empty() {
            return Err(DocumentError::new(DocumentErrorCode::UnsupportedNodes));
        }
        Ok(ExactDocumentParse { content, parsed })
    })?
}

fn is_provider_only_clash(document: &serde_json::Value) -> bool {
    let has_external = document
        .get("proxy-providers")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|values| !values.is_empty());
    let proxies = document
        .get("proxies")
        .and_then(serde_json::Value::as_array);
    has_external && proxies.is_none_or(Vec::is_empty)
}

fn validate_input_size(content: &str) -> Result<(), DocumentError> {
    if content.is_empty() {
        return Err(DocumentError::new(DocumentErrorCode::InvalidInput));
    }
    if content.len() > MAX_SUBSCRIPTION_DOCUMENT_BYTES {
        return Err(DocumentError::new(DocumentErrorCode::ContentTooLarge));
    }
    Ok(())
}

fn run_bounded<T, F>(work: F) -> Result<T, DocumentError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    DOCUMENT_WORK_IN_PROGRESS
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| DocumentError::new(DocumentErrorCode::Busy))?;
    let (sender, receiver) = mpsc::sync_channel(1);
    if std::thread::Builder::new()
        .name("veyra-subscription-document".to_owned())
        .spawn(move || {
            let guard = DocumentWorkGuard;
            let result = work();
            drop(guard);
            let _ = sender.send(result);
        })
        .is_err()
    {
        DOCUMENT_WORK_IN_PROGRESS.store(false, Ordering::Release);
        return Err(DocumentError::new(DocumentErrorCode::FormatFailed));
    }
    receiver
        .recv_timeout(DOCUMENT_WORK_TIMEOUT)
        .map_err(|error| {
            if matches!(error, mpsc::RecvTimeoutError::Timeout) {
                DocumentError::new(DocumentErrorCode::OperationTimedOut)
            } else {
                DocumentError::new(DocumentErrorCode::FormatFailed)
            }
        })
}

fn yaml_format_error(error: serde_yaml_ng::Error) -> DocumentError {
    error.location().map_or_else(
        || DocumentError::new(DocumentErrorCode::FormatFailed),
        |location| {
            DocumentError::at(
                DocumentErrorCode::FormatFailed,
                location.line(),
                location.column(),
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_format_canonicalizes_but_exact_parse_keeps_comments() {
        let source = "# keep this\nproxies:\n  - name: test\n    type: socks5\n    server: example.invalid\n    port: 1080\n";
        let parsed = parse_exact_document(source.to_owned(), SubscriptionDocumentFormat::Yaml)
            .expect("valid exact document");
        assert_eq!(parsed.parsed.nodes.len(), 1);
        assert_eq!(parsed.content, source);

        let formatted = format_document(source.to_owned(), SubscriptionDocumentFormat::Yaml)
            .expect("explicit format");
        assert!(!formatted.contains("# keep this"));
        assert!(source.contains("# keep this"));
    }

    #[test]
    fn strict_parse_rejects_one_unsupported_node_in_a_mixed_document() {
        let source = r#"
proxies:
  - name: accepted
    type: socks5
    server: accepted.invalid
    port: 1080
  - name: rejected
    type: unsupported
    server: rejected.invalid
    port: 443
"#;
        assert!(matches!(
            parse_exact_document(source.to_owned(), SubscriptionDocumentFormat::Yaml),
            Err(DocumentError {
                code: DocumentErrorCode::UnsupportedNodes,
                location: None
            })
        ));
    }

    #[test]
    fn declared_format_and_size_are_strict() {
        let invalid = format_document("{".to_owned(), SubscriptionDocumentFormat::Json)
            .expect_err("invalid JSON");
        assert_eq!(invalid.code, DocumentErrorCode::FormatFailed);
        assert!(invalid.location.is_some());

        assert!(matches!(
            parse_exact_document(
                r#"{"outbounds":[]}"#.to_owned(),
                SubscriptionDocumentFormat::Yaml,
            ),
            Err(DocumentError {
                code: DocumentErrorCode::ParseFailed,
                location: None
            })
        ));
        assert_eq!(
            format_document(
                "x".repeat(MAX_SUBSCRIPTION_DOCUMENT_BYTES + 1),
                SubscriptionDocumentFormat::Yaml,
            ),
            Err(DocumentError::new(DocumentErrorCode::ContentTooLarge))
        );

        let providers_only =
            "proxy-providers:\n  remote:\n    type: http\n    url: https://fixture.invalid/sub\n";
        assert!(matches!(
            parse_exact_document(providers_only.to_owned(), SubscriptionDocumentFormat::Yaml),
            Err(DocumentError {
                code: DocumentErrorCode::UnsupportedClashProviders,
                location: None
            })
        ));
    }
}
