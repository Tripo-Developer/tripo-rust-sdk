//! Request parameter structs for every `TripoClient` method.
//!
//! Every struct derives `Default` so callers can use struct-update syntax:
//!
//! ```no_run
//! use tripo3d_sdk::params::TextToModelParams;
//!
//! let params = TextToModelParams {
//!     prompt: "a cute cat".into(),
//!     texture: Some(true),
//!     ..Default::default()
//! };
//! ```
//!
//! Unknown/forward-compatible fields can be passed via `extra`, which is
//! flattened into the JSON payload alongside the typed fields.

use crate::models::{
    FileDescriptor, FileInput, MultiviewPrompt, MultiviewTaskRef, TexturePrompt, ViewImage,
};
use serde::Serialize;
use std::collections::HashMap;

/// `POST /v3/generation/text-to-model`
#[derive(Debug, Clone, Default, Serialize)]
pub struct TextToModelParams {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pbr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_size: Option<bool>,
    /// Outputs a quad mesh instead of triangles, which forces the output
    /// format to FBX rather than GLB. Within the P series only
    /// [`model_version::P2`](crate::constants::model_version::P2) accepts it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quad: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smart_low_poly: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_parts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_uv: Option<bool>,
    /// Forward axis of the exported model; see the
    /// [`export_orientation`](crate::constants::export_orientation) constants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_orientation: Option<String>,
    /// Texture model; see the [`texture_version`](crate::constants::texture_version)
    /// constants. The server picks the default for `model` when unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_version: Option<String>,
    /// Removes baked-in lighting from the texture. The server defaults to
    /// `true`; it only takes effect with
    /// [`texture_version::V3_5`](crate::constants::texture_version::V3_5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delight: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl TextToModelParams {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }
}

/// `POST /v3/generation/image-to-model`
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImageToModelParams {
    /// The source image: a public URL, a `file_token`, or the `task_id` of
    /// an earlier text-to-image or image-to-image task.
    pub input: FileInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_image_autofix: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pbr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_alignment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_size: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
    /// Outputs a quad mesh instead of triangles, which forces the output
    /// format to FBX rather than GLB. Within the P series only
    /// [`model_version::P2`](crate::constants::model_version::P2) accepts it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quad: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smart_low_poly: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_parts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_uv: Option<bool>,
    /// Forward axis of the exported model; see the
    /// [`export_orientation`](crate::constants::export_orientation) constants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_orientation: Option<String>,
    /// Texture model; see the [`texture_version`](crate::constants::texture_version)
    /// constants. The server picks the default for `model` when unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_version: Option<String>,
    /// Removes baked-in lighting from the texture. The server defaults to
    /// `true`; it only takes effect with
    /// [`texture_version::V3_5`](crate::constants::texture_version::V3_5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delight: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ImageToModelParams {
    pub fn new(input: impl Into<FileInput>) -> Self {
        Self {
            input: input.into(),
            ..Default::default()
        }
    }
}

/// The `inputs` payload of `POST /v3/generation/multiview-to-model`.
// The variants differ in size because `Views` holds four descriptors; boxing
// would penalise the common path to shrink a value built once per request.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum MultiviewInputs {
    /// 2 to 4 images keyed by angle, in any order (recommended). The front
    /// view is mandatory.
    Keyed(Vec<ViewImage>),
    /// Exactly 4 views in `[front, left, back, right]` order. The front view
    /// is mandatory and at least 2 views must be supplied; use
    /// [`FileInput::Empty`] to skip the others.
    Views([FileInput; 4]),
    /// Reuses the 4-view output of a successful image-to-multiview or
    /// edit-multiview task.
    TaskId([MultiviewTaskRef; 1]),
}

/// `POST /v3/generation/multiview-to-model`
#[derive(Debug, Clone, Default, Serialize)]
pub struct MultiviewToModelParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<MultiviewInputs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pbr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_alignment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_size: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
    /// Outputs a quad mesh instead of triangles, which forces the output
    /// format to FBX rather than GLB. Within the P series only
    /// [`model_version::P2`](crate::constants::model_version::P2) accepts it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quad: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smart_low_poly: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_parts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_uv: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_orientation: Option<String>,
    /// Texture model; see the [`texture_version`](crate::constants::texture_version)
    /// constants. The server picks the default for `model` when unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_version: Option<String>,
    /// Removes baked-in lighting from the texture. The server defaults to
    /// `true`; it only takes effect with
    /// [`texture_version::V3_5`](crate::constants::texture_version::V3_5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delight: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl MultiviewToModelParams {
    /// 2 to 4 images keyed by angle; the front view is mandatory.
    ///
    /// ```
    /// use tripo3d_sdk::{view, ViewImage, params::MultiviewToModelParams};
    ///
    /// let params = MultiviewToModelParams::from_view_images([
    ///     ViewImage::new(view::FRONT, "https://example.com/front.png"),
    ///     ViewImage::new(view::BACK, "https://example.com/back.png"),
    /// ]);
    /// ```
    pub fn from_view_images(views: impl IntoIterator<Item = ViewImage>) -> Self {
        Self {
            inputs: Some(MultiviewInputs::Keyed(views.into_iter().collect())),
            ..Default::default()
        }
    }

    /// Views in `[front, left, back, right]` order — use `None` for the
    /// views you want to skip.
    ///
    /// ```
    /// use tripo3d_sdk::{models::FileInput, params::MultiviewToModelParams};
    ///
    /// let params = MultiviewToModelParams::from_views([
    ///     Some(FileInput::from("https://example.com/front.png")),
    ///     None,
    ///     Some(FileInput::from("https://example.com/back.png")),
    ///     None,
    /// ]);
    /// ```
    pub fn from_views(views: [Option<FileInput>; 4]) -> Self {
        Self {
            inputs: Some(MultiviewInputs::Views(
                views.map(|v| v.unwrap_or(FileInput::Empty)),
            )),
            ..Default::default()
        }
    }

    /// Reuses the 4-view output of an earlier image-to-multiview or
    /// edit-multiview task.
    pub fn from_task_id(task_id: impl Into<String>) -> Self {
        Self {
            inputs: Some(MultiviewInputs::TaskId([MultiviewTaskRef {
                task_id: task_id.into(),
            }])),
            ..Default::default()
        }
    }
}

/// `POST /v3/generation/text-to-image`
///
/// Several fields are only honoured by a subset of the
/// [`image_model`](crate::constants::image_model) values; see the constant
/// documentation for `quality`, `background` and `aspect_ratio`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TextToImageParams {
    /// Required unless `template` is set. Chinese and English are both
    /// supported; put the most important elements first and append a
    /// negative prompt after `--no`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Either a resolution tier such as `"2K"` or exact pixels such as
    /// `"2048x2048"`. The accepted values differ per model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    /// Adds an AI-generated-content watermark. Only the seedream models
    /// honour it: the banana models always embed an invisible watermark
    /// that cannot be disabled, and chat_image has no watermark control.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark: Option<bool>,
    /// Applies a generation preset and makes `prompt` optional; see the
    /// [`image_template`](crate::constants::image_template) constants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl TextToImageParams {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    /// Builds a prompt-less request driven entirely by a template.
    pub fn from_template(template: impl Into<String>) -> Self {
        Self {
            template: Some(template.into()),
            ..Default::default()
        }
    }
}

/// `POST /v3/generation/image-to-image` — edit, style transfer, or
/// multi-image fusion.
///
/// Several fields are only honoured by a subset of the
/// [`image_model`](crate::constants::image_model) values; see the constant
/// documentation for `quality`, `background` and `aspect_ratio`. Note that
/// `seedream_v5` is the only seedream model this endpoint accepts —
/// `seedream_v4` is text-to-image only.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImageToImageParams {
    /// A single reference image. Mutually exclusive with `inputs`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<FileInput>,
    /// Multiple reference images, referenced from `prompt` as `image[1]`,
    /// `image[2]` and so on. The ceiling depends on the model: 4 for
    /// seedream, 10 for banana, 16 for chat_image. Mutually exclusive with
    /// `input`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<FileInput>>,
    /// The edit instruction. Required unless `template` is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Either a resolution tier such as `"2K"` or exact pixels such as
    /// `"2048x2048"`. The accepted values differ per model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    /// Applies an edit preset and makes `prompt` optional; see the
    /// [`image_template`](crate::constants::image_template) constants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ImageToImageParams {
    /// A single reference image plus an edit instruction.
    pub fn new(input: impl Into<FileInput>, prompt: impl Into<String>) -> Self {
        Self {
            input: Some(input.into()),
            prompt: Some(prompt.into()),
            ..Default::default()
        }
    }

    /// Several reference images, addressed from the prompt as `image[1]`,
    /// `image[2]` and so on.
    pub fn from_inputs(
        inputs: impl IntoIterator<Item = FileInput>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            inputs: Some(inputs.into_iter().collect()),
            prompt: Some(prompt.into()),
            ..Default::default()
        }
    }
}

/// `POST /v3/generation/image-to-multiview`
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImageToMultiviewParams {
    /// The source image: a public URL, a `file_token`, or the `task_id` of
    /// an earlier image generation task.
    pub input: FileInput,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ImageToMultiviewParams {
    pub fn new(input: impl Into<FileInput>) -> Self {
        Self {
            input: input.into(),
            ..Default::default()
        }
    }
}

/// `POST /v3/generation/edit-multiview` — apply per-view edits to a
/// previously generated multiview set.
#[derive(Debug, Clone, Default, Serialize)]
pub struct EditMultiviewParams {
    /// The `task_id` of an earlier successful image-to-multiview or
    /// edit-multiview task. The API documents `file_token` and URL inputs
    /// too, but the service currently rejects anything that is not a
    /// `task_id`.
    pub input: FileInput,
    /// 1 to 4 per-view edit instructions.
    pub prompts: Vec<MultiviewPrompt>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl EditMultiviewParams {
    pub fn new(
        task_id: impl Into<String>,
        prompts: impl IntoIterator<Item = MultiviewPrompt>,
    ) -> Self {
        Self {
            input: FileInput::Ref(task_id.into()),
            prompts: prompts.into_iter().collect(),
            ..Default::default()
        }
    }
}

/// `POST /v3/models/texture`
#[derive(Debug, Clone, Default, Serialize)]
pub struct TextureModelParams {
    /// The `task_id` of a Tripo-generated or imported model.
    pub input: String,
    /// Texture model; see the [`texture_version`](crate::constants::texture_version)
    /// constants. The server defaults to
    /// [`texture_version::V3_0`](crate::constants::texture_version::V3_0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Guides the new texture. May be omitted for models generated by
    /// Tripo, which then reuse their original prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_prompt: Option<TexturePrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pbr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_alignment: Option<String>,
    /// Removes baked-in lighting from the texture. The server defaults to
    /// `true`; it only takes effect with
    /// [`texture_version::V3_5`](crate::constants::texture_version::V3_5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delight: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bake: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_names: Option<Vec<String>>,
    /// The API ignores a top-level `text_prompt`, so `texture_model` moves
    /// this value into `texture_prompt`.
    #[deprecated(since = "0.4.0", note = "use `texture_prompt.text`; removed in 1.0")]
    #[serde(skip)]
    pub text_prompt: Option<String>,
    /// Moved into `texture_prompt` by `texture_model`.
    #[deprecated(since = "0.4.0", note = "use `texture_prompt.image`; removed in 1.0")]
    #[serde(skip)]
    pub image_prompt: Option<FileDescriptor>,
    /// Moved into `texture_prompt` by `texture_model`.
    #[deprecated(
        since = "0.4.0",
        note = "use `texture_prompt.style_image`; removed in 1.0"
    )]
    #[serde(skip)]
    pub style_image: Option<FileDescriptor>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl TextureModelParams {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            ..Default::default()
        }
    }

    /// Moves the deprecated top-level prompt fields into `texture_prompt`.
    #[allow(deprecated)]
    pub(crate) fn migrate_deprecated_prompt(&mut self) -> std::result::Result<(), String> {
        let text = self.text_prompt.take();
        let image = self.image_prompt.take();
        let style_image = self.style_image.take();
        if text.is_none() && image.is_none() && style_image.is_none() {
            return Ok(());
        }
        if self.texture_prompt.is_some() {
            return Err("the deprecated `text_prompt`, `image_prompt` and `style_image` fields cannot be combined with `texture_prompt`".into());
        }
        self.texture_prompt = Some(TexturePrompt {
            text,
            image: image.map(FileInput::Descriptor),
            images: None,
            style_image: style_image.map(FileInput::Descriptor),
        });
        Ok(())
    }
}

/// `POST /v3/models/convert`
#[derive(Debug, Clone, Default, Serialize)]
pub struct ConvertModelParams {
    pub input: String,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quad: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatten_bottom: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatten_bottom_threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pivot_to_center_bottom: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_animation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack_uv: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_symmetry: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bake: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_names: Option<Vec<String>>,
    /// Writes vertex colors; only OBJ and GLTF honour it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_vertex_colors: Option<bool>,
    /// Uniform output scale. The server defaults to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_factor: Option<f64>,
    /// Strips root motion from animated models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animate_in_place: Option<bool>,
    /// Forward axis of the exported model; see the
    /// [`export_orientation`](crate::constants::export_orientation) constants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_orientation: Option<String>,
    /// FBX compatibility profile; see the
    /// [`fbx_preset`](crate::constants::fbx_preset) constants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fbx_preset: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ConvertModelParams {
    pub fn new(input: impl Into<String>, format: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            format: format.into(),
            ..Default::default()
        }
    }
}

/// `POST /v3/mesh/segment`
///
/// `segmentation_granularity`, `split_by_connectivity` and `ref_image` only
/// take effect with [`segment_model::V2`](crate::constants::segment_model::V2);
/// when `ref_image` is set the other two are ignored.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SegmentMeshParams {
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// See the [`segmentation_granularity`](crate::constants::segmentation_granularity)
    /// constants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segmentation_granularity: Option<String>,
    /// Splits along connected components. The server defaults to `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split_by_connectivity: Option<bool>,
    /// A `file_token` or URL of a reference image, such as a segmentation
    /// mask, that guides the semantic split.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_image: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl SegmentMeshParams {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            ..Default::default()
        }
    }
}

/// `POST /v3/mesh/complete`
#[derive(Debug, Clone, Default, Serialize)]
pub struct CompleteMeshParams {
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_names: Option<Vec<String>>,
    /// See the [`completion_mode`](crate::constants::completion_mode)
    /// constants; the server defaults to `ai_completion`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_mode: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl CompleteMeshParams {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            ..Default::default()
        }
    }
}

/// `POST /v3/mesh/decimate`
#[derive(Debug, Clone, Default, Serialize)]
pub struct DecimateMeshParams {
    pub input: String,
    /// See the [`decimate_model`](crate::constants::decimate_model) constants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quad: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bake: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_names: Option<Vec<String>>,
    /// Generates a cleaner UV layout. Only
    /// [`decimate_model::V3_0`](crate::constants::decimate_model::V3_0)
    /// accepts it; other models reject the request with code 1004.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smart_uv: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl DecimateMeshParams {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            ..Default::default()
        }
    }
}

/// `POST /v3/mesh/uv`
///
/// The input model must not exceed 80,000 equivalent triangles (a quad
/// counts as two). Larger models are accepted at creation time but the task
/// then fails and refunds its credits, so reduce the face count first, e.g.
/// with `decimate_mesh`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MeshUvParams {
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bake: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_orientation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl MeshUvParams {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            ..Default::default()
        }
    }
}

/// `POST /v3/animations/rig-check`
#[derive(Debug, Clone, Default, Serialize)]
pub struct RigCheckParams {
    pub input: String,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl RigCheckParams {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            ..Default::default()
        }
    }
}

/// `POST /v3/animations/rig`
#[derive(Debug, Clone, Default, Serialize)]
pub struct RigModelParams {
    pub input: String,
    /// Rigging model; see the [`rig_model`](crate::constants::rig_model)
    /// constants. When unset, `rig_model` sends
    /// [`rig_model::V3_0`](crate::constants::rig_model::V3_0): the API
    /// currently rejects rig requests that omit `model`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rig_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_format: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl RigModelParams {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            ..Default::default()
        }
    }
}

/// `POST /v3/animations/retarget`
///
/// Provide either `animation` (single preset) or `animations` (up to 5).
#[derive(Debug, Clone, Default, Serialize)]
pub struct RetargetAnimationParams {
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animations: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bake_animation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_with_geometry: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animate_in_place: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl RetargetAnimationParams {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            ..Default::default()
        }
    }
}
