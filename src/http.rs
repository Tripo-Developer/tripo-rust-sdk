//! Thin `reqwest`-based HTTP layer used by [`crate::TripoClient`].
//!
//! Responsibilities:
//!   - Attach `Authorization: Bearer …` header.
//!   - Retry on transient errors with exponential back-off (honoring
//!     `Retry-After` when present), but only when the server cannot have
//!     processed the request: non-idempotent calls (every task-creation
//!     POST) are never replayed once they may have landed, since those are
//!     billed per submission.
//!   - Parse the standard `{ code, data, message, suggestion }` envelope and
//!     surface [`crate::Error::Api`] / [`crate::Error::Request`] as needed.

use crate::error::{Error, Result};
use crate::models::Envelope;
use rand::Rng;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use std::time::Duration;

/// What a failure tells us about whether the server acted on the request.
///
/// Task-creation endpoints are billed per submission, so replaying a request
/// that may already have been processed can charge the caller twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetrySafety {
    /// Non-transient failure: never retry.
    Fatal,
    /// The request may or may not have been processed. Only idempotent
    /// methods may be retried.
    Unknown,
    /// The server provably never acted on the request, so a retry is safe
    /// regardless of method.
    Clean,
}

/// Statuses where the server answered and told us it declined to do the work.
const CLEAN_RETRY_STATUSES: &[u16] = &[429, 503];

/// Statuses where the server answered but whether it processed the request is
/// unknowable — a 504 in particular is often emitted by a proxy after the
/// origin already accepted the work.
const UNKNOWN_RETRY_STATUSES: &[u16] = &[408, 425, 500, 502, 504];

const INDETERMINATE_HINT: &str =
    "the server may already have accepted this request, so it was not retried \
     automatically; check your task list before resubmitting to avoid being billed twice";

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout: Duration,
    pub retries: u32,
    pub user_agent: String,
}

#[derive(Clone)]
pub(crate) struct HttpClient {
    inner: reqwest::Client,
    config: HttpConfig,
}

/// Options for a single request, layered on top of the client defaults.
#[derive(Default)]
pub(crate) struct RequestOptions<'a> {
    pub json: Option<&'a serde_json::Value>,
    pub retries: Option<u32>,
}

impl HttpClient {
    pub fn new(config: HttpConfig) -> Result<Self> {
        let inner = reqwest::Client::builder()
            .user_agent(config.user_agent.clone())
            .build()
            .map_err(|e| Error::Request {
                message: format!("failed to build HTTP client: {e}"),
                status: None,
                body: None,
                source: Some(e),
                indeterminate: false,
            })?;
        Ok(Self { inner, config })
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!(
                "{}/{}",
                self.config.base_url.trim_end_matches('/'),
                path.trim_start_matches('/')
            )
        }
    }

    /// Perform a request and deserialize the `data` field of the standard
    /// envelope into `T`.
    pub async fn request_json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        opts: RequestOptions<'_>,
    ) -> Result<T> {
        let idempotent = is_idempotent_method(&method);
        let response = self.execute(method, path, opts).await?;
        let status = response.status().as_u16();
        // The server already answered, so it has done the work; only the
        // read-back failed.
        let bytes = response.bytes().await.map_err(|e| Error::Request {
            message: if idempotent {
                format!("failed to read response body: {e}")
            } else {
                format!("failed to read response body: {e}; {INDETERMINATE_HINT}")
            },
            status: Some(status),
            body: None,
            source: Some(e),
            indeterminate: !idempotent,
        })?;
        parse_envelope(&bytes, Some(status))
    }

    /// Perform a `multipart/form-data` request (used by file upload).
    pub async fn request_multipart<T: DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<T> {
        let url = self.url(path);
        let response = self
            .inner
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .multipart(form)
            .timeout(self.config.timeout)
            .send()
            .await
            .map_err(|e| {
                // Upload is a POST and is never retried here, but the caller
                // still needs to know whether the upload may have landed.
                let indeterminate = classify_transport_err(&e) == RetrySafety::Unknown;
                Error::Request {
                    message: if indeterminate {
                        format!("network error: {e}; {INDETERMINATE_HINT}")
                    } else {
                        format!("network error: {e}")
                    },
                    status: None,
                    body: None,
                    source: Some(e),
                    indeterminate,
                }
            })?;
        let status = response.status().as_u16();
        let bytes = response.bytes().await.map_err(|e| Error::Request {
            message: format!("failed to read response body: {e}; {INDETERMINATE_HINT}"),
            status: Some(status),
            body: None,
            source: Some(e),
            indeterminate: true,
        })?;
        parse_envelope(&bytes, Some(status))
    }

    /// Fetch a raw resource (e.g. downloading a model URL) without envelope
    /// parsing. Returns the raw bytes and the `Content-Type` header.
    pub async fn get_raw(&self, url: &str) -> Result<(Vec<u8>, Option<String>)> {
        let response = self
            .inner
            .get(url)
            .timeout(self.config.timeout)
            .send()
            .await
            .map_err(|e| Error::Request {
                message: format!("network error: {e}"),
                status: None,
                body: None,
                source: Some(e),
                indeterminate: false,
            })?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.ok();
            return Err(Error::Request {
                message: format!("download failed with HTTP {status}"),
                status: Some(status),
                body,
                source: None,
                indeterminate: false,
            });
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let bytes = response.bytes().await.map_err(|e| Error::Request {
            message: format!("failed to read response body: {e}"),
            status: None,
            body: None,
            source: Some(e),
            indeterminate: false,
        })?;
        Ok((bytes.to_vec(), content_type))
    }

    /// PUT `size` bytes to a presigned storage URL. The URL carries its own
    /// signature, so no `Authorization` header is sent: storage rejects
    /// requests with a second credential. There is no timeout, since large
    /// uploads can legitimately take long, and no retry, since a streamed
    /// body cannot be replayed.
    pub async fn put_raw(&self, url: &str, body: reqwest::Body, size: u64) -> Result<()> {
        let response = self
            .inner
            .put(url)
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .header(reqwest::header::CONTENT_LENGTH, size)
            .body(body)
            .send()
            .await
            .map_err(|e| Error::Request {
                message: format!("upload to storage failed: {e}"),
                status: None,
                body: None,
                source: Some(e),
                indeterminate: false,
            })?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.ok();
            return Err(Error::Request {
                message: format!("upload to storage failed with HTTP {status}"),
                status: Some(status),
                body,
                source: None,
                indeterminate: false,
            });
        }
        Ok(())
    }

    async fn execute(
        &self,
        method: Method,
        path: &str,
        opts: RequestOptions<'_>,
    ) -> Result<reqwest::Response> {
        let url = self.url(path);
        let total_attempts = opts.retries.unwrap_or(self.config.retries) + 1;
        let idempotent = is_idempotent_method(&method);

        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let mut builder = self
                .inner
                .request(method.clone(), &url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .timeout(self.config.timeout);
            if let Some(json) = opts.json {
                builder = builder.json(json);
            }

            let result = builder.send().await;
            match result {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return Ok(response);
                    }
                    let safety = classify_status(status);
                    if can_retry(safety, idempotent) && attempt < total_attempts {
                        let retry_after = response
                            .headers()
                            .get(reqwest::header::RETRY_AFTER)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<f64>().ok());
                        tokio::time::sleep(backoff(attempt, retry_after)).await;
                        continue;
                    }
                    if safety == RetrySafety::Unknown && !idempotent {
                        let code = status.as_u16();
                        let body = response.text().await.ok();
                        return Err(Error::Request {
                            message: format!("HTTP {code}; {INDETERMINATE_HINT}"),
                            status: Some(code),
                            body,
                            source: None,
                            indeterminate: true,
                        });
                    }
                    // Not retryable (or attempts exhausted): let the caller's
                    // envelope parser decide whether this is Error::Api or
                    // Error::Request based on the body shape.
                    return Ok(response);
                }
                Err(e) => {
                    let safety = classify_transport_err(&e);
                    if can_retry(safety, idempotent) && attempt < total_attempts {
                        tokio::time::sleep(backoff(attempt, None)).await;
                        continue;
                    }
                    let indeterminate = safety == RetrySafety::Unknown && !idempotent;
                    let message = if indeterminate {
                        format!("network error: {e}; {INDETERMINATE_HINT}")
                    } else {
                        format!("network error: {e}")
                    };
                    return Err(Error::Request {
                        message,
                        status: None,
                        body: None,
                        source: Some(e),
                        indeterminate,
                    });
                }
            }
        }
    }
}

fn is_idempotent_method(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn can_retry(safety: RetrySafety, idempotent: bool) -> bool {
    match safety {
        RetrySafety::Clean => true,
        RetrySafety::Unknown => idempotent,
        RetrySafety::Fatal => false,
    }
}

fn classify_status(status: StatusCode) -> RetrySafety {
    let code = status.as_u16();
    if CLEAN_RETRY_STATUSES.contains(&code) {
        RetrySafety::Clean
    } else if UNKNOWN_RETRY_STATUSES.contains(&code) {
        RetrySafety::Unknown
    } else {
        RetrySafety::Fatal
    }
}

/// Decide how much a transport error tells us about whether the request
/// reached the server's handler.
fn classify_transport_err(e: &reqwest::Error) -> RetrySafety {
    // `is_connect` covers DNS failures and refused/unreachable peers, none of
    // which ever delivered the request.
    if e.is_connect() {
        return RetrySafety::Clean;
    }
    // Timeouts and resets can land after the server has already read and
    // acted on the request.
    if e.is_timeout() || e.is_request() || e.is_body() {
        return RetrySafety::Unknown;
    }
    RetrySafety::Fatal
}

fn backoff(attempt: u32, retry_after_secs: Option<f64>) -> Duration {
    if let Some(secs) = retry_after_secs {
        if secs.is_finite() && secs >= 0.0 {
            return Duration::from_millis((secs * 1000.0).min(30_000.0) as u64);
        }
    }
    let base_ms = 1000u64
        .saturating_mul(1 << attempt.saturating_sub(1))
        .min(8000);
    let jitter_ms: u64 = rand::thread_rng().gen_range(0..250);
    Duration::from_millis(base_ms + jitter_ms)
}

/// Parse the standard `{ code, data, message, suggestion }` envelope,
/// returning `data` on success (`code == 0`) or [`Error::Api`] otherwise.
fn parse_envelope<T: DeserializeOwned>(bytes: &[u8], status: Option<u16>) -> Result<T> {
    if bytes.is_empty() {
        return Err(Error::Request {
            message: "empty response body".to_string(),
            status,
            body: None,
            source: None,
            indeterminate: false,
        });
    }
    let envelope: Envelope<T> = serde_json::from_slice(bytes).map_err(|e| {
        let raw = String::from_utf8_lossy(bytes).to_string();
        Error::Request {
            message: format!("malformed response: {e}"),
            status,
            body: Some(raw),
            source: None,
            indeterminate: false,
        }
    })?;
    if envelope.code != 0 {
        return Err(Error::Api {
            code: envelope.code,
            message: envelope.message,
            suggestion: envelope.suggestion,
            status,
            request_id: envelope.request_id,
        });
    }
    envelope.data.ok_or_else(|| Error::Request {
        message: "response envelope had code=0 but no `data` field".to_string(),
        status,
        body: None,
        source: None,
        indeterminate: false,
    })
}
