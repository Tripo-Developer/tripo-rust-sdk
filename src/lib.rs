//! `tripo3d-sdk` — the official Rust SDK for the [Tripo3D v3 API](https://developers.tripo3d.ai/en/docs/introduction).
//!
//! Global API base URL: `https://openapi.tripo3d.ai/v3`  
//! China API base URL: `https://openapi.tripo3d.com/v3`
//!
//! ```no_run
//! use tripo3d_sdk::{TripoClient, ClientOptions, params::TextToModelParams};
//!
//! # async fn run() -> tripo3d_sdk::Result<()> {
//! let client = TripoClient::new(ClientOptions::default())?; // reads TRIPO_API_KEY
//!
//! let task_id = client
//!     .text_to_model(TextToModelParams::new("a cute cat"))
//!     .await?;
//!
//! let task = client.wait_for_task(&task_id, Default::default()).await?;
//! println!("model url: {:?}", task.primary_model_url());
//! # Ok(())
//! # }
//! ```

mod client;
pub mod constants;
mod error;
pub mod models;
pub mod params;

mod http;

pub use client::MAX_LIST_TASKS;
pub use client::{ClientOptions, DownloadedModel, TripoClient, WaitOptions};
pub use constants::{
    aspect_ratio, completion_mode, decimate_model, export_orientation, fbx_preset,
    geometry_quality, image_background, image_format, image_model, image_quality, image_template,
    model_version, orientation, rig_model, segment_model, segmentation_granularity,
    texture_alignment, texture_quality, texture_version, view, AnimOutFormat, Animation,
    OutputFormat, RigSpec, RigType, TaskStatus, TextureFormat, DEFAULT_BASE_URL,
};
pub use error::{Error, Result};
pub use models::{
    Balance, FileDescriptor, FileInput, MultiviewPrompt, MultiviewTaskRef, ObjectRef, Task,
    TaskList, TaskOutput, TexturePrompt, UploadedFile, ViewImage,
};
