//! Payload-shape tests for the 0.4.0 parameter changes — no real network
//! access. Each test records the JSON bodies a `wiremock` server receives.

use serde_json::{json, Value};
use std::time::Duration;
use tripo3d_sdk::constants::{
    completion_mode, decimate_model, export_orientation, fbx_preset, model_version, rig_model,
    segment_model, segmentation_granularity, texture_quality, texture_version, view,
};
use tripo3d_sdk::models::{FileDescriptor, FileInput, TexturePrompt, ViewImage};
use tripo3d_sdk::params::{
    CompleteMeshParams, ConvertModelParams, DecimateMeshParams, ImageToModelParams,
    MultiviewToModelParams, RigModelParams, SegmentMeshParams, TextToModelParams,
    TextureModelParams,
};
use tripo3d_sdk::{ClientOptions, Error, TaskStatus, TripoClient, MAX_LIST_TASKS};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn recording_server(reply: Value) -> (MockServer, TripoClient) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply))
        .mount(&server)
        .await;
    let client = TripoClient::new(ClientOptions {
        api_key: Some("test-key".into()),
        base_url: Some(server.uri()),
        timeout: Some(Duration::from_secs(5)),
        retries: Some(0),
        ..Default::default()
    })
    .unwrap();
    (server, client)
}

async fn task_server() -> (MockServer, TripoClient) {
    recording_server(json!({ "code": 0, "data": { "task_id": "task_new" } })).await
}

/// `(path, body)` of every request the server has received, in order.
async fn bodies(server: &MockServer) -> Vec<(String, Value)> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|r| (r.url.path().to_string(), r.body_json().unwrap()))
        .collect()
}

fn assert_invalid(result: tripo3d_sdk::Result<String>, what: &str) {
    match result {
        Err(Error::InvalidArgument(_)) => {}
        other => panic!("{what}: expected InvalidArgument, got {other:?}"),
    }
}

#[tokio::test]
async fn list_tasks_returns_tasks_keyed_by_id_and_missed() {
    let (server, client) = recording_server(json!({
        "code": 0,
        "status": "success",
        "data": {
            "tasks": {
                "task_a": {
                    "task_id": "task_a", "type": "animate_rig", "status": "success", "progress": 100,
                    "output": { "model_url": "https://cdn.example/rig.fbx" }, "credits_consumed": 25
                }
            },
            "missed": ["task_gone"]
        }
    }))
    .await;

    let list = client
        .list_tasks(&["task_a".into(), "task_gone".into()])
        .await
        .unwrap();
    let task = &list.tasks["task_a"];
    assert_eq!(task.status, TaskStatus::Success);
    assert_eq!(
        task.primary_model_url(),
        Some("https://cdn.example/rig.fbx")
    );
    assert_eq!(list.missed, vec!["task_gone".to_string()]);
    assert_eq!(
        bodies(&server).await,
        vec![(
            "/tasks/list".into(),
            json!({ "task_ids": ["task_a", "task_gone"] })
        )]
    );
}

#[tokio::test]
async fn list_tasks_rejects_more_than_the_server_limit() {
    let (server, client) = task_server().await;
    let ids = vec!["task".to_string(); MAX_LIST_TASKS + 1];
    assert!(matches!(
        client.list_tasks(&ids).await,
        Err(Error::InvalidArgument(_))
    ));
    assert!(bodies(&server).await.is_empty());
}

#[tokio::test]
async fn texture_model_sends_a_nested_texture_prompt() {
    let (server, client) = task_server().await;
    client
        .texture_model(TextureModelParams {
            model: Some(texture_version::V3_5.into()),
            delight: Some(false),
            texture_prompt: Some(TexturePrompt::text("red paint").with_style_image("file_style")),
            ..TextureModelParams::new("task_src")
        })
        .await
        .unwrap();
    client
        .texture_model(TextureModelParams {
            texture_prompt: Some(TexturePrompt::images([
                ViewImage::new(view::FRONT, "file_front"),
                ViewImage::new(view::BACK, FileInput::Url("https://ex.com/back.png".into())),
            ])),
            ..TextureModelParams::new("task_src")
        })
        .await
        .unwrap();

    let got = bodies(&server).await;
    assert_eq!(
        got[0].1,
        json!({
            "input": "task_src", "model": "v3.5-20260815", "delight": false,
            "texture_prompt": { "text": "red paint", "style_image": "file_style" }
        })
    );
    assert_eq!(
        got[1].1,
        json!({
            "input": "task_src",
            "texture_prompt": { "images": [
                { "front": "file_front" },
                { "back": { "url": "https://ex.com/back.png" } }
            ] }
        })
    );
}

#[tokio::test]
#[allow(deprecated)]
async fn texture_model_moves_deprecated_fields_into_texture_prompt() {
    let (server, client) = task_server().await;
    client
        .texture_model(TextureModelParams {
            text_prompt: Some("red paint".into()),
            style_image: Some(FileDescriptor {
                file_token: Some("file_style".into()),
                ..Default::default()
            }),
            ..TextureModelParams::new("task_src")
        })
        .await
        .unwrap();
    assert_eq!(
        bodies(&server).await[0].1,
        json!({
            "input": "task_src",
            "texture_prompt": { "text": "red paint", "style_image": { "file_token": "file_style" } }
        })
    );
}

#[tokio::test]
#[allow(deprecated)]
async fn texture_model_rejects_invalid_texture_prompts() {
    let (server, client) = task_server().await;
    let with = |prompt: TexturePrompt| TextureModelParams {
        texture_prompt: Some(prompt),
        ..TextureModelParams::new("x")
    };
    let cases = [
        (
            "text and image",
            with(TexturePrompt {
                image: Some("f".into()),
                ..TexturePrompt::text("a")
            }),
        ),
        (
            "style without text",
            with(TexturePrompt::image("f").with_style_image("s")),
        ),
        (
            "images without front",
            with(TexturePrompt::images([
                ViewImage::new(view::LEFT, "l"),
                ViewImage::new(view::BACK, "b"),
            ])),
        ),
        (
            "single image view",
            with(TexturePrompt::images([ViewImage::new(view::FRONT, "f")])),
        ),
        (
            "deprecated with new",
            TextureModelParams {
                text_prompt: Some("a".into()),
                ..with(TexturePrompt::text("b"))
            },
        ),
    ];
    for (what, params) in cases {
        assert_invalid(client.texture_model(params).await, what);
    }
    assert!(bodies(&server).await.is_empty());
}

#[tokio::test]
async fn rig_model_defaults_to_v3_and_keeps_an_explicit_model() {
    let (server, client) = task_server().await;
    client
        .rig_model(RigModelParams::new("task_src"))
        .await
        .unwrap();
    client
        .rig_model(RigModelParams {
            model: Some(rig_model::V1_0.into()),
            ..RigModelParams::new("task_src")
        })
        .await
        .unwrap();
    let mut via_extra = RigModelParams::new("task_src");
    via_extra
        .extra
        .insert("model".into(), json!(rig_model::V2_5));
    client.rig_model(via_extra).await.unwrap();

    let models: Vec<Value> = bodies(&server)
        .await
        .into_iter()
        .map(|(_, b)| b["model"].clone())
        .collect();
    assert_eq!(
        models,
        vec![
            json!("v3.0-20260909"),
            json!("v1.0-20240301"),
            json!("v2.5-20260210")
        ]
    );
}

#[tokio::test]
async fn generation_defaults_to_h3_1_and_keeps_an_explicit_model() {
    let (server, client) = task_server().await;
    client
        .text_to_model(TextToModelParams::new("a cat"))
        .await
        .unwrap();
    client
        .text_to_model(TextToModelParams {
            model: Some(model_version::P1.into()),
            ..TextToModelParams::new("a cat")
        })
        .await
        .unwrap();
    let mut via_extra = ImageToModelParams::new("file_x");
    via_extra
        .extra
        .insert("model".into(), json!(model_version::H3_0));
    client.image_to_model(via_extra).await.unwrap();
    client
        .image_to_model(ImageToModelParams::new("file_x"))
        .await
        .unwrap();
    client
        .multiview_to_model(MultiviewToModelParams::from_view_images([
            ViewImage::new(view::FRONT, "file_front"),
            ViewImage::new(view::BACK, "file_back"),
        ]))
        .await
        .unwrap();

    let models: Vec<Value> = bodies(&server)
        .await
        .into_iter()
        .map(|(_, b)| b["model"].clone())
        .collect();
    assert_eq!(
        models,
        vec![
            json!("v3.1-20260211"),
            json!("P1-20260311"),
            json!("v3.0-20250812"),
            json!("v3.1-20260211"),
            json!("v3.1-20260211")
        ]
    );
}

#[tokio::test]
async fn multiview_to_model_sends_view_keyed_inputs() {
    let (server, client) = task_server().await;
    client
        .multiview_to_model(MultiviewToModelParams {
            texture_version: Some(texture_version::V3_5.into()),
            texture_quality: Some(texture_quality::FAST.into()),
            ..MultiviewToModelParams::from_view_images([
                ViewImage::new(view::FRONT, "file_front"),
                ViewImage::new(view::RIGHT, "file_right"),
            ])
        })
        .await
        .unwrap();
    assert_eq!(
        bodies(&server).await[0].1,
        json!({
            "inputs": [{ "front": "file_front" }, { "right": "file_right" }],
            "model": "v3.1-20260211",
            "texture_version": "v3.5-20260815",
            "texture_quality": "fast"
        })
    );
}

#[tokio::test]
async fn multiview_to_model_rejects_invalid_views() {
    let (server, client) = task_server().await;
    let front = || ViewImage::new(view::FRONT, "f");
    let cases = [
        ("unknown view", vec![front(), ViewImage::new("top", "t")]),
        ("duplicate view", vec![front(), front()]),
        (
            "empty image",
            vec![front(), ViewImage::new(view::LEFT, FileInput::Empty)],
        ),
        (
            "no front",
            vec![
                ViewImage::new(view::LEFT, "l"),
                ViewImage::new(view::BACK, "b"),
            ],
        ),
    ];
    for (what, views) in cases {
        assert_invalid(
            client
                .multiview_to_model(MultiviewToModelParams::from_view_images(views))
                .await,
            what,
        );
    }
    assert!(bodies(&server).await.is_empty());
}

#[tokio::test]
async fn new_post_processing_parameters_are_serialized() {
    let (server, client) = task_server().await;
    client
        .image_to_model(ImageToModelParams {
            texture_version: Some(texture_version::V3_5.into()),
            delight: Some(false),
            ..ImageToModelParams::new("file_img")
        })
        .await
        .unwrap();
    client
        .convert_model(ConvertModelParams {
            export_vertex_colors: Some(true),
            scale_factor: Some(0.01),
            animate_in_place: Some(true),
            export_orientation: Some(export_orientation::MINUS_Y.into()),
            fbx_preset: Some(fbx_preset::BAKE_SCALE.into()),
            ..ConvertModelParams::new("task_src", "FBX")
        })
        .await
        .unwrap();
    client
        .segment_mesh(SegmentMeshParams {
            model: Some(segment_model::V2.into()),
            segmentation_granularity: Some(segmentation_granularity::DETAILED.into()),
            split_by_connectivity: Some(false),
            ref_image: Some("file_mask".into()),
            ..SegmentMeshParams::new("task_src")
        })
        .await
        .unwrap();
    client
        .complete_mesh(CompleteMeshParams {
            completion_mode: Some(completion_mode::QUICK_CAP.into()),
            ..CompleteMeshParams::new("task_src")
        })
        .await
        .unwrap();
    client
        .decimate_mesh(DecimateMeshParams {
            model: Some(decimate_model::V3_0.into()),
            smart_uv: Some(true),
            ..DecimateMeshParams::new("task_src")
        })
        .await
        .unwrap();

    assert_eq!(
        bodies(&server).await,
        vec![
            (
                "/generation/image-to-model".into(),
                json!({ "input": "file_img", "model": "v3.1-20260211", "texture_version": "v3.5-20260815", "delight": false })
            ),
            (
                "/models/convert".into(),
                json!({
                    "input": "task_src", "format": "FBX", "export_vertex_colors": true,
                    "scale_factor": 0.01, "animate_in_place": true,
                    "export_orientation": "-y", "fbx_preset": "bake_scale"
                })
            ),
            (
                "/mesh/segment".into(),
                json!({
                    "input": "task_src", "model": "v2.0-20260430",
                    "segmentation_granularity": "detailed", "split_by_connectivity": false,
                    "ref_image": "file_mask"
                })
            ),
            (
                "/mesh/complete".into(),
                json!({ "input": "task_src", "completion_mode": "quick_cap" })
            ),
            (
                "/mesh/decimate".into(),
                json!({ "input": "task_src", "model": "v3.0", "smart_uv": true })
            ),
        ]
    );
}
