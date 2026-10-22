//! Enumerations and public constants for the Tripo3D v3 API.
//!
//! Enums implement `Serialize`/`Deserialize` via their kebab/snake-case wire
//! representation (matching the JSON strings the API expects), and
//! `AsRef<str>` / `Display` for easy interpolation into request payloads.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Default REST endpoint (China mainland). Overseas / global:
/// `https://openapi.tripo3d.ai/v3`.
pub const DEFAULT_BASE_URL: &str = "https://openapi.tripo3d.com/v3";

/// Task lifecycle statuses returned by `GET /v3/tasks/{task_id}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Queued,
    Running,
    Success,
    Failed,
    Cancelled,
    Unknown,
    Banned,
    Expired,
}

impl TaskStatus {
    /// A task will not change once it enters one of these states.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Success
                | TaskStatus::Failed
                | TaskStatus::Cancelled
                | TaskStatus::Banned
                | TaskStatus::Expired
        )
    }

    pub fn is_success(&self) -> bool {
        matches!(self, TaskStatus::Success)
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TaskStatus::Queued => "queued",
            TaskStatus::Running => "running",
            TaskStatus::Success => "success",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Unknown => "unknown",
            TaskStatus::Banned => "banned",
            TaskStatus::Expired => "expired",
        };
        f.write_str(s)
    }
}

/// Preset animation identifiers accepted by `POST /v3/animations/retarget`.
/// Combine at most 5 in a single retarget call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Animation {
    Idle,
    Walk,
    Run,
    Dive,
    Climb,
    Jump,
    Slash,
    Shoot,
    Hurt,
    Fall,
    Turn,
    QuadrupedWalk,
    HexapodWalk,
    OctopodWalk,
    SerpentineMarch,
    AquaticMarch,
}

impl Animation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Animation::Idle => "preset:idle",
            Animation::Walk => "preset:walk",
            Animation::Run => "preset:run",
            Animation::Dive => "preset:dive",
            Animation::Climb => "preset:climb",
            Animation::Jump => "preset:jump",
            Animation::Slash => "preset:slash",
            Animation::Shoot => "preset:shoot",
            Animation::Hurt => "preset:hurt",
            Animation::Fall => "preset:fall",
            Animation::Turn => "preset:turn",
            Animation::QuadrupedWalk => "preset:quadruped:walk",
            Animation::HexapodWalk => "preset:hexapod:walk",
            Animation::OctopodWalk => "preset:octopod:walk",
            Animation::SerpentineMarch => "preset:serpentine:march",
            Animation::AquaticMarch => "preset:aquatic:march",
        }
    }
}

impl fmt::Display for Animation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Animation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

/// Skeleton topology types for `POST /v3/animations/rig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RigType {
    Biped,
    Quadruped,
    Hexapod,
    Octopod,
    Avian,
    Serpentine,
    Aquatic,
    Others,
}

impl fmt::Display for RigType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RigType::Biped => "biped",
            RigType::Quadruped => "quadruped",
            RigType::Hexapod => "hexapod",
            RigType::Octopod => "octopod",
            RigType::Avian => "avian",
            RigType::Serpentine => "serpentine",
            RigType::Aquatic => "aquatic",
            RigType::Others => "others",
        };
        f.write_str(s)
    }
}

/// Rig specifications — controls bone naming/hierarchy conventions. Only
/// honoured for [`RigType::Biped`]. `ActorCore`, `Unity`, `Unreal` and `Vrm`
/// require [`rig_model::V3_0`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum RigSpec {
    Mixamo,
    Tripo,
    ActorCore,
    Unity,
    Unreal,
    Vrm,
}

impl fmt::Display for RigSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RigSpec::Mixamo => "mixamo",
            RigSpec::Tripo => "tripo",
            RigSpec::ActorCore => "actorcore",
            RigSpec::Unity => "unity",
            RigSpec::Unreal => "unreal",
            RigSpec::Vrm => "vrm",
        };
        f.write_str(s)
    }
}

/// `model` values accepted by `POST /v3/animations/rig`. [`rig_model::V1_0`]
/// rigs bipeds only; [`rig_model::V2_5`] and [`rig_model::V3_0`] rig every
/// [`RigType`], and only [`rig_model::V3_0`] accepts [`RigType::Others`].
pub mod rig_model {
    pub const V1_0: &str = "v1.0-20240301";
    pub const V2_5: &str = "v2.5-20260210";
    pub const V3_0: &str = "v3.0-20260909";
}

/// Texture models, selectable via `texture_version` on the 3D generation
/// endpoints and via `model` on `POST /v3/models/texture`.
/// [`texture_quality::FAST`] and `delight` require [`texture_version::V3_5`].
pub mod texture_version {
    pub const V3_5: &str = "v3.5-20260815";
    pub const V3_0: &str = "v3.0-20250812";
    pub const V2_5: &str = "v2.5-20250123";
}

/// `model` values accepted by `POST /v3/mesh/decimate`. The server defaults
/// to [`decimate_model::V2_0`]. `smart_uv` requires [`decimate_model::V3_0`],
/// and [`decimate_model::V1_0`] accepts neither `bake` nor `part_names`.
pub mod decimate_model {
    pub const V1_0: &str = "v1.0";
    pub const V2_0: &str = "v2.0";
    pub const V3_0: &str = "v3.0";
}

/// `model` values accepted by `POST /v3/mesh/segment`. The server defaults
/// to [`segment_model::V1`]; `segmentation_granularity`,
/// `split_by_connectivity` and `ref_image` require [`segment_model::V2`]
/// (beta).
pub mod segment_model {
    pub const V1: &str = "v1.0-20250506";
    pub const V2: &str = "v2.0-20260430";
}

/// `segmentation_granularity` of `POST /v3/mesh/segment`. The server
/// defaults to [`segmentation_granularity::BALANCED`].
pub mod segmentation_granularity {
    pub const SIMPLE: &str = "simple";
    pub const BALANCED: &str = "balanced";
    pub const DETAILED: &str = "detailed";
}

/// `completion_mode` of `POST /v3/mesh/complete`.
pub mod completion_mode {
    /// Fills missing regions with generated geometry (default).
    pub const AI_COMPLETION: &str = "ai_completion";
    /// Closes open boundaries without AI generation.
    pub const QUICK_CAP: &str = "quick_cap";
}

/// `fbx_preset` compatibility profile of `POST /v3/models/convert`. The
/// server defaults to [`fbx_preset::BLENDER`].
pub mod fbx_preset {
    pub const BLENDER: &str = "blender";
    pub const MAX_3DS: &str = "3dsmax";
    pub const MIXAMO: &str = "mixamo";
    /// Bakes the scale transform into the geometry instead of keeping it on
    /// the node transform.
    pub const BAKE_SCALE: &str = "bake_scale";
}

/// Output format for rig / retarget tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnimOutFormat {
    Glb,
    Fbx,
}

impl fmt::Display for AnimOutFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AnimOutFormat::Glb => "glb",
            AnimOutFormat::Fbx => "fbx",
        };
        f.write_str(s)
    }
}

/// `model` values accepted by the 3D generation endpoints.
/// These are intentionally plain strings (not a closed enum) since Tripo3D
/// regularly ships new model versions.
pub mod model_version {
    pub const H3_1: &str = "v3.1-20260211";
    pub const H3_0: &str = "v3.0-20250812";
    pub const H2_5: &str = "v2.5-20250123";
    pub const P1: &str = "P1-20260311";
    /// Next-generation P series. The only model that accepts `quad`, and
    /// unlike P1 it supports face limits up to 50,000 triangles (25,000
    /// with `quad`). Preview.
    pub const P2: &str = "P2-20260801";
}

/// `model` values accepted by `POST /v3/generation/text-to-image` and
/// `POST /v3/generation/image-to-image`.
///
/// `chat_image_1` and `chat_image_1.5` are omitted deliberately — they
/// retire on 2026-10-23 and 2026-12-01 respectively. Pass them as a raw
/// string during migration if you still need them.
pub mod image_model {
    /// The text-to-image default; image-to-image rejects it.
    pub const SEEDREAM_V4: &str = "seedream_v4";
    pub const SEEDREAM_V5: &str = "seedream_v5";
    pub const BANANA: &str = "banana";
    pub const BANANA_PRO: &str = "banana_pro";
    pub const BANANA2: &str = "banana2";
    pub const CHAT_IMAGE_2: &str = "chat_image_2";
    pub const CHAT_IMAGE_2_5_FLARE: &str = "chat_image_2.5_flare";
    pub const CHAT_IMAGE_2_5_SUNBURST: &str = "chat_image_2.5_sunburst";
}

/// `quality` render tiers for image generation.
///
/// Only `chat_image_2` and the 2.5 models accept this parameter; any other
/// model rejects the request outright. Omitting it is equivalent to
/// [`LOW`](image_quality::LOW) — note this differs from OpenAI's own `auto`
/// default, which is not supported here.
///
/// [`XHIGH`](image_quality::XHIGH) and [`MAX`](image_quality::MAX) are
/// exclusive to the 2.5 models. [`HIGH`](image_quality::HIGH) and above cost
/// extra credits and take noticeably longer.
pub mod image_quality {
    pub const LOW: &str = "low";
    pub const MEDIUM: &str = "medium";
    pub const HIGH: &str = "high";
    pub const XHIGH: &str = "xhigh";
    pub const MAX: &str = "max";
}

/// `background` handling modes, supported only by the 2.5 models. Other
/// models ignore the field rather than rejecting the request.
/// [`TRANSPARENT`](image_background::TRANSPARENT) requires
/// [`image_format::PNG`].
pub mod image_background {
    pub const AUTO: &str = "auto";
    pub const OPAQUE: &str = "opaque";
    pub const TRANSPARENT: &str = "transparent";
}

/// `output_format` of a generated image.
pub mod image_format {
    pub const PNG: &str = "png";
    pub const JPEG: &str = "jpeg";
}

/// `aspect_ratio` of a generated image. Only the banana models accept it;
/// seedream and chat_image size their output via `size` instead.
///
/// The 1:8, 1:4, 4:1 and 8:1 ratios are exclusive to `banana2`.
pub mod aspect_ratio {
    pub const R1X1: &str = "1:1";
    pub const R2X3: &str = "2:3";
    pub const R3X2: &str = "3:2";
    pub const R3X4: &str = "3:4";
    pub const R4X3: &str = "4:3";
    pub const R4X5: &str = "4:5";
    pub const R5X4: &str = "5:4";
    pub const R9X16: &str = "9:16";
    pub const R16X9: &str = "16:9";
    pub const R21X9: &str = "21:9";
    pub const R1X8: &str = "1:8";
    pub const R1X4: &str = "1:4";
    pub const R4X1: &str = "4:1";
    pub const R8X1: &str = "8:1";
}

/// `template` presets for image generation. Setting one makes `prompt`
/// optional.
///
/// [`ASSET_EXTRACTION`](image_template::ASSET_EXTRACTION) is accepted only by
/// text-to-image and [`ENHANCE_3D`](image_template::ENHANCE_3D) only by
/// image-to-image; the rest work on both.
pub mod image_template {
    pub const ASSET_EXTRACTION: &str = "asset_extraction";
    pub const CHARACTER_COMPLETION: &str = "character_completion";
    pub const T_POSE: &str = "t_pose";
    pub const VARIANTS: &str = "variants";
    pub const FIGURE: &str = "figure";
    pub const ENHANCE_3D: &str = "3d_enhance";
}

/// `export_orientation` (forward axis) of a generated model.
///
/// It applies to that generation only. If the model will be fed into
/// post-processing (texture, rig, retarget, convert), leave this unset and
/// reorient in the last step via `convert_model` instead — a wrongly
/// oriented post-processing result still reports success rather than
/// raising an error.
pub mod export_orientation {
    pub const PLUS_X: &str = "+x";
    pub const MINUS_X: &str = "-x";
    pub const PLUS_Y: &str = "+y";
    pub const MINUS_Y: &str = "-y";
}

/// `texture_quality` levels for 3D generation and re-texturing.
pub mod texture_quality {
    /// Requires [`texture_version::V3_5`](crate::constants::texture_version::V3_5);
    /// other texture models reject it with code 1004 rather than falling back.
    pub const FAST: &str = "fast";
    pub const STANDARD: &str = "standard";
    pub const DETAILED: &str = "detailed";
    pub const EXTREME: &str = "extreme";
}

/// `geometry_quality` levels for 3D generation. Only effective for model
/// versions >= [`model_version::H3_0`]; do not send it with
/// [`model_version::H2_5`].
pub mod geometry_quality {
    pub const STANDARD: &str = "standard";
    pub const DETAILED: &str = "detailed";
}

/// `texture_alignment` priority for image- and multiview-based generation.
pub mod texture_alignment {
    pub const ORIGINAL_IMAGE: &str = "original_image";
    pub const GEOMETRY: &str = "geometry";
}

/// `orientation` of a generated model relative to the input image. Only
/// effective when texturing is enabled.
pub mod orientation {
    pub const DEFAULT: &str = "default";
    pub const ALIGN_IMAGE: &str = "align_image";
}

/// The four canonical camera angles used by the multiview endpoints.
pub mod view {
    pub const FRONT: &str = "front";
    pub const LEFT: &str = "left";
    pub const BACK: &str = "back";
    pub const RIGHT: &str = "right";
}

/// Model conversion output formats accepted by `POST /v3/models/convert`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutputFormat {
    #[serde(rename = "GLTF")]
    Gltf,
    #[serde(rename = "GLB")]
    Glb,
    #[serde(rename = "USDZ")]
    Usdz,
    #[serde(rename = "FBX")]
    Fbx,
    #[serde(rename = "OBJ")]
    Obj,
    #[serde(rename = "STL")]
    Stl,
    #[serde(rename = "3MF")]
    ThreeMf,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OutputFormat::Gltf => "GLTF",
            OutputFormat::Glb => "GLB",
            OutputFormat::Usdz => "USDZ",
            OutputFormat::Fbx => "FBX",
            OutputFormat::Obj => "OBJ",
            OutputFormat::Stl => "STL",
            OutputFormat::ThreeMf => "3MF",
        };
        f.write_str(s)
    }
}

/// Texture image formats supported by the conversion API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextureFormat {
    #[serde(rename = "BMP")]
    Bmp,
    #[serde(rename = "DPX")]
    Dpx,
    #[serde(rename = "HDR")]
    Hdr,
    #[serde(rename = "JPEG")]
    Jpeg,
    #[serde(rename = "OPEN_EXR")]
    OpenExr,
    #[serde(rename = "PNG")]
    Png,
    #[serde(rename = "TARGA")]
    Targa,
    #[serde(rename = "TIFF")]
    Tiff,
    #[serde(rename = "WEBP")]
    Webp,
}
