//! Webhook signature verification.
//!
//! ```no_run
//! use tripo3d_sdk::webhook::{self, WebhookOptions, SIGNATURE_HEADER};
//!
//! # fn handle(body: &[u8], headers: &std::collections::HashMap<String, String>) -> tripo3d_sdk::Result<()> {
//! let secret = std::env::var("TRIPO_WEBHOOK_SECRET").unwrap();
//! let signature = headers.get(SIGNATURE_HEADER).map(String::as_str).unwrap_or("");
//! let event = webhook::verify_webhook(body, signature, &secret, WebhookOptions::default())?;
//! if event.event_type == webhook::TASK_COMPLETED {
//!     println!("{:?}", event.task()?.primary_model_url());
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use crate::models::Task;
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Carries `t=<unix seconds>,v1=<hex HMAC>`.
pub const SIGNATURE_HEADER: &str = "Tripo-Webhook-Signature";
/// Unique per delivery; use it to drop duplicates, since failed deliveries
/// are retried.
pub const DELIVERY_HEADER: &str = "Tripo-Webhook-Delivery";
/// Repeats [`WebhookEvent::event_type`].
pub const EVENT_HEADER: &str = "Tripo-Webhook-Event";
/// Identifies the endpoint configured in the console.
pub const ID_HEADER: &str = "Tripo-Webhook-Id";

pub const TASK_COMPLETED: &str = "task.completed";
pub const TASK_FAILED: &str = "task.failed";
pub const BALANCE_LOW: &str = "balance.low";

/// How far the signature timestamp may drift from now by default.
pub const DEFAULT_TOLERANCE: Duration = Duration::from_secs(300);

/// Options for [`verify_webhook`].
#[derive(Debug, Clone, Copy, Default)]
pub struct WebhookOptions {
    /// Accepted clock drift; `None` means [`DEFAULT_TOLERANCE`]. Pass
    /// `Some(Duration::MAX)` to skip the replay check.
    pub tolerance: Option<Duration>,
    /// Overrides the current time, for tests.
    pub now: Option<SystemTime>,
}

/// The JSON body of a webhook delivery.
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookEvent {
    /// The task_id and event type, e.g. `"<task_id>:task.completed"`.
    pub id: String,
    /// [`TASK_COMPLETED`], [`TASK_FAILED`] or [`BALANCE_LOW`].
    #[serde(rename = "type")]
    pub event_type: String,
    /// ISO 8601 timestamp.
    pub created_at: String,
    /// The task snapshot for task events; `balance` and `threshold` for
    /// [`BALANCE_LOW`].
    pub data: Value,
}

impl WebhookEvent {
    /// Decode [`WebhookEvent::data`] as a task snapshot. Only meaningful
    /// for [`TASK_COMPLETED`] and [`TASK_FAILED`].
    pub fn task(&self) -> Result<Task> {
        Ok(Task::deserialize(&self.data)?)
    }
}

/// Check that `payload`, the raw and unmodified request body, was signed by
/// Tripo with the endpoint's signing secret (`whsec_…`), then decode it.
/// `signature_header` is the value of [`SIGNATURE_HEADER`].
///
/// Verify before acting on the event, and pass the body exactly as
/// received: re-serialising parsed JSON changes the bytes and breaks the
/// signature. Failures return [`Error::WebhookSignature`]; respond with an
/// HTTP 4xx.
pub fn verify_webhook(
    payload: &[u8],
    signature_header: &str,
    secret: &str,
    opts: WebhookOptions,
) -> Result<WebhookEvent> {
    if secret.is_empty() {
        return Err(Error::invalid_argument(
            "verify_webhook: `secret` is required",
        ));
    }
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in signature_header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", value)) => timestamp = Some(value),
            Some(("v1", value)) => signatures.extend(decode_hex(value)),
            _ => {}
        }
    }
    let (Some(timestamp), false) = (timestamp, signatures.is_empty()) else {
        return Err(Error::WebhookSignature(format!(
            "malformed {SIGNATURE_HEADER} header"
        )));
    };
    let unix: u64 = timestamp
        .parse()
        .map_err(|_| Error::WebhookSignature(format!("malformed {SIGNATURE_HEADER} header")))?;

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(payload);
    if !signatures
        .iter()
        .any(|sig| mac.clone().verify_slice(sig).is_ok())
    {
        return Err(Error::WebhookSignature("signature mismatch".into()));
    }

    let tolerance = opts.tolerance.unwrap_or(DEFAULT_TOLERANCE);
    let now = opts
        .now
        .unwrap_or_else(SystemTime::now)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let drift = now.abs_diff(Duration::from_secs(unix));
    if drift > tolerance {
        return Err(Error::WebhookSignature(format!(
            "timestamp is {}s away from now",
            drift.as_secs()
        )));
    }

    Ok(serde_json::from_slice(payload)?)
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}
