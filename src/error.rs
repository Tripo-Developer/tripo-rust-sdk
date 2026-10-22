//! Error hierarchy for the Tripo3D SDK.
//!
//!   Error
//!   ├── Request   — transport/network/HTTP-status failures
//!   ├── Api       — well-formed error responses (non-zero `code`)
//!   ├── Task      — task ended in `failed` / `cancelled` / `banned` / `expired`
//!   └── Timeout   — `wait_for_task` exceeded its timeout budget

use crate::models::Task;
use thiserror::Error;

/// Top-level error type returned by all fallible `TripoClient` methods.
#[derive(Debug, Error)]
pub enum Error {
    /// Transport-level failure: network error, non-2xx status without a
    /// parseable `{code, message}` envelope, or a malformed response body.
    #[error("request error: {message}{}", status.map(|s| format!(" (HTTP {s})")).unwrap_or_default())]
    Request {
        message: String,
        status: Option<u16>,
        body: Option<String>,
        #[source]
        source: Option<reqwest::Error>,
        /// True when the request may have been processed by the server
        /// despite the failure. Resubmitting a billed task-creation request
        /// in this state risks being charged twice; reconcile against
        /// [`crate::TripoClient::list_tasks`] first.
        indeterminate: bool,
    },

    /// A well-formed `{ code, message, suggestion }` error envelope with a
    /// non-zero `code`.
    #[error("Tripo API error (code={code}){}{}{}", message.as_deref().map(|m| format!(": {m}")).unwrap_or_default(), suggestion.as_deref().map(|s| format!(" — {s}")).unwrap_or_default(), request_id.as_deref().map(|r| format!(" (request_id={r})")).unwrap_or_default())]
    Api {
        code: i64,
        message: Option<String>,
        suggestion: Option<String>,
        status: Option<u16>,
        /// Identifies the failed request; quote it when contacting Tripo3D
        /// support.
        request_id: Option<String>,
    },

    /// A task reached a non-successful terminal state
    /// (`failed` / `cancelled` / `banned` / `expired`).
    #[error("task {} ended with status \"{}\"{}", task.task_id, task.status, task.error_message.as_deref().map(|m| format!(": {m}")).unwrap_or_default())]
    Task { task: Box<Task> },

    /// `wait_for_task` exceeded the caller-supplied timeout.
    #[error("timed out after {timeout_ms}ms waiting for task {task_id}")]
    Timeout { task_id: String, timeout_ms: u64 },

    /// A caller-supplied argument was invalid (e.g. missing required field).
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// Local I/O failure (e.g. reading a file to upload).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization failure.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl Error {
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Error::InvalidArgument(msg.into())
    }

    /// The task associated with a `Error::Task` variant, if any.
    pub fn task(&self) -> Option<&Task> {
        match self {
            Error::Task { task } => Some(task),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
