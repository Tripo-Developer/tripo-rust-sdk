//! Hermetic tests for the 0.5.0 endpoints, uploads to storage and webhook
//! verification.

use serde_json::json;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tripo3d_sdk::constants::{seg_type, smart_segment_granularity};
use tripo3d_sdk::params::{
    ImageToSplatParams, ImportModelParams, SmartSegmentParams, IDENTITY_TRANSFORM,
};
use tripo3d_sdk::webhook::{self, WebhookOptions};
use tripo3d_sdk::{ClientOptions, Error, TaskStatus, TripoClient, UsageOptions, MAX_USAGE_LIMIT};
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn client(server: &MockServer) -> TripoClient {
    TripoClient::new(ClientOptions {
        api_key: Some("test-key".into()),
        base_url: Some(server.uri()),
        timeout: Some(Duration::from_secs(5)),
        retries: Some(0),
        ..Default::default()
    })
    .unwrap()
}

fn created() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_json(json!({ "code": 0, "data": { "task_id": "task_new" } }))
}

#[tokio::test]
async fn new_task_endpoints_send_their_payloads() {
    let server = MockServer::start().await;
    for (route, body) in [
        (
            "/generation/image-to-splat",
            json!({ "input": "file_img", "model_seed": 7 }),
        ),
        ("/models/import", json!({ "input": "https://ex.com/a.fbx" })),
        (
            "/mesh/smartsegment",
            json!({ "seg_type": "image", "input": "file_img", "granularity": "fine", "hint": "sword" }),
        ),
        (
            "/mesh/smartsegment",
            json!({ "seg_type": "model", "input": "file_glb",
                    "transform": [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0] }),
        ),
    ] {
        Mock::given(method("POST"))
            .and(path(route))
            .and(body_json(body))
            .respond_with(created())
            .expect(1)
            .mount(&server)
            .await;
    }
    let c = client(&server);

    c.image_to_splat(ImageToSplatParams {
        model_seed: Some(7),
        ..ImageToSplatParams::new("file_img")
    })
    .await
    .unwrap();
    c.import_model(ImportModelParams::new("https://ex.com/a.fbx"))
        .await
        .unwrap();
    c.smart_segment(SmartSegmentParams {
        granularity: Some(smart_segment_granularity::FINE.into()),
        hint: Some("sword".into()),
        ..SmartSegmentParams::image("file_img")
    })
    .await
    .unwrap();
    c.smart_segment(SmartSegmentParams::model("file_glb", IDENTITY_TRANSFORM))
        .await
        .unwrap();
}

#[tokio::test]
async fn smart_segment_rejects_invalid_params_without_calling_the_api() {
    let server = MockServer::start().await;
    let c = client(&server);
    let cases = [
        SmartSegmentParams {
            input: "file_x".into(),
            ..Default::default()
        },
        SmartSegmentParams {
            seg_type: "mesh".into(),
            ..SmartSegmentParams::image("file_x")
        },
        SmartSegmentParams::image(" "),
        SmartSegmentParams {
            seg_type: seg_type::MODEL.into(),
            ..SmartSegmentParams::image("file_x")
        },
    ];
    for params in cases {
        let err = c.smart_segment(params.clone()).await.unwrap_err();
        assert!(
            matches!(err, Error::InvalidArgument(_)),
            "{params:?}: {err}"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn smart_segment_output_is_the_primary_model() {
    let task: tripo3d_sdk::Task = serde_json::from_value(json!({
        "task_id": "t", "type": "smart_segment", "status": "success",
        "output": { "seg_model_url": "https://cdn/seg.glb", "mask_url": "https://cdn/mask.png",
                    "prompt": "head, body", "seg_task_id": "s", "model_task_id": "m" }
    }))
    .unwrap();
    assert_eq!(task.primary_model_url(), Some("https://cdn/seg.glb"));
    let output = task.output.unwrap();
    assert_eq!(output.mask_url.as_deref(), Some("https://cdn/mask.png"));
    assert_eq!(output.model_task_id.as_deref(), Some("m"));
}

#[tokio::test]
async fn get_usage_pages_with_limit_and_offset() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/usage"))
        .and(query_param("limit", "5"))
        .and(query_param("offset", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": [{ "task_id": "t1", "type": "animate_rig", "status": "success",
                       "credits_consumed": 25, "created_at": "2026-10-20T09:35:19.372280Z" }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server);

    let records = c
        .get_usage(UsageOptions {
            limit: Some(5),
            offset: Some(10),
        })
        .await
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].task_type, "animate_rig");
    assert_eq!(records[0].status, TaskStatus::Success);
    assert_eq!(records[0].credits_consumed, 25.0);

    for limit in [0, MAX_USAGE_LIMIT + 1] {
        let err = c
            .get_usage(UsageOptions {
                limit: Some(limit),
                offset: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidArgument(_)));
    }
}

async fn mount_presign(server: &MockServer, storage_path: &str) {
    Mock::given(method("POST"))
        .and(path("/files/presign"))
        .and(body_json(json!({ "format": "glb" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "data": {
                "presigned_url": format!("{}{storage_path}?X-Amz-Signature=abc", server.uri()),
                "file_token": "file_big",
                "expires_in": 1800
            }
        })))
        .mount(server)
        .await;
}

fn assert_storage_put(request: &Request, body: &[u8]) {
    assert_eq!(request.method.as_str(), "PUT");
    assert_eq!(request.url.query(), Some("X-Amz-Signature=abc"));
    assert!(
        request.headers.get("authorization").is_none(),
        "the PUT leaked the API key"
    );
    assert_eq!(
        request.headers.get("content-type").unwrap(),
        "application/octet-stream"
    );
    assert_eq!(
        request.headers.get("content-length").unwrap(),
        &body.len().to_string()
    );
    assert!(request.headers.get("transfer-encoding").is_none());
    assert_eq!(request.body, body);
}

#[tokio::test]
async fn upload_large_file_presigns_then_puts_without_the_api_key() {
    let server = MockServer::start().await;
    mount_presign(&server, "/bucket/a.glb").await;
    Mock::given(method("PUT"))
        .and(path("/bucket/a.glb"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let uploaded = client(&server)
        .upload_large_file(b"glTF-bytes".to_vec(), ".GLB")
        .await
        .unwrap();
    assert_eq!(uploaded.file_token, "file_big");
    let requests = server.received_requests().await.unwrap();
    assert_storage_put(&requests[1], b"glTF-bytes");
}

#[tokio::test]
async fn upload_large_file_from_path_streams_with_a_content_length() {
    let server = MockServer::start().await;
    mount_presign(&server, "/bucket/b.glb").await;
    Mock::given(method("PUT"))
        .and(path("/bucket/b.glb"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let file = std::env::temp_dir().join(format!("tripo-upload-{}.GLB", std::process::id()));
    let data = vec![42u8; 300_000];
    std::fs::write(&file, &data).unwrap();

    let result = client(&server)
        .upload_large_file_from_path(&file, None)
        .await;
    std::fs::remove_file(&file).ok();
    assert_eq!(result.unwrap().file_token, "file_big");
    let requests = server.received_requests().await.unwrap();
    assert_storage_put(&requests[1], &data);
}

#[tokio::test]
async fn upload_large_file_surfaces_storage_errors() {
    let server = MockServer::start().await;
    mount_presign(&server, "/bucket/c.glb").await;
    Mock::given(method("PUT"))
        .and(path("/bucket/c.glb"))
        .respond_with(
            ResponseTemplate::new(403).set_body_string("<Error><Code>AccessDenied</Code></Error>"),
        )
        .mount(&server)
        .await;

    let err = client(&server)
        .upload_large_file(vec![1, 2, 3], "glb")
        .await
        .unwrap_err();
    match err {
        Error::Request { status, body, .. } => {
            assert_eq!(status, Some(403));
            assert!(body.unwrap().contains("AccessDenied"));
        }
        other => panic!("expected a 403 Request error, got {other}"),
    }
}

// Signed with: printf '%s' "1714992000.<body>" | openssl dgst -sha256 -hmac whsec_test_secret
const SECRET: &str = "whsec_test_secret";
const BODY: &str = r#"{"id":"task_abc123:task.completed","type":"task.completed","created_at":"2026-05-06T12:00:00Z","data":{"task_id":"task_abc123","type":"text_to_model","status":"success","output":{"model_url":"https://cdn.tripo3d.ai/output/model_pbr.glb"}}}"#;
const SIG: &str = "bcf3ecafcc87d73c1035ee9e31917393c8b15afcad1fba34b2fe4557db766b45";

fn at(seconds: u64) -> WebhookOptions {
    WebhookOptions {
        now: Some(UNIX_EPOCH + Duration::from_secs(seconds)),
        ..Default::default()
    }
}

#[test]
fn verify_webhook_accepts_a_valid_signature() {
    let header = format!("t=1714992000,v1={SIG}");
    let event =
        webhook::verify_webhook(BODY.as_bytes(), &header, SECRET, at(1714992000 + 60)).unwrap();
    assert_eq!(event.event_type, webhook::TASK_COMPLETED);
    assert_eq!(event.id, "task_abc123:task.completed");
    let task = event.task().unwrap();
    assert_eq!(
        task.primary_model_url(),
        Some("https://cdn.tripo3d.ai/output/model_pbr.glb")
    );

    let rotated = format!("t=1714992000,v1={},v1={SIG}", "0".repeat(64));
    webhook::verify_webhook(BODY.as_bytes(), &rotated, SECRET, at(1714992000)).unwrap();
    let no_replay_check = WebhookOptions {
        tolerance: Some(Duration::MAX),
        now: Some(SystemTime::now()),
    };
    webhook::verify_webhook(BODY.as_bytes(), &header, SECRET, no_replay_check).unwrap();
}

#[test]
fn verify_webhook_rejects_tampered_stale_or_malformed_deliveries() {
    let header = format!("t=1714992000,v1={SIG}");
    let tampered = BODY.replace("success", "failed");
    let cases: [(&str, &str, String, &str, WebhookOptions); 9] = [
        (
            "tampered body",
            &tampered,
            header.clone(),
            SECRET,
            at(1714992000),
        ),
        (
            "wrong secret",
            BODY,
            header.clone(),
            "whsec_other",
            at(1714992000),
        ),
        ("stale", BODY, header.clone(), SECRET, at(1714992000 + 301)),
        (
            "from the future",
            BODY,
            header.clone(),
            SECRET,
            at(1714992000 - 301),
        ),
        (
            "no timestamp",
            BODY,
            format!("v1={SIG}"),
            SECRET,
            at(1714992000),
        ),
        (
            "no signature",
            BODY,
            "t=1714992000".into(),
            SECRET,
            at(1714992000),
        ),
        (
            "bad hex",
            BODY,
            "t=1714992000,v1=zz".into(),
            SECRET,
            at(1714992000),
        ),
        ("empty header", BODY, String::new(), SECRET, at(1714992000)),
        (
            "shifted timestamp",
            BODY,
            header.replace("1714992000", "1714992001"),
            SECRET,
            at(1714992000),
        ),
    ];
    for (name, body, header, secret, opts) in cases {
        let err = webhook::verify_webhook(body.as_bytes(), &header, secret, opts).unwrap_err();
        assert!(matches!(err, Error::WebhookSignature(_)), "{name}: {err}");
    }
    assert!(matches!(
        webhook::verify_webhook(BODY.as_bytes(), &header, "", at(1714992000)),
        Err(Error::InvalidArgument(_))
    ));
}
