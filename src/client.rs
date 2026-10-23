//! [`TripoClient`] — high-level facade for the Tripo3D v3 API.
//!
//! All generation-style methods return a `task_id` (`String`). Use
//! [`TripoClient::wait_for_task`] to poll until the task settles, or
//! [`TripoClient::get_task`] for a single snapshot.
//!
//! ```no_run
//! use tripo3d_sdk::{TripoClient, ClientOptions, params::TextToModelParams};
//!
//! # async fn run() -> tripo3d_sdk::Result<()> {
//! let client = TripoClient::new(ClientOptions::default())?; // reads TRIPO_API_KEY
//! let task_id = client.text_to_model(TextToModelParams::new("a cute cat")).await?;
//! let task = client.wait_for_task(&task_id, Default::default()).await?;
//! println!("{:?}", task.primary_model_url());
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use crate::http::{HttpClient, HttpConfig, RequestOptions};
use crate::models::{
    validate_views, Balance, PresignedUpload, Task, TaskCreated, TaskList, UploadedFile,
    UsageRecord,
};
use crate::params::*;
use reqwest::Method;
use serde::Serialize;
use std::time::{Duration, Instant};

/// The most task IDs `POST /v3/tasks/list` accepts per call.
pub const MAX_LIST_TASKS: usize = 100;

/// The most records `GET /v3/account/usage` returns per call.
pub const MAX_USAGE_LIMIT: u32 = 200;

/// Paging for [`TripoClient::get_usage`]. The default returns the server's
/// 50 most recent records.
#[derive(Debug, Clone, Copy, Default)]
pub struct UsageOptions {
    /// Number of records, from 1 to [`MAX_USAGE_LIMIT`].
    pub limit: Option<u32>,
    /// Skips that many of the most recent records.
    pub offset: Option<u32>,
}

/// Options accepted by [`TripoClient::new`]. All fields are optional; `api_key`
/// falls back to the `TRIPO_API_KEY` environment variable when unset.
#[derive(Debug, Clone, Default)]
pub struct ClientOptions {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub timeout: Option<Duration>,
    pub retries: Option<u32>,
    pub user_agent: Option<String>,
}

/// Options accepted by [`TripoClient::wait_for_task`].
#[derive(Debug, Clone)]
pub struct WaitOptions {
    /// Delay between polling attempts. Default: 2 seconds.
    pub poll_interval: Duration,
    /// Overall timeout budget. `None` means wait indefinitely.
    pub timeout: Option<Duration>,
    /// When `true` (default), a non-`success` terminal status raises
    /// [`Error::Task`] instead of being returned as `Ok`.
    pub throw_on_failure: bool,
}

impl Default for WaitOptions {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(2),
            timeout: None,
            throw_on_failure: true,
        }
    }
}

/// Result of [`TripoClient::download_model`].
#[derive(Debug, Clone)]
pub struct DownloadedModel {
    pub url: String,
    pub content_type: Option<String>,
    pub data: Vec<u8>,
}

impl DownloadedModel {
    /// The lower-case file extension of the downloaded model, without the
    /// leading dot (e.g. `"glb"`, `"fbx"`), or `None` if the URL carries
    /// none.
    ///
    /// Do not assume GLB: setting `quad` on a generation task forces FBX
    /// output, and `convert_model` emits whichever format was requested.
    pub fn extension(&self) -> Option<String> {
        self.url
            .split(['?', '#'])
            .next()?
            .rsplit('/')
            .next()?
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_lowercase())
    }

    /// A download-ready `<name>.<ext>` for this model, falling back to
    /// `glb` when the URL carries no extension.
    pub fn filename(&self, name: &str) -> String {
        format!(
            "{name}.{}",
            self.extension().unwrap_or_else(|| "glb".into())
        )
    }
}

pub struct TripoClient {
    http: HttpClient,
}

impl TripoClient {
    /// Build a client, reading `TRIPO_API_KEY` from the environment when
    /// `options.api_key` is not provided.
    pub fn new(options: ClientOptions) -> Result<Self> {
        let api_key = options
            .api_key
            .or_else(|| std::env::var("TRIPO_API_KEY").ok())
            .ok_or_else(|| {
                Error::invalid_argument(
                    "an API key is required: pass `api_key` in ClientOptions or set the TRIPO_API_KEY environment variable",
                )
            })?;
        let config = HttpConfig {
            base_url: options
                .base_url
                .unwrap_or_else(|| crate::constants::DEFAULT_BASE_URL.to_string()),
            api_key,
            timeout: options.timeout.unwrap_or(Duration::from_secs(60)),
            retries: options.retries.unwrap_or(2),
            user_agent: options
                .user_agent
                .unwrap_or_else(|| format!("tripo3d-sdk-rust/{}", env!("CARGO_PKG_VERSION"))),
        };
        Ok(Self {
            http: HttpClient::new(config)?,
        })
    }

    /// Shorthand for `TripoClient::new(ClientOptions::default())`.
    pub fn from_env() -> Result<Self> {
        Self::new(ClientOptions::default())
    }

    // ─────────────────────────────── Account ───────────────────────────────

    /// `GET /v3/account/balance`
    pub async fn get_balance(&self) -> Result<Balance> {
        self.http
            .request_json(Method::GET, "/account/balance", RequestOptions::default())
            .await
    }

    /// `GET /v3/account/usage` — credits consumed per task, newest first.
    /// With the default options the server returns the 50 most recent
    /// records.
    pub async fn get_usage(&self, opts: UsageOptions) -> Result<Vec<UsageRecord>> {
        let mut query = Vec::new();
        if let Some(limit) = opts.limit {
            if limit == 0 || limit > MAX_USAGE_LIMIT {
                return Err(Error::invalid_argument(format!(
                    "get_usage: `limit` must be between 1 and {MAX_USAGE_LIMIT}, got {limit}"
                )));
            }
            query.push(format!("limit={limit}"));
        }
        if let Some(offset) = opts.offset {
            query.push(format!("offset={offset}"));
        }
        let mut path = "/account/usage".to_string();
        if !query.is_empty() {
            path = format!("{path}?{}", query.join("&"));
        }
        self.http
            .request_json(Method::GET, &path, RequestOptions::default())
            .await
    }

    // ─────────────────────────────── Files ─────────────────────────────────

    /// `POST /v3/files` — upload a raw file and receive a `file_token`.
    pub async fn upload_file(
        &self,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        content_type: Option<&str>,
    ) -> Result<UploadedFile> {
        let filename = filename.into();
        let mut part = reqwest::multipart::Part::bytes(bytes).file_name(filename);
        if let Some(ct) = content_type {
            part = part
                .mime_str(ct)
                .map_err(|e| Error::invalid_argument(format!("invalid content type: {e}")))?;
        }
        let form = reqwest::multipart::Form::new().part("file", part);
        self.http.request_multipart("/files", form).await
    }

    /// `POST /v3/files/presign` — reserve a `file_token` and a URL to PUT
    /// the file to. `format` is the file extension, such as `"glb"`; a
    /// leading dot is ignored. Most callers want
    /// [`TripoClient::upload_large_file`] or
    /// [`TripoClient::upload_large_file_from_path`], which also perform the
    /// PUT.
    pub async fn presign_upload(&self, format: &str) -> Result<PresignedUpload> {
        let format = normalize_format(format);
        if format.is_empty() {
            return Err(Error::invalid_argument(
                "presign_upload: `format` (file extension) is required",
            ));
        }
        let body = serde_json::json!({ "format": format });
        self.http
            .request_json(
                Method::POST,
                "/files/presign",
                RequestOptions {
                    json: Some(&body),
                    retries: None,
                },
            )
            .await
    }

    /// Upload `bytes` straight to storage and return the file's token. This
    /// is the recommended path for files over 60 MB: the file never passes
    /// through the API server. To avoid loading a file on disk into memory,
    /// use [`TripoClient::upload_large_file_from_path`].
    ///
    /// The upload has no timeout and is not retried.
    pub async fn upload_large_file(&self, bytes: Vec<u8>, format: &str) -> Result<UploadedFile> {
        if bytes.is_empty() {
            return Err(Error::invalid_argument(
                "upload_large_file: `bytes` is empty",
            ));
        }
        let size = bytes.len() as u64;
        self.upload_presigned(format, reqwest::Body::from(bytes), size)
            .await
    }

    /// Stream a file on disk straight to storage and return its token.
    /// `format` defaults to the file's extension. See
    /// [`TripoClient::upload_large_file`].
    pub async fn upload_large_file_from_path(
        &self,
        path: impl AsRef<std::path::Path>,
        format: Option<&str>,
    ) -> Result<UploadedFile> {
        let path = path.as_ref();
        let format = match format {
            Some(f) => f.to_string(),
            None => path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_string)
                .ok_or_else(|| {
                    Error::invalid_argument(
                        "upload_large_file_from_path: the path has no extension; pass `format`",
                    )
                })?,
        };
        let file = tokio::fs::File::open(path).await?;
        let size = file.metadata().await?.len();
        if size == 0 {
            return Err(Error::invalid_argument(
                "upload_large_file_from_path: the file is empty",
            ));
        }
        self.upload_presigned(&format, reqwest::Body::from(file), size)
            .await
    }

    async fn upload_presigned(
        &self,
        format: &str,
        body: reqwest::Body,
        size: u64,
    ) -> Result<UploadedFile> {
        let presigned = self.presign_upload(format).await?;
        self.http
            .put_raw(&presigned.presigned_url, body, size)
            .await?;
        Ok(UploadedFile {
            file_token: presigned.file_token,
            extra: Default::default(),
        })
    }

    // ────────────────────────── Task management ────────────────────────────

    /// `GET /v3/tasks/{task_id}`
    pub async fn get_task(&self, task_id: &str) -> Result<Task> {
        if task_id.is_empty() {
            return Err(Error::invalid_argument("get_task: `task_id` is required"));
        }
        let path = format!("/tasks/{}", urlencoding_encode(task_id));
        self.http
            .request_json(Method::GET, &path, RequestOptions::default())
            .await
    }

    /// `POST /v3/tasks/list` — batch query up to [`MAX_LIST_TASKS`] tasks in
    /// one round trip. IDs the server cannot find are reported in
    /// [`TaskList::missed`] rather than as an error.
    pub async fn list_tasks(&self, task_ids: &[String]) -> Result<TaskList> {
        if task_ids.is_empty() {
            return Err(Error::invalid_argument(
                "list_tasks: `task_ids` must be non-empty",
            ));
        }
        if task_ids.len() > MAX_LIST_TASKS {
            return Err(Error::invalid_argument(format!(
                "list_tasks: at most {MAX_LIST_TASKS} task IDs per call, got {}",
                task_ids.len()
            )));
        }
        let body = serde_json::json!({ "task_ids": task_ids });
        self.http
            .request_json(
                Method::POST,
                "/tasks/list",
                RequestOptions {
                    json: Some(&body),
                    retries: None,
                },
            )
            .await
    }

    /// Poll a task until it reaches a terminal state.
    pub async fn wait_for_task(&self, task_id: &str, opts: WaitOptions) -> Result<Task> {
        self.wait_for_task_with_progress(task_id, opts, |_| {})
            .await
    }

    /// Same as [`TripoClient::wait_for_task`], but invokes `on_progress` after
    /// every poll (including the final one).
    pub async fn wait_for_task_with_progress<F>(
        &self,
        task_id: &str,
        opts: WaitOptions,
        mut on_progress: F,
    ) -> Result<Task>
    where
        F: FnMut(&Task),
    {
        let started = Instant::now();
        loop {
            let task = self.get_task(task_id).await?;
            on_progress(&task);

            if task.status.is_terminal() {
                if opts.throw_on_failure && !task.status.is_success() {
                    return Err(Error::Task {
                        task: Box::new(task),
                    });
                }
                return Ok(task);
            }

            if let Some(timeout) = opts.timeout {
                if started.elapsed() >= timeout {
                    return Err(Error::Timeout {
                        task_id: task_id.to_string(),
                        timeout_ms: timeout.as_millis() as u64,
                    });
                }
            }

            tokio::time::sleep(opts.poll_interval).await;
        }
    }

    // ─────────────────────────── 3D Generation ─────────────────────────────

    /// `POST /v3/generation/text-to-model`. Sends
    /// [`model_version::H3_1`](crate::constants::model_version::H3_1) when no
    /// `model` is set: the API currently rejects generation requests that omit
    /// it.
    pub async fn text_to_model(&self, mut params: TextToModelParams) -> Result<String> {
        if params.prompt.trim().is_empty() {
            return Err(Error::invalid_argument(
                "text_to_model: `prompt` is required",
            ));
        }
        if params.model.is_none() && !params.extra.contains_key("model") {
            params.model = Some(crate::constants::model_version::H3_1.to_string());
        }
        self.create_task("/generation/text-to-model", &params).await
    }

    /// `POST /v3/generation/image-to-model`. Sends
    /// [`model_version::H3_1`](crate::constants::model_version::H3_1) when no
    /// `model` is set.
    pub async fn image_to_model(&self, mut params: ImageToModelParams) -> Result<String> {
        if params.input.is_empty() {
            return Err(Error::invalid_argument(
                "image_to_model: `input` is required (url, file_token, or task_id)",
            ));
        }
        if params.model.is_none() && !params.extra.contains_key("model") {
            params.model = Some(crate::constants::model_version::H3_1.to_string());
        }
        self.create_task("/generation/image-to-model", &params)
            .await
    }

    /// `POST /v3/generation/multiview-to-model`
    ///
    /// The positional form is a fixed `[front, left, back, right]` array, so
    /// the type system guarantees its length; what is checked here is that a
    /// front view is present and at least 2 views are supplied.
    /// Sends [`model_version::H3_1`](crate::constants::model_version::H3_1)
    /// when no `model` is set.
    pub async fn multiview_to_model(&self, mut params: MultiviewToModelParams) -> Result<String> {
        match &params.inputs {
            None => {
                return Err(Error::invalid_argument(
                    "multiview_to_model: provide `inputs` (view-keyed, [front, left, back, right], or a source task_id)",
                ))
            }
            Some(MultiviewInputs::Keyed(views)) => {
                validate_views(views).map_err(|e| {
                    Error::invalid_argument(format!("multiview_to_model: `inputs`: {e}"))
                })?;
            }
            Some(MultiviewInputs::Views(views)) => {
                if views[0].is_empty() {
                    return Err(Error::invalid_argument(
                        "multiview_to_model: the front view (`inputs[0]`) is required",
                    ));
                }
                if views.iter().filter(|v| !v.is_empty()).count() < 2 {
                    return Err(Error::invalid_argument(
                        "multiview_to_model: at least 2 views are required",
                    ));
                }
            }
            Some(MultiviewInputs::TaskId(_)) => {}
        }
        if params.model.is_none() && !params.extra.contains_key("model") {
            params.model = Some(crate::constants::model_version::H3_1.to_string());
        }
        self.create_task("/generation/multiview-to-model", &params)
            .await
    }

    /// `POST /v3/generation/image-to-splat` — reconstruct a 3D Gaussian
    /// splat from one image. The finished task's `model_url` points to a
    /// `.splat` file.
    pub async fn image_to_splat(&self, params: ImageToSplatParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "image_to_splat: `input` (file_token or URL) is required",
            ));
        }
        self.create_task("/generation/image-to-splat", &params)
            .await
    }

    // ───────────────────────── Image generation ─────────────────────────────

    /// `POST /v3/generation/text-to-image`
    pub async fn text_to_image(&self, params: TextToImageParams) -> Result<String> {
        if params.prompt.trim().is_empty() && params.template.is_none() {
            return Err(Error::invalid_argument(
                "text_to_image: `prompt` is required unless `template` is set",
            ));
        }
        self.create_task("/generation/text-to-image", &params).await
    }

    /// `POST /v3/generation/image-to-image`
    pub async fn image_to_image(&self, params: ImageToImageParams) -> Result<String> {
        let has_inputs = params.inputs.as_ref().is_some_and(|v| !v.is_empty());
        if params.input.is_none() && !has_inputs {
            return Err(Error::invalid_argument(
                "image_to_image: `input` or `inputs` is required",
            ));
        }
        if params.input.is_some() && has_inputs {
            return Err(Error::invalid_argument(
                "image_to_image: `input` and `inputs` are mutually exclusive",
            ));
        }
        if params.prompt.is_none() && params.template.is_none() {
            return Err(Error::invalid_argument(
                "image_to_image: `prompt` is required unless `template` is set",
            ));
        }
        self.create_task("/generation/image-to-image", &params)
            .await
    }

    /// `POST /v3/generation/image-to-multiview`
    pub async fn image_to_multiview(&self, params: ImageToMultiviewParams) -> Result<String> {
        if params.input.is_empty() {
            return Err(Error::invalid_argument(
                "image_to_multiview: `input` is required (url, file_token, or task_id)",
            ));
        }
        self.create_task("/generation/image-to-multiview", &params)
            .await
    }

    /// `POST /v3/generation/edit-multiview`
    pub async fn edit_multiview(&self, params: EditMultiviewParams) -> Result<String> {
        if params.input.is_empty() {
            return Err(Error::invalid_argument(
                "edit_multiview: `input` is required (the task_id of a multiview task)",
            ));
        }
        if params.prompts.is_empty() || params.prompts.len() > 4 {
            return Err(Error::invalid_argument(
                "edit_multiview: `prompts` must contain 1 to 4 items",
            ));
        }
        self.create_task("/generation/edit-multiview", &params)
            .await
    }

    // ───────────────────────── Model post-processing ────────────────────────

    /// `POST /v3/models/texture` — re-texture an existing model.
    ///
    /// The deprecated top-level `text_prompt`, `image_prompt` and
    /// `style_image` fields are moved into `texture_prompt` before sending.
    pub async fn texture_model(&self, mut params: TextureModelParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "texture_model: `input` (source task_id) is required",
            ));
        }
        params
            .migrate_deprecated_prompt()
            .map_err(|e| Error::invalid_argument(format!("texture_model: {e}")))?;
        if let Some(prompt) = &params.texture_prompt {
            prompt.validate().map_err(|e| {
                Error::invalid_argument(format!("texture_model: `texture_prompt`: {e}"))
            })?;
        }
        self.create_task("/models/texture", &params).await
    }

    /// `POST /v3/models/convert` — convert a completed model to another format.
    pub async fn convert_model(&self, params: ConvertModelParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "convert_model: `input` is required",
            ));
        }
        if params.format.trim().is_empty() {
            return Err(Error::invalid_argument(
                "convert_model: `format` is required",
            ));
        }
        self.create_task("/models/convert", &params).await
    }

    /// `POST /v3/models/import` — bring an external model into Tripo. The
    /// finished task can be the `input` of the post-processing endpoints,
    /// such as `texture_model` or `convert_model`.
    pub async fn import_model(&self, params: ImportModelParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "import_model: `input` (file_token or URL) is required",
            ));
        }
        self.create_task("/models/import", &params).await
    }

    /// `POST /v3/mesh/segment` — semantic segmentation of a mesh.
    pub async fn segment_mesh(&self, params: SegmentMeshParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument("segment_mesh: `input` is required"));
        }
        self.create_task("/mesh/segment", &params).await
    }

    /// `POST /v3/mesh/smartsegment` — segmentation pipeline. The finished
    /// task reports the segmented model in
    /// [`TaskOutput::seg_model_url`](crate::models::TaskOutput::seg_model_url).
    pub async fn smart_segment(&self, params: SmartSegmentParams) -> Result<String> {
        use crate::constants::seg_type;
        match params.seg_type.as_str() {
            seg_type::IMAGE | seg_type::MODEL => {}
            "" => {
                return Err(Error::invalid_argument(
                    "smart_segment: `seg_type` is required; build with SmartSegmentParams::image or ::model",
                ))
            }
            other => {
                return Err(Error::invalid_argument(format!(
                    "smart_segment: unknown `seg_type` {other:?}"
                )))
            }
        }
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "smart_segment: `input` is required",
            ));
        }
        if params.seg_type == seg_type::MODEL && params.transform.is_none() {
            return Err(Error::invalid_argument(
                "smart_segment: `transform` is required for seg_type::MODEL",
            ));
        }
        self.create_task("/mesh/smartsegment", &params).await
    }

    /// `POST /v3/mesh/complete` — mesh completion / repair.
    pub async fn complete_mesh(&self, params: CompleteMeshParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "complete_mesh: `input` is required",
            ));
        }
        self.create_task("/mesh/complete", &params).await
    }

    /// `POST /v3/mesh/decimate` — retopology / face-count reduction.
    pub async fn decimate_mesh(&self, params: DecimateMeshParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "decimate_mesh: `input` is required",
            ));
        }
        self.create_task("/mesh/decimate", &params).await
    }

    /// `POST /v3/mesh/uv` — re-unwrap UVs, remapping the existing textures,
    /// skeleton and animations onto the new layout. The finished task also
    /// carries a UV layout preview in [`TaskOutput::uv_render_url`](crate::models::TaskOutput::uv_render_url).
    pub async fn mesh_uv(&self, params: MeshUvParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument("mesh_uv: `input` is required"));
        }
        self.create_task("/mesh/uv", &params).await
    }

    // ─────────────────────────────── Animation ──────────────────────────────

    /// `POST /v3/animations/rig-check`
    pub async fn rig_check(&self, params: RigCheckParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "rig_check: `input` (source task_id) is required",
            ));
        }
        self.create_task("/animations/rig-check", &params).await
    }

    /// `POST /v3/animations/rig` — attach a skeleton to a model. Sends
    /// [`rig_model::V3_0`](crate::constants::rig_model::V3_0) when no `model`
    /// is set.
    pub async fn rig_model(&self, mut params: RigModelParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "rig_model: `input` (source task_id) is required",
            ));
        }
        if params.model.is_none() && !params.extra.contains_key("model") {
            params.model = Some(crate::constants::rig_model::V3_0.to_string());
        }
        self.create_task("/animations/rig", &params).await
    }

    /// `POST /v3/animations/retarget` — apply preset animations to a rigged model.
    pub async fn retarget_animation(&self, params: RetargetAnimationParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "retarget_animation: `input` (rigged task_id) is required",
            ));
        }
        let has_single = params.animation.is_some();
        let animations_len = params.animations.as_ref().map(|v| v.len()).unwrap_or(0);
        if !has_single && animations_len == 0 {
            return Err(Error::invalid_argument(
                "retarget_animation: provide `animation` or a non-empty `animations` list",
            ));
        }
        if animations_len > 5 {
            return Err(Error::invalid_argument(
                "retarget_animation: at most 5 animations per call",
            ));
        }
        self.create_task("/animations/retarget", &params).await
    }

    // ────────────────────────── Downloads ───────────────────────────────────

    /// Download the primary model URL of a completed task.
    /// Returns `None` when the task has no model output.
    pub async fn download_model(&self, task: &Task) -> Result<Option<DownloadedModel>> {
        let url = match task.primary_model_url() {
            Some(u) => u.to_string(),
            None => return Ok(None),
        };
        let (data, content_type) = self.http.get_raw(&url).await?;
        Ok(Some(DownloadedModel {
            url,
            content_type,
            data,
        }))
    }

    // ────────────────────────────── Internals ───────────────────────────────

    async fn create_task<T: Serialize>(&self, path: &str, params: &T) -> Result<String> {
        let value = serde_json::to_value(params)?;
        let created: TaskCreated = self
            .http
            .request_json(
                Method::POST,
                path,
                RequestOptions {
                    json: Some(&value),
                    retries: None,
                },
            )
            .await?;
        Ok(created.task_id)
    }
}

fn normalize_format(format: &str) -> String {
    let format = format.trim();
    format
        .strip_prefix('.')
        .unwrap_or(format)
        .to_ascii_lowercase()
}

/// Minimal path-segment percent-encoding (avoids pulling in the `urlencoding`
/// crate for a single call site).
fn urlencoding_encode(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for b in segment.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
