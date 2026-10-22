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
use crate::models::{Balance, Task, TaskCreated, TaskList, UploadedFile};
use crate::params::*;
use reqwest::Method;
use serde::Serialize;
use std::time::{Duration, Instant};

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

    /// `POST /v3/tasks/list` — batch query multiple tasks in one round trip.
    pub async fn list_tasks(&self, task_ids: &[String]) -> Result<Vec<Task>> {
        if task_ids.is_empty() {
            return Err(Error::invalid_argument(
                "list_tasks: `task_ids` must be non-empty",
            ));
        }
        let body = serde_json::json!({ "task_ids": task_ids });
        let result: TaskList = self
            .http
            .request_json(
                Method::POST,
                "/tasks/list",
                RequestOptions {
                    json: Some(&body),
                    retries: None,
                },
            )
            .await?;
        Ok(result.tasks)
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

    /// `POST /v3/generation/text-to-model`
    pub async fn text_to_model(&self, params: TextToModelParams) -> Result<String> {
        if params.prompt.trim().is_empty() {
            return Err(Error::invalid_argument(
                "text_to_model: `prompt` is required",
            ));
        }
        self.create_task("/generation/text-to-model", &params).await
    }

    /// `POST /v3/generation/image-to-model`
    pub async fn image_to_model(&self, params: ImageToModelParams) -> Result<String> {
        if params.input.is_empty() {
            return Err(Error::invalid_argument(
                "image_to_model: `input` is required (url, file_token, or task_id)",
            ));
        }
        self.create_task("/generation/image-to-model", &params)
            .await
    }

    /// `POST /v3/generation/multiview-to-model`
    ///
    /// The positional form is a fixed `[front, left, back, right]` array, so
    /// the type system guarantees its length; what is checked here is that a
    /// front view is present and at least 2 views are supplied.
    pub async fn multiview_to_model(&self, params: MultiviewToModelParams) -> Result<String> {
        match &params.inputs {
            None => {
                return Err(Error::invalid_argument(
                    "multiview_to_model: provide `inputs` ([front, left, back, right]) or a source task_id",
                ))
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
        self.create_task("/generation/multiview-to-model", &params)
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
    pub async fn texture_model(&self, params: TextureModelParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "texture_model: `input` (source task_id) is required",
            ));
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

    /// `POST /v3/mesh/segment` — semantic segmentation of a mesh.
    pub async fn segment_mesh(&self, params: SegmentMeshParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument("segment_mesh: `input` is required"));
        }
        self.create_task("/mesh/segment", &params).await
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

    /// `POST /v3/animations/rig` — attach a skeleton to a model.
    pub async fn rig_model(&self, params: RigModelParams) -> Result<String> {
        if params.input.trim().is_empty() {
            return Err(Error::invalid_argument(
                "rig_model: `input` (source task_id) is required",
            ));
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
