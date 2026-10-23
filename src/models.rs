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
    #[serde(default)]
    pub request_id: Option<String>,
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
    /// Segmented model produced by `smart_segment`.
    #[serde(default)]
    pub seg_model_url: Option<String>,
    /// Segmentation mask produced by `smart_segment`.
    #[serde(default)]
    pub mask_url: Option<String>,
    /// Comma-separated part list detected by `smart_segment`.
    #[serde(default)]
    pub prompt: Option<String>,
    /// ID of the segmentation task `smart_segment` ran internally.
    #[serde(default)]
    pub seg_task_id: Option<String>,
    /// ID of the generation task `smart_segment` ran internally.
    #[serde(default)]
    pub model_task_id: Option<String>,
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
            .or(self.seg_model_url.as_deref())
    }
}

impl Task {
    /// Convenience accessor mirroring [`TaskOutput::primary_model_url`].
    pub fn primary_model_url(&self) -> Option<&str> {
        self.output.as_ref().and_then(TaskOutput::primary_model_url)
    }
}

/// `POST /v3/tasks/list` response payload.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TaskList {
    /// Snapshots of the tasks the server found, keyed by `task_id`.
    #[serde(default)]
    pub tasks: HashMap<String, Task>,
    /// Requested IDs the server could not find: unknown, expired, or
    /// created with a different API key.
    #[serde(default)]
    pub missed: Vec<String>,
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

/// One entry of `GET /v3/account/usage`: the credits a single task
/// consumed.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageRecord {
    pub task_id: String,
    /// Task type, e.g. `"image_to_model"` or `"animate_rig"`.
    #[serde(rename = "type")]
    pub task_type: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub credits_consumed: f64,
    /// ISO 8601 creation time.
    pub created_at: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// `POST /v3/files/presign` response payload.
#[derive(Debug, Clone, Deserialize)]
pub struct PresignedUpload {
    /// Accepts a single HTTP PUT of the file body. It carries its own
    /// signature, so the request must not send an `Authorization` header.
    pub presigned_url: String,
    /// Refers to the uploaded file once the PUT has succeeded.
    pub file_token: String,
    /// Lifetime of `presigned_url` in seconds.
    pub expires_in: u64,
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

/// One entry of a view-keyed image list, as accepted by
/// [`MultiviewInputs::Keyed`](crate::params::MultiviewInputs::Keyed) and
/// [`TexturePrompt::images`]. Serialises as a single-key object such as
/// `{"front": "file_abc"}`.
#[derive(Debug, Clone)]
pub struct ViewImage {
    /// One of the [`view`](crate::constants::view) constants.
    pub view: String,
    pub image: FileInput,
}

impl ViewImage {
    pub fn new(view: impl Into<String>, image: impl Into<FileInput>) -> Self {
        Self {
            view: view.into(),
            image: image.into(),
        }
    }
}

impl Serialize for ViewImage {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.view, &self.image)?;
        map.end()
    }
}

/// Checks a view-keyed list: known views only, no repeats, the front view
/// present, and at least two views in total.
pub(crate) fn validate_views(views: &[ViewImage]) -> std::result::Result<(), String> {
    use crate::constants::view;
    let mut seen: Vec<&str> = Vec::with_capacity(views.len());
    for v in views {
        if ![view::FRONT, view::LEFT, view::BACK, view::RIGHT].contains(&v.view.as_str()) {
            return Err(format!("unknown view {:?}", v.view));
        }
        if seen.contains(&v.view.as_str()) {
            return Err(format!("view {:?} given more than once", v.view));
        }
        if v.image.is_empty() {
            return Err(format!("view {:?} has no image", v.view));
        }
        seen.push(&v.view);
    }
    if !seen.contains(&view::FRONT) {
        return Err("the front view is required".into());
    }
    if views.len() < 2 {
        return Err("at least 2 views are required".into());
    }
    Ok(())
}

/// Guides `POST /v3/models/texture`. `text`, `image` and `images` are
/// mutually exclusive, and `style_image` may only accompany `text`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TexturePrompt {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// A single reference image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<FileInput>,
    /// 2 to 4 reference images keyed by angle; the front view is mandatory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ViewImage>>,
    /// A style reference used together with `text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_image: Option<FileInput>,
}

impl TexturePrompt {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    pub fn image(image: impl Into<FileInput>) -> Self {
        Self {
            image: Some(image.into()),
            ..Default::default()
        }
    }

    pub fn images(images: impl IntoIterator<Item = ViewImage>) -> Self {
        Self {
            images: Some(images.into_iter().collect()),
            ..Default::default()
        }
    }

    /// Adds a style reference; only valid together with `text`.
    pub fn with_style_image(mut self, style_image: impl Into<FileInput>) -> Self {
        self.style_image = Some(style_image.into());
        self
    }

    pub(crate) fn validate(&self) -> std::result::Result<(), String> {
        let has_text = self.text.as_deref().is_some_and(|t| !t.is_empty());
        let kinds = [has_text, self.image.is_some(), self.images.is_some()]
            .iter()
            .filter(|&&set| set)
            .count();
        if kinds > 1 {
            return Err("`text`, `image` and `images` are mutually exclusive".into());
        }
        if self.style_image.is_some() && !has_text {
            return Err("`style_image` requires `text`".into());
        }
        if let Some(images) = &self.images {
            validate_views(images).map_err(|e| format!("`images`: {e}"))?;
        }
        Ok(())
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
