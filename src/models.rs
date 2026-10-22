//! Data models shared across the Tripo3D v3 API surface.

use crate::constants::TaskStatus;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// The standard `{ code, data, message, suggestion }` response envelope.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Envelope<T> {
    pub code: i64,
    // Deliberately no `#[serde(default)]` here: serde already treats a
    // missing key as `None` for `Option<T>` fields, and adding the
    // attribute would force an (unwanted) `T: Default` bound on this type.
    pub data: Option<T>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub suggestion: Option<String>,
}

/// A task snapshot as returned by `GET /v3/tasks/{task_id}` and
/// `POST /v3/tasks/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub progress: Option<u32>,
    #[serde(default)]
    pub input: Option<Value>,
    #[serde(default)]
    pub output: Option<TaskOutput>,
    /// Credits consumed, a decimal with up to two places (e.g. `48.00`).
    /// Deliberately a float: VIP discounts produce fractional values that
    /// integer parsing would truncate.
    #[serde(default)]
    pub credits_consumed: Option<f64>,
    /// ISO 8601 creation time.
    #[serde(default)]
    pub created_at: Option<String>,
    /// ISO 8601 completion time; absent until the task is terminal.
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub running_left_time: Option<i64>,
    #[serde(default)]
    pub queuing_num: Option<i64>,
    #[serde(default)]
    pub error_code: Option<i64>,
    #[serde(default)]
    pub error_message: Option<String>,
    /// Any additional fields the API returns that aren't modeled above.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// The `output` object of a completed task. Fields vary by task type, so
/// everything is optional; unknown fields land in `extra`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskOutput {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub model_url: Option<String>,
    #[serde(default)]
    pub model_urls: Option<Vec<String>>,
    #[serde(default)]
    pub base_model: Option<String>,
    #[serde(default)]
    pub pbr_model: Option<String>,
    #[serde(default)]
    pub rendered_image: Option<String>,
    #[serde(default)]
    pub rendered_image_url: Option<String>,
    /// Output of the text-to-image and image-to-image endpoints. The 3D
    /// generation endpoints also populate it with the reference image they
    /// synthesised internally.
    #[serde(default)]
    pub generated_image_url: Option<String>,
    /// UV layout preview image produced by `mesh_uv`.
    #[serde(default)]
    pub uv_render_url: Option<String>,
    #[serde(default)]
    pub riggable: Option<bool>,
    #[serde(default)]
    pub rig_type: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl TaskOutput {
    /// Best-effort extraction of "the" primary model URL, checking the
    /// common output fields in priority order.
    pub fn primary_model_url(&self) -> Option<&str> {
        self.model_url
            .as_deref()
            .or(self.model.as_deref())
            .or(self.pbr_model.as_deref())
            .or(self.base_model.as_deref())
            .or_else(|| {
                self.model_urls
                    .as_ref()
                    .and_then(|v| v.first().map(String::as_str))
            })
    }
}

impl Task {
    /// Convenience accessor mirroring [`TaskOutput::primary_model_url`].
    pub fn primary_model_url(&self) -> Option<&str> {
        self.output.as_ref().and_then(TaskOutput::primary_model_url)
    }
}

/// `POST /v3/tasks/list` response payload.
#[derive(Debug, Clone, Deserialize)]
pub struct TaskList {
    #[serde(default)]
    pub tasks: Vec<Task>,
}

/// `GET /v3/account/balance` response payload.
#[derive(Debug, Clone, Deserialize)]
pub struct Balance {
    pub balance: f64,
    #[serde(default)]
    pub frozen: Option<f64>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// `POST /v3/files` response payload.
#[derive(Debug, Clone, Deserialize)]
pub struct UploadedFile {
    pub file_token: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// A generic `{ task_id }` response payload used by every task-creation
/// endpoint.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TaskCreated {
    pub task_id: String,
}

/// Reuses the 4-view output of an earlier multiview task.
#[derive(Debug, Clone, Serialize)]
pub struct MultiviewTaskRef {
    pub task_id: String,
}

/// Bucket/key pair for pre-uploaded assets (STS-style upload).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectRef {
    pub bucket: String,
    pub key: String,
}

/// The file descriptor shape accepted by every endpoint that takes an image
/// or model file as input.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileDescriptor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<ObjectRef>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub file_type: Option<String>,
}

/// An image or model reference, in any of the shapes the v3 API accepts.
///
/// A [`FileInput::Ref`] serialises as a bare JSON string and lets the server
/// infer what it is — a public URL, a `file_token`, or the `task_id` of an
/// earlier task whose output should be reused. That inference is what makes
/// chaining tasks possible, so it is what `From<&str>` produces. Use the
/// other variants when you want to be explicit.
#[derive(Debug, Clone, Default)]
pub enum FileInput {
    /// A bare reference whose kind the server infers.
    Ref(String),
    Url(String),
    FileToken(String),
    Descriptor(FileDescriptor),
    /// An omitted view. Only meaningful in a multiview slot, where it
    /// serialises as the empty string the API uses to skip a view.
    #[default]
    Empty,
}

impl Serialize for FileInput {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match self {
            FileInput::Ref(s) => serializer.serialize_str(s),
            FileInput::Empty => serializer.serialize_str(""),
            FileInput::Url(url) => FileDescriptor {
                url: Some(url.clone()),
                ..Default::default()
            }
            .serialize(serializer),
            FileInput::FileToken(token) => FileDescriptor {
                file_token: Some(token.clone()),
                ..Default::default()
            }
            .serialize(serializer),
            FileInput::Descriptor(d) => d.serialize(serializer),
        }
    }
}

impl From<&str> for FileInput {
    fn from(value: &str) -> Self {
        FileInput::Ref(value.to_string())
    }
}

impl From<String> for FileInput {
    fn from(value: String) -> Self {
        FileInput::from(value.as_str())
    }
}

impl From<FileDescriptor> for FileInput {
    fn from(value: FileDescriptor) -> Self {
        FileInput::Descriptor(value)
    }
}

impl FileInput {
    /// Reports whether this input carries no reference at all.
    pub fn is_empty(&self) -> bool {
        match self {
            FileInput::Empty => true,
            FileInput::Ref(s) | FileInput::Url(s) | FileInput::FileToken(s) => s.is_empty(),
            FileInput::Descriptor(d) => {
                d.file_token.is_none() && d.url.is_none() && d.object.is_none()
            }
        }
    }

    /// Collapses this input into the explicit object form. A [`FileInput::Ref`]
    /// is classified by its prefix, which cannot distinguish a `file_token`
    /// from a `task_id` — prefer serialising the `FileInput` directly.
    pub fn into_descriptor(self) -> FileDescriptor {
        match self {
            FileInput::Url(url) => FileDescriptor {
                url: Some(url),
                ..Default::default()
            },
            FileInput::FileToken(token) => FileDescriptor {
                file_token: Some(token),
                ..Default::default()
            },
            FileInput::Ref(r) => {
                if r.starts_with("http://") || r.starts_with("https://") {
                    FileDescriptor {
                        url: Some(r),
                        ..Default::default()
                    }
                } else {
                    FileDescriptor {
                        file_token: Some(r),
                        ..Default::default()
                    }
                }
            }
            FileInput::Descriptor(d) => d,
            FileInput::Empty => FileDescriptor::default(),
        }
    }
}

/// A single per-view edit instruction for `edit_multiview`.
#[derive(Debug, Clone, Serialize)]
pub struct MultiviewPrompt {
    /// The edit instruction, e.g. "change the shirt color to red".
    pub prompt: String,
    /// The view to apply the edit to; see the [`view`](crate::constants::view)
    /// constants.
    pub view: String,
}

impl MultiviewPrompt {
    pub fn new(prompt: impl Into<String>, view: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            view: view.into(),
        }
    }
}
