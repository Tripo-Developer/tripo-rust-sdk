//! Hermetic integration tests for `TripoClient` — no real network access.
//! `wiremock` stands in for `openapi.tripo3d.com`.

use serde_json::json;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tripo3d_sdk::constants::{
    export_orientation, image_background, image_format, image_model, image_quality, model_version,
    view,
};
use tripo3d_sdk::models::{FileInput, MultiviewPrompt};
use tripo3d_sdk::params::{
    EditMultiviewParams, ImageToImageParams, ImageToModelParams, MeshUvParams,
    MultiviewToModelParams, RetargetAnimationParams, RigCheckParams, RigModelParams,
    TextToImageParams, TextToModelParams,
};
use tripo3d_sdk::{ClientOptions, DownloadedModel, Error, TaskStatus, TripoClient, WaitOptions};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// Returns "running" for the first two calls, then "success" — used to
/// deterministically exercise the WaitForTask polling loop without relying
/// on wiremock's mock-matching priority across overlapping mocks.
struct StatusSequence {
    calls: AtomicU32,
}

impl Respond for StatusSequence {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let status = if n >= 3 { "success" } else { "running" };
        ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_xyz", "type": "text_to_model", "status": status, "progress": n * 30 }
        }))
    }
}

async fn test_client(server: &MockServer) -> TripoClient {
    TripoClient::new(ClientOptions {
        api_key: Some("test-key".into()),
        base_url: Some(server.uri()),
        timeout: Some(Duration::from_secs(5)),
        retries: Some(1),
        ..Default::default()
    })
    .unwrap()
}

#[tokio::test]
async fn constructor_requires_api_key() {
    let saved = std::env::var("TRIPO_API_KEY").ok();
    std::env::remove_var("TRIPO_API_KEY");
    let result = TripoClient::new(ClientOptions::default());
    assert!(matches!(result, Err(Error::InvalidArgument(_))));
    if let Some(v) = saved {
        std::env::set_var("TRIPO_API_KEY", v);
    }
}

#[tokio::test]
async fn text_to_model_posts_expected_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/generation/text-to-model"))
        .and(header("Authorization", "Bearer test-key"))
        .and(body_json(
            json!({ "prompt": "a cat", "model": "v3.1-20260211", "texture": true }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_123" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let id = client
        .text_to_model(TextToModelParams {
            prompt: "a cat".into(),
            model: Some("v3.1-20260211".into()),
            texture: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(id, "task_123");
}

#[tokio::test]
async fn text_to_model_rejects_empty_prompt() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;
    let err = client
        .text_to_model(TextToModelParams::new(""))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[tokio::test]
async fn image_to_model_sends_a_bare_input_string() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/generation/image-to-model"))
        .and(body_json(json!({
            "input": "https://ex.com/a.png",
            "model": "v3.1-20260211"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_img" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let id = client
        .image_to_model(ImageToModelParams::new("https://ex.com/a.png"))
        .await
        .unwrap();
    assert_eq!(id, "task_img");
}

#[tokio::test]
async fn image_to_model_sends_an_explicit_input_object() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/generation/image-to-model"))
        .and(body_json(json!({
            "input": { "url": "https://ex.com/a.png" },
            "model": model_version::P2,
            "export_orientation": export_orientation::MINUS_Y,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_img" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let id = client
        .image_to_model(ImageToModelParams {
            model: Some(model_version::P2.to_string()),
            export_orientation: Some(export_orientation::MINUS_Y.to_string()),
            ..ImageToModelParams::new(FileInput::Url("https://ex.com/a.png".into()))
        })
        .await
        .unwrap();
    assert_eq!(id, "task_img");
}

#[tokio::test]
async fn image_to_model_rejects_missing_file() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;
    let err = client
        .image_to_model(ImageToModelParams::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[tokio::test]
async fn multiview_to_model_sends_positional_inputs() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/generation/multiview-to-model"))
        .and(body_json(json!({
            "inputs": ["front.png", "", "back.png", ""],
            "model": model_version::H3_1,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_mv" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let id = client
        .multiview_to_model(MultiviewToModelParams {
            model: Some(model_version::H3_1.to_string()),
            ..MultiviewToModelParams::from_views([
                Some(FileInput::from("front.png")),
                None,
                Some(FileInput::from("back.png")),
                None,
            ])
        })
        .await
        .unwrap();
    assert_eq!(id, "task_mv");
}

#[tokio::test]
async fn multiview_to_model_reuses_a_task_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/generation/multiview-to-model"))
        .and(body_json(
            json!({ "inputs": [{ "task_id": "task_mv_src" }], "model": "v3.1-20260211" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_mv" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let id = client
        .multiview_to_model(MultiviewToModelParams::from_task_id("task_mv_src"))
        .await
        .unwrap();
    assert_eq!(id, "task_mv");
}

#[tokio::test]
async fn multiview_to_model_rejects_invalid_inputs() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    for params in [
        MultiviewToModelParams::default(),
        MultiviewToModelParams::from_views([None, Some(FileInput::from("left.png")), None, None]),
        MultiviewToModelParams::from_views([Some(FileInput::from("front.png")), None, None, None]),
    ] {
        let err = client.multiview_to_model(params).await.unwrap_err();
        assert!(matches!(err, Error::InvalidArgument(_)));
    }
}

#[tokio::test]
async fn image_to_image_sends_a_model_and_multiple_inputs() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/generation/image-to-image"))
        .and(body_json(json!({
            "inputs": ["https://ex.com/a.png", "ftok-b"],
            "prompt": "use image[1] and image[2]",
            "model": image_model::SEEDREAM_V5,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_i2i" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let id = client
        .image_to_image(ImageToImageParams {
            model: Some(image_model::SEEDREAM_V5.to_string()),
            ..ImageToImageParams::from_inputs(
                [
                    FileInput::from("https://ex.com/a.png"),
                    FileInput::from("ftok-b"),
                ],
                "use image[1] and image[2]",
            )
        })
        .await
        .unwrap();
    assert_eq!(id, "task_i2i");
}

#[tokio::test]
async fn image_to_image_rejects_invalid_inputs() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    for params in [
        ImageToImageParams {
            prompt: Some("x".into()),
            ..Default::default()
        },
        ImageToImageParams {
            inputs: Some(vec![FileInput::from("b")]),
            ..ImageToImageParams::new("a", "x")
        },
        ImageToImageParams {
            input: Some(FileInput::from("a")),
            ..Default::default()
        },
    ] {
        let err = client.image_to_image(params).await.unwrap_err();
        assert!(matches!(err, Error::InvalidArgument(_)));
    }
}

#[tokio::test]
async fn text_to_image_sends_the_new_image_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/generation/text-to-image"))
        .and(body_json(json!({
            "prompt": "a glass sneaker",
            "model": image_model::CHAT_IMAGE_2_5_SUNBURST,
            "quality": image_quality::MAX,
            "background": image_background::TRANSPARENT,
            "output_format": image_format::PNG,
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_t2i" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let id = client
        .text_to_image(TextToImageParams {
            model: Some(image_model::CHAT_IMAGE_2_5_SUNBURST.to_string()),
            quality: Some(image_quality::MAX.to_string()),
            background: Some(image_background::TRANSPARENT.to_string()),
            output_format: Some(image_format::PNG.to_string()),
            ..TextToImageParams::new("a glass sneaker")
        })
        .await
        .unwrap();
    assert_eq!(id, "task_t2i");
}

#[tokio::test]
async fn edit_multiview_sends_per_view_prompts() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/generation/edit-multiview"))
        .and(body_json(json!({
            "input": "task_mv_src",
            "prompts": [
                { "prompt": "make the shirt red", "view": view::FRONT },
                { "prompt": "add a logo", "view": view::BACK },
            ],
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_edit" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let id = client
        .edit_multiview(EditMultiviewParams::new(
            "task_mv_src",
            [
                MultiviewPrompt::new("make the shirt red", view::FRONT),
                MultiviewPrompt::new("add a logo", view::BACK),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(id, "task_edit");
}

#[tokio::test]
async fn edit_multiview_rejects_invalid_prompts() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    for params in [
        EditMultiviewParams::new("", [MultiviewPrompt::new("x", view::FRONT)]),
        EditMultiviewParams::new("task_mv_src", []),
        EditMultiviewParams::new(
            "task_mv_src",
            (0..5).map(|_| MultiviewPrompt::new("x", view::FRONT)),
        ),
    ] {
        let err = client.edit_multiview(params).await.unwrap_err();
        assert!(matches!(err, Error::InvalidArgument(_)));
    }
}

#[tokio::test]
async fn wait_for_task_polls_until_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tasks/task_xyz"))
        .respond_with(StatusSequence {
            calls: AtomicU32::new(0),
        })
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let mut statuses = Vec::new();
    let task = client
        .wait_for_task_with_progress(
            "task_xyz",
            WaitOptions {
                poll_interval: Duration::from_millis(1),
                ..Default::default()
            },
            |t| statuses.push(t.status),
        )
        .await
        .unwrap();
    assert_eq!(task.status, TaskStatus::Success);
    assert!(statuses.last() == Some(&TaskStatus::Success));
}

#[tokio::test]
async fn wait_for_task_raises_task_error_on_failure() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tasks/task_fail"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_fail", "type": "text_to_model", "status": "failed", "error_code": 42, "error_msg": "nope" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let err = client
        .wait_for_task("task_fail", WaitOptions::default())
        .await
        .unwrap_err();
    match err {
        Error::Task { task } => {
            assert_eq!(task.error_code, Some(42));
        }
        other => panic!("expected Error::Task, got {other:?}"),
    }
}

#[tokio::test]
async fn wait_for_task_respects_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tasks/task_slow"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_slow", "type": "text_to_model", "status": "running", "progress": 5 }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let err = client
        .wait_for_task(
            "task_slow",
            WaitOptions {
                poll_interval: Duration::from_millis(5),
                timeout: Some(Duration::from_millis(20)),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Timeout { .. }));
}

#[tokio::test]
async fn api_error_response_raises_error_api() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/generation/text-to-model"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 2010,
            "message": "Insufficient credits",
            "suggestion": "Please top up your account",
            "request_id": "req_123"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let err = client
        .text_to_model(TextToModelParams::new("a cat"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("(request_id=req_123)"), "{err}");
    match err {
        Error::Api {
            code,
            message,
            request_id,
            ..
        } => {
            assert_eq!(code, 2010);
            assert_eq!(message.as_deref(), Some("Insufficient credits"));
            assert_eq!(request_id.as_deref(), Some("req_123"));
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn rig_check_rig_model_and_retarget_build_correct_payloads() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/animations/rig-check"))
        .and(body_json(json!({ "input": "task_src" })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "code": 0, "data": { "task_id": "chk_1" } })),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/animations/rig"))
        .and(body_json(
            json!({ "input": "task_src", "rig_type": "biped", "spec": "mixamo", "model": "v3.0-20260909" }),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "code": 0, "data": { "task_id": "rig_1" } })),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/animations/retarget"))
        .and(body_json(
            json!({ "input": "rig_1", "animations": ["preset:walk", "preset:idle"] }),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "code": 0, "data": { "task_id": "anim_1" } })),
        )
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let chk = client
        .rig_check(RigCheckParams::new("task_src"))
        .await
        .unwrap();
    let rig = client
        .rig_model(RigModelParams {
            rig_type: Some("biped".into()),
            spec: Some("mixamo".into()),
            ..RigModelParams::new("task_src")
        })
        .await
        .unwrap();
    let anim = client
        .retarget_animation(RetargetAnimationParams {
            animations: Some(vec!["preset:walk".into(), "preset:idle".into()]),
            ..RetargetAnimationParams::new(rig.clone())
        })
        .await
        .unwrap();

    assert_eq!(chk, "chk_1");
    assert_eq!(rig, "rig_1");
    assert_eq!(anim, "anim_1");
}

#[tokio::test]
async fn retarget_animation_rejects_more_than_five_animations() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;
    let err = client
        .retarget_animation(RetargetAnimationParams {
            animations: Some(vec!["preset:walk".into(); 6]),
            ..RetargetAnimationParams::new("x")
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[tokio::test]
async fn mesh_uv_posts_payload_and_exposes_uv_render() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/mesh/uv"))
        .and(body_json(json!({
            "input": "file_abc",
            "bake": false,
            "export_orientation": export_orientation::PLUS_Y,
            "compress": "geometry",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "task_id": "task_uv" }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/tasks/task_uv"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": {
                "task_id": "task_uv",
                "type": "mesh_uv",
                "status": "success",
                "progress": 100,
                "output": {
                    "model_url": "https://cdn.example/model.glb",
                    "uv_render_url": "https://cdn.example/uv_render.webp"
                }
            }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let id = client
        .mesh_uv(MeshUvParams {
            bake: Some(false),
            export_orientation: Some(export_orientation::PLUS_Y.to_string()),
            compress: Some("geometry".into()),
            ..MeshUvParams::new("file_abc")
        })
        .await
        .unwrap();
    let task = client.get_task(&id).await.unwrap();
    let output = task.output.unwrap();
    assert_eq!(
        output.uv_render_url.as_deref(),
        Some("https://cdn.example/uv_render.webp")
    );

    let err = client.mesh_uv(MeshUvParams::new(" ")).await.unwrap_err();
    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[tokio::test]
async fn get_balance_parses_the_standard_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/balance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "balance": 12.5, "frozen": 0.0 }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let balance = client.get_balance().await.unwrap();
    assert_eq!(balance.balance, 12.5);
}

#[tokio::test]
async fn upload_file_returns_file_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": { "file_token": "ftok-123" }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let uploaded = client
        .upload_file(vec![1, 2, 3], "a.png", Some("image/png"))
        .await
        .unwrap();
    assert_eq!(uploaded.file_token, "ftok-123");
}

#[tokio::test]
async fn download_model_fetches_the_primary_model_url() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/blob/model.glb"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(vec![0x67, 0x6c, 0x54, 0x46])
                .insert_header("Content-Type", "model/gltf-binary"),
        )
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let task: tripo3d_sdk::Task = serde_json::from_value(json!({
        "task_id": "t1",
        "type": "text_to_model",
        "status": "success",
        "output": { "model_url": format!("{}/blob/model.glb", server.uri()) }
    }))
    .unwrap();

    let downloaded = client.download_model(&task).await.unwrap().unwrap();
    assert_eq!(downloaded.data, vec![0x67, 0x6c, 0x54, 0x46]);
    assert_eq!(
        downloaded.content_type.as_deref(),
        Some("model/gltf-binary")
    );
}

#[tokio::test]
async fn http_500_triggers_retries_then_request_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/balance"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal boom"))
        .mount(&server)
        .await;

    let client = test_client(&server).await;
    let err = client.get_balance().await.unwrap_err();
    assert!(matches!(err, Error::Request { .. }));
}

#[test]
fn downloaded_model_extension_tracks_the_url() {
    // quad=true generations return FBX, so the extension cannot be assumed.
    for (url, want_ext, want_name) in [
        ("https://cdn/a/model.glb", Some("glb"), "out.glb"),
        (
            "https://cdn/a/model.fbx?auth_key=1-abc-0-def",
            Some("fbx"),
            "out.fbx",
        ),
        ("https://cdn/a/model.USDZ#frag", Some("usdz"), "out.usdz"),
        ("https://cdn/a/model", None, "out.glb"),
        ("https://cdn/a.b/model?x=1", None, "out.glb"),
    ] {
        let d = DownloadedModel {
            url: url.to_string(),
            content_type: None,
            data: Vec::new(),
        };
        assert_eq!(d.extension().as_deref(), want_ext, "extension of {url}");
        assert_eq!(d.filename("out"), want_name, "filename of {url}");
    }
}

// ──────────────────── Retry safety (billing-sensitive) ────────────────────
//
// Task-creation endpoints are billed per submission, so a POST must never be
// replayed once the server may have seen it. These tests pin the exact number
// of times the request reaches the server.

use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

fn retry_client(base_url: String) -> TripoClient {
    TripoClient::new(ClientOptions {
        api_key: Some("test-key".into()),
        base_url: Some(base_url),
        timeout: Some(Duration::from_secs(5)),
        retries: Some(2),
        ..Default::default()
    })
    .unwrap()
}

async fn billable_call(client: &TripoClient) -> Result<String, Error> {
    client
        .image_to_image(ImageToImageParams {
            prompt: Some("x".into()),
            input: Some(FileInput::Url("https://example.com/a.png".into())),
            ..Default::default()
        })
        .await
}

fn is_indeterminate(err: &Error) -> bool {
    matches!(
        err,
        Error::Request {
            indeterminate: true,
            ..
        }
    )
}

/// A listener that reads each request and then drops the socket without
/// replying, which surfaces to the caller as a reset mid-flight.
async fn dropping_server() -> (String, Arc<AtomicU32>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicU32::new(0));
    let counter = hits.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let counter = counter.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                // One read is enough to know the request headers arrived.
                let _ = socket.read(&mut buf).await;
                counter.fetch_add(1, Ordering::SeqCst);
                drop(socket);
            });
        }
    });
    (format!("http://{addr}"), hits)
}

async fn status_server(status: u16) -> (MockServer, String) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(status).set_body_json(json!({
            "code": 1000, "message": "nope"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(status).set_body_json(json!({
            "code": 1000, "message": "nope"
        })))
        .mount(&server)
        .await;
    let uri = server.uri();
    (server, uri)
}

#[tokio::test]
async fn billable_post_is_not_replayed_when_connection_drops_mid_flight() {
    let (base_url, hits) = dropping_server().await;
    let client = retry_client(base_url);

    let err = billable_call(&client).await.unwrap_err();

    assert_eq!(hits.load(Ordering::SeqCst), 1, "must submit exactly once");
    assert!(
        is_indeterminate(&err),
        "must be flagged indeterminate: {err}"
    );
    assert!(
        err.to_string().contains("billed twice"),
        "should warn about double billing: {err}"
    );
}

#[tokio::test]
async fn billable_post_is_replayed_when_server_declines_outright() {
    // 429 means the server refused the work, so a retry cannot double-bill.
    let (server, uri) = status_server(429).await;
    let client = retry_client(uri);

    let _ = billable_call(&client).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3, "want 3 attempts (retries: 2)");
}

#[tokio::test]
async fn billable_post_is_not_replayed_on_ambiguous_statuses() {
    for status in [500u16, 502, 504] {
        let (server, uri) = status_server(status).await;
        let client = retry_client(uri);

        let err = billable_call(&client).await.unwrap_err();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "HTTP {status}: must submit exactly once");
        assert!(
            is_indeterminate(&err),
            "HTTP {status}: must be flagged indeterminate"
        );
    }
}

#[tokio::test]
async fn idempotent_get_is_still_retried() {
    let (base_url, hits) = dropping_server().await;
    let client = retry_client(base_url);
    let err = client.get_task("t1").await.unwrap_err();
    assert_eq!(hits.load(Ordering::SeqCst), 3, "reads stay retryable");
    assert!(!is_indeterminate(&err), "reads are never indeterminate");

    for status in [500u16, 429] {
        let (server, uri) = status_server(status).await;
        let client = retry_client(uri);
        let _ = client.get_task("t1").await;
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 3, "HTTP {status}: reads stay retryable");
    }
}
