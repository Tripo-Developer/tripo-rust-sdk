# tripo3d-sdk (Rust)

**English** · [简体中文](./README.zh-CN.md)

The official, async **Rust SDK** for the [Tripo3D v3 API](https://developers.tripo3d.ai/en/docs/introduction) — a full AI 3D generation platform covering text-to-3D, image-to-3D, multiview-to-3D, re-texturing, mesh editing, auto-rigging and animation retargeting.

- Built on `tokio` + `reqwest` (rustls, no OpenSSL dependency).
- Strongly-typed request/response models via `serde`.
- Automatic retries on transient network / 5xx errors, honoring `Retry-After`.
- A rich `Error` enum (`Error::Api`, `Error::Task`, `Error::Timeout`, `Error::Request`, …).
- `wait_for_task` / `wait_for_task_with_progress` pollers.
- Sibling SDK to [`tripo3d-sdk-js`](../tripo3d-sdk-js) and [`tripo3d-sdk-go`](../tripo3d-sdk-go) — same API surface, idiomatic per language.

> Base URL (global): `https://openapi.tripo3d.ai/v3`  
> Base URL (China): `https://openapi.tripo3d.com/v3`  
> This SDK targets the **v3** REST API, not the older `/v2/openapi/task` endpoint.  
> Pass `base_url` to select your region (see [Client options](#client-options)).

---

## Installation

```toml
[dependencies]
tripo3d-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
```

Or from git / a local path while developing against this repo:

```toml
[dependencies]
tripo3d-sdk = { git = "https://github.com/Tripo-Developer/tripo-rust-sdk.git" }
# tripo3d-sdk = { path = "../tripo3d-sdk-rust" }
```

Create an API key on the [Tripo console](https://platform.tripo3d.ai/) and export it (use [platform.tripo3d.com](https://platform.tripo3d.com/) in China):

```bash
export TRIPO_API_KEY="tsk_..."
```

## Quick start

```rust
use tripo3d_sdk::{TripoClient, ClientOptions, WaitOptions, params::TextToModelParams, constants::model_version};

#[tokio::main]
async fn main() -> tripo3d_sdk::Result<()> {
    let client = TripoClient::new(ClientOptions {
        // reads TRIPO_API_KEY
        base_url: Some("https://openapi.tripo3d.ai/v3".into()), // use https://openapi.tripo3d.com/v3 in China
        ..Default::default()
    })?;

    let task_id = client
        .text_to_model(TextToModelParams {
            prompt: "a cute red panda holding bamboo".into(),
            model: Some(model_version::H3_1.to_string()),
            texture: Some(true),
            pbr: Some(true),
            texture_quality: Some("detailed".into()),
            ..Default::default()
        })
        .await?;

    let task = client
        .wait_for_task_with_progress(&task_id, WaitOptions::default(), |t| {
            println!("{} — {}%", t.status, t.progress.unwrap_or(0));
        })
        .await?;

    println!("Model URL: {:?}", task.primary_model_url());
    Ok(())
}
```

> ⚠️ Model URLs expire ~5 minutes after task completion — download them right away. See `client.download_model(&task)`.

---

## Client options

```rust
pub struct ClientOptions {
    pub api_key: Option<String>,      // defaults to TRIPO_API_KEY env var
    pub base_url: Option<String>,     // global: https://openapi.tripo3d.ai/v3 · China: https://openapi.tripo3d.com/v3
    pub timeout: Option<Duration>,    // per-request timeout, default 60s
    pub retries: Option<u32>,         // extra attempts on 5xx / network errors, default 2
    pub user_agent: Option<String>,
}
```

---

## API reference

Every generation method returns a `task_id` (`String`). Use `wait_for_task()` / `wait_for_task_with_progress()` to await the terminal result.

### Generation

| Method | Endpoint | Description |
| --- | --- | --- |
| `text_to_model(params)` | `POST /generation/text-to-model` | Text → 3D model |
| `image_to_model(params)` | `POST /generation/image-to-model` | Single image → 3D model |
| `multiview_to_model(params)` | `POST /generation/multiview-to-model` | 2–4 views (keyed by angle via `from_view_images`, or positional `[front, left, back, right]`) → 3D model |
| `text_to_image(params)` | `POST /generation/text-to-image` | Concept image from text |
| `image_to_image(params)` | `POST /generation/image-to-image` | Image style / edit |
| `image_to_multiview(params)` | `POST /generation/image-to-multiview` | Image → 4-view sheet |
| `edit_multiview(params)` | `POST /generation/edit-multiview` | Refine multiview output |
| `image_to_splat(params)` | `POST /generation/image-to-splat` | Single image → 3D Gaussian splat (`.splat`) |

### Model post-processing

| Method | Endpoint | Description |
| --- | --- | --- |
| `texture_model(params)` | `POST /models/texture` | Re-texture an existing model, guided by `texture_prompt` |
| `convert_model(params)` | `POST /models/convert` | Convert to GLTF / FBX / OBJ / STL / USDZ / 3MF |
| `import_model(params)` | `POST /models/import` | Bring an external GLB / GLTF / FBX / OBJ / STL into Tripo for post-processing (free) |
| `segment_mesh(params)` | `POST /mesh/segment` | Semantic segmentation |
| `smart_segment(params)` | `POST /mesh/smartsegment` | Segmentation pipeline from an image (`SmartSegmentParams::image`) or a GLB (`SmartSegmentParams::model`, with a transform); result in `output.seg_model_url` |
| `complete_mesh(params)` | `POST /mesh/complete` | Mesh completion / repair |
| `decimate_mesh(params)` | `POST /mesh/decimate` | Retopology / face-count reduction |
| `mesh_uv(params)` | `POST /mesh/uv` | Re-unwrap UVs, remapping textures / rig / animations; also returns a UV layout preview |

### Animation

| Method | Endpoint | Description |
| --- | --- | --- |
| `rig_check(params)` | `POST /animations/rig-check` | Detect whether a model is riggable |
| `rig_model(params)` | `POST /animations/rig` | Attach a skeleton (sends `rig_model::V3_0` when `model` is unset) |
| `retarget_animation(params)` | `POST /animations/retarget` | Apply preset animations |

### Utility

| Method | Endpoint | Description |
| --- | --- | --- |
| `get_task(task_id)` | `GET /tasks/{task_id}` | Fetch a task snapshot |
| `list_tasks(task_ids)` | `POST /tasks/list` | Batch-query up to 100 tasks; returns `tasks` keyed by task_id plus the `missed` IDs |
| `wait_for_task(task_id, opts)` | — | Poll until terminal state |
| `wait_for_task_with_progress(task_id, opts, cb)` | — | Same, with a progress callback |
| `upload_file(bytes, filename, content_type)` | `POST /files` | Upload a raw file and get a `file_token` |
| `upload_large_file_from_path(path, format)` | `POST /files/presign` + PUT | Stream a file on disk straight to storage; recommended over 60 MB |
| `upload_large_file(bytes, format)` | `POST /files/presign` + PUT | Same, from memory |
| `presign_upload(format)` | `POST /files/presign` | Reserve a `file_token` and a storage URL to PUT to |
| `get_balance()` | `GET /account/balance` | Account credit balance |
| `get_usage(opts)` | `GET /account/usage` | Credits consumed per task, newest first (`limit` up to 200, `offset`) |
| `download_model(&task)` | — | Download the primary model URL into a `Vec<u8>` |
| `webhook::verify_webhook(payload, header, secret, opts)` | — | Verify and decode a webhook delivery |

---

## Passing images / files

Every endpoint that takes an image or model accepts it as a [`FileInput`]. A bare string is forwarded untouched so the server can infer what it is — a public URL, a `file_token`, or the `task_id` of an earlier task whose output should be reused. Use the explicit variants when you would rather not rely on inference:

```rust
use tripo3d_sdk::{FileInput, FileDescriptor, ObjectRef};

let a: FileInput = "https://example.com/hero.png".into();   // a public URL
let b: FileInput = "8f2a4c...".into();                       // a file_token
let e: FileInput = previous_task_id.as_str().into();         // reuse a task's output
let c: FileInput = FileDescriptor { url: Some("https://example.com/a.png".into()), ..Default::default() }.into();
let d: FileInput = FileDescriptor {
    object: Some(ObjectRef { bucket: "tripo-data".into(), key: "uploads/abc.png".into() }),
    ..Default::default()
}.into();
```

Upload a local buffer to get a `file_token`:

```rust
let bytes = tokio::fs::read("./hero.png").await?;
let uploaded = client.upload_file(bytes, "hero.png", Some("image/png")).await?;

let task_id = client
    .image_to_model(tripo3d_sdk::params::ImageToModelParams::new(uploaded.file_token))
    .await?;
```

Chaining tasks needs no download-and-reupload round trip — pass the upstream
`task_id` straight in:

```rust
use tripo3d_sdk::constants::{image_model, model_version};
use tripo3d_sdk::params::{ImageToModelParams, TextToImageParams};

let image_id = client
    .text_to_image(TextToImageParams {
        model: Some(image_model::SEEDREAM_V5.to_string()),
        ..TextToImageParams::new("a low-poly wooden treasure chest")
    })
    .await?;
client.wait_for_task(&image_id, WaitOptions::default()).await?;

let model_id = client
    .image_to_model(ImageToModelParams {
        model: Some(model_version::P2.to_string()),
        ..ImageToModelParams::new(image_id.as_str())
    })
    .await?;
```

### Large files and external models

`upload_file` sends the file through the API server. For files over 60 MB, use
`upload_large_file_from_path`, which streams the file from disk straight to
storage; `format` defaults to the file's extension. Then `import_model` turns
an external model into a task that the post-processing endpoints accept:

```rust
use tripo3d_sdk::params::{ImportModelParams, RigModelParams};

let uploaded = client.upload_large_file_from_path("./character.fbx", None).await?;

let import_id = client.import_model(ImportModelParams::new(uploaded.file_token)).await?;
client.wait_for_task(&import_id, WaitOptions::default()).await?;
let rig_id = client.rig_model(RigModelParams::new(import_id)).await?;
```

---

## End-to-end pipeline: game-ready character

```rust
use tripo3d_sdk::{
    ClientOptions, TripoClient, WaitOptions,
    constants::{model_version, rig_model},
    params::{ImageToModelParams, RigCheckParams, RigModelParams, RetargetAnimationParams},
};

let client = TripoClient::new(ClientOptions::default())?;

// 1. Image -> 3D (low-poly P series topology, mobile/game friendly)
let model_id = client
    .image_to_model(ImageToModelParams {
        model: Some(model_version::P2.to_string()),
        face_limit: Some(5000),
        texture: Some(true),
        ..ImageToModelParams::new("https://example.com/hero.png")
    })
    .await?;
client.wait_for_task(&model_id, WaitOptions::default()).await?;

// 2. Verify skeleton compatibility
let check_id = client.rig_check(RigCheckParams::new(model_id.as_str())).await?;
let check = client.wait_for_task(&check_id, WaitOptions::default()).await?;
let output = check.output.clone().unwrap_or_default();
assert!(output.riggable.unwrap_or(false), "model is not riggable");

// 3. Attach skeleton (Mixamo-compatible bones -> Unity/Unreal ready)
let rig_id = client
    .rig_model(RigModelParams {
        model: Some(rig_model::V3_0.into()),
        rig_type: output.rig_type.clone(),
        spec: Some("mixamo".into()),
        ..RigModelParams::new(model_id.as_str())
    })
    .await?;
client.wait_for_task(&rig_id, WaitOptions::default()).await?;

// 4. Bake preset locomotion animations
let anim_id = client
    .retarget_animation(RetargetAnimationParams {
        animations: Some(vec!["preset:idle".into(), "preset:walk".into(), "preset:run".into()]),
        out_format: Some("glb".into()),
        ..RetargetAnimationParams::new(rig_id.as_str())
    })
    .await?;
let anim = client.wait_for_task(&anim_id, WaitOptions::default()).await?;

println!("Animated GLB URLs: {:?}", anim.output.and_then(|o| o.model_urls));
```

---

## Error handling

```rust
use tripo3d_sdk::Error;

match client.text_to_model(params).await {
    Ok(task_id) => { /* ... */ }
    Err(Error::Api { code, message, suggestion, request_id, .. }) => {
        eprintln!("API error {code}: {message:?} — {suggestion:?} (request_id={request_id:?})");
    }
    Err(Error::Task { task }) => {
        eprintln!("Task {} failed: {:?}", task.task_id, task.error_message);
    }
    Err(Error::Timeout { task_id, timeout_ms }) => {
        eprintln!("Gave up after {timeout_ms}ms — task {task_id}");
    }
    Err(Error::Request { status, body, .. }) => {
        eprintln!("Transport failure: HTTP {status:?} — {body:?}");
    }
    Err(e) => eprintln!("{e}"),
}
```

---

### Retries and duplicate submissions

Task-creation calls are billed per submission, so the SDK never replays a
request the server may already have accepted. A failure is retried only when
it proves the request was never processed — the connection was refused, DNS
failed, or the server answered `429` / `503`. Ambiguous failures (a reset
mid-flight, a timeout, `500` / `502` / `504`) end the call immediately for
non-idempotent requests, while idempotent reads keep retrying as before.

When a task-creation call fails ambiguously, the error is flagged so you can
tell "definitely failed" apart from "unknown":

```rust
match client.image_to_image(params).await {
    Err(Error::Request { indeterminate: true, .. }) => {
        // The submission may have gone through. Check list_tasks rather
        // than resubmitting.
    }
    Err(_) => { /* Definitely failed; safe to retry yourself. */ }
    Ok(task_id) => { /* … */ }
}
```

Reconcile against your task list before resubmitting; retrying blindly is
what causes double charges.

## Webhooks

Instead of polling, Tripo can POST `task.completed`, `task.failed` and
`balance.low` events to an endpoint configured in the console. Verify each
delivery against the endpoint's signing secret (`whsec_…`) before acting on
it, and pass the body exactly as received:

```rust
use tripo3d_sdk::webhook::{self, WebhookOptions};

// `body` is the raw request body, `signature` the Tripo-Webhook-Signature header.
match webhook::verify_webhook(&body, signature, &secret, WebhookOptions::default()) {
    Ok(event) if event.event_type == webhook::TASK_COMPLETED => {
        println!("{:?}", event.task()?.primary_model_url());
    }
    Ok(_) => {}
    Err(Error::WebhookSignature(_)) => { /* respond 400 */ }
    Err(e) => return Err(e),
}
```

`verify_webhook` also rejects deliveries whose timestamp is more than 5 minutes
from now, to stop replays. Failed deliveries are retried, so use the
`Tripo-Webhook-Delivery` header (`webhook::DELIVERY_HEADER`) to drop
duplicates.

## Constants

```rust
use tripo3d_sdk::{TaskStatus, Animation, RigType, RigSpec, OutputFormat};
use tripo3d_sdk::constants::{image_model, model_version};

TaskStatus::Success;
Animation::Walk.as_str();          // "preset:walk"
RigType::Biped;                    // serializes as "biped"
RigSpec::Mixamo;                   // serializes as "mixamo"
model_version::H3_1;               // "v3.1-20260211"
model_version::P2;                 // "P2-20260801"
image_model::SEEDREAM_V5;          // "seedream_v5"
image_model::CHAT_IMAGE_2_5_SUNBURST; // "chat_image_2.5_sunburst"
OutputFormat::Fbx;                 // serializes as "FBX"
```

### 3D generation models

| Constant | Value | Notes |
| --- | --- | --- |
| `model_version::H3_1` | `v3.1-20260211` | Latest, best quality. `text_to_model`, `image_to_model` and `multiview_to_model` send it when `model` is `None` |
| `model_version::H3_0` | `v3.0-20250812` | Stable, advanced features |
| `model_version::H2_5` | `v2.5-20250123` | Legacy; does not accept `geometry_quality` |
| `model_version::P1` | `P1-20260311` | Low-poly, clean topology |
| `model_version::P2` | `P2-20260801` | Next-gen P series, quad output. Preview |

`quad` is accepted only by `model_version::P2` within the P series — sending it with `P1` returns a `400`. P1 also rejects `smart_low_poly`, `generate_parts`, and `geometry_quality`. Enabling `quad` also forces the output format to **FBX** instead of GLB, so derive the file extension from `model_url` rather than assuming `.glb` — use `downloaded.filename(name)` for that.

### Texture, rigging and post-processing models

| Module | Used by | Notes |
| --- | --- | --- |
| `texture_version::{V3_5, V3_0, V2_5}` | `texture_version` on the generation endpoints, `model` on `texture_model` | `texture_quality::FAST` and `delight` require `V3_5` |
| `rig_model::{V1_0, V2_5, V3_0}` | `rig_model` | `rig_model` sends `V3_0` when `model` is unset. Only V3.0 accepts `RigType::Others` and the `ActorCore` / `Unity` / `Unreal` / `Vrm` specs |
| `decimate_model::{V1_0, V2_0, V3_0}` | `decimate_mesh` | Server default V2.0; `smart_uv` requires V3.0 |
| `segment_model::{V1, V2}` | `segment_mesh` | `segmentation_granularity`, `split_by_connectivity` and `ref_image` require V2 (beta) |
| `completion_mode::{AI_COMPLETION, QUICK_CAP}` | `complete_mesh` | AI completion (default) or a fast cap of open boundaries |
| `fbx_preset::{BLENDER, MAX_3DS, MIXAMO, BAKE_SCALE}` | `convert_model` | FBX compatibility profile |

`RigSpec` is `#[non_exhaustive]`, so `match` on it needs a wildcard arm.

Re-texturing takes a nested `TexturePrompt`. `text`, `image` and `images` are mutually exclusive, and `style_image` only goes with `text`:

```rust
use tripo3d_sdk::{constants::texture_version, params::TextureModelParams, TexturePrompt};

client
    .texture_model(TextureModelParams {
        model: Some(texture_version::V3_5.into()),
        texture_prompt: Some(TexturePrompt::text("weathered red paint")),
        ..TextureModelParams::new(model_id.as_str())
    })
    .await?;
```

The old top-level `text_prompt`, `image_prompt` and `style_image` fields are deprecated. The API ignores them at the top level, so `texture_model` moves them into `texture_prompt` for you; they will be removed in 1.0.

### Image generation models

Used by `text_to_image` and `image_to_image`.

| Constant | Value | Notes |
| --- | --- | --- |
| `image_model::SEEDREAM_V5` | `seedream_v5` | Strongest editing, style transfer, multi-image fusion |
| `image_model::BANANA` | `banana` | Fast |
| `image_model::BANANA_PRO` | `banana_pro` | Higher quality |
| `image_model::BANANA2` | `banana2` | Latest fast option |
| `image_model::CHAT_IMAGE_2` | `chat_image_2` | Best quality |
| `image_model::CHAT_IMAGE_2_5_FLARE` | `chat_image_2.5_flare` | 2.5 speed tier |
| `image_model::CHAT_IMAGE_2_5_SUNBURST` | `chat_image_2.5_sunburst` | 2.5 fidelity tier |

A few parameters are model-specific: `quality` is accepted only by `chat_image_2` and the 2.5 models (other models reject the request), `background` only by the 2.5 models, and `aspect_ratio` only by the banana models — seedream and chat_image size their output through `size` instead.

`chat_image_1` and `chat_image_1.5` are omitted deliberately: they retire on 2026-10-23 and 2026-12-01 respectively. Pass them as a raw string if you still need them during migration.

---

## Running the examples

```bash
export TRIPO_API_KEY="tsk_..."

cargo run --example text_to_model -- "a wooden treasure chest"
cargo run --example image_to_model -- ./hero.png
cargo run --example rig_and_animate -- https://example.com/hero.png
```

## Development

```bash
cargo build
cargo test      # hermetic — uses `wiremock` to stub the HTTP layer, no real API key needed
```

Source tree:

```
src/
  lib.rs         # public exports
  client.rs      # TripoClient — all API methods
  http.rs        # reqwest wrapper with retry + envelope parsing
  error.rs       # Error enum
  constants.rs   # enums (TaskStatus, Animation, RigType, …)
  models.rs      # Task, TaskOutput, Balance, FileDescriptor, …
  params.rs       # per-endpoint request parameter structs
examples/        # runnable end-to-end demos
tests/           # wiremock-backed integration tests
```

---

## Reference

- API docs: https://developers.tripo3d.ai/en/docs/introduction
- API base URL (global): `https://openapi.tripo3d.ai/v3`
- API base URL (China): `https://openapi.tripo3d.com/v3`

## License

MIT — see `LICENSE`.
