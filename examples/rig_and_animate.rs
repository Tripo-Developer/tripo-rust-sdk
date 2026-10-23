//! examples/rig_and_animate.rs
//!
//! End-to-end "game-ready character" pipeline:
//!
//!   image-to-model  ->  rig-check  ->  rig  ->  retarget(walk, idle, run)
//!
//!   $ cargo run --example rig_and_animate -- https://example.com/hero.png

use std::env;
use tokio::fs;
use tripo3d_sdk::{
    constants::{model_version, rig_model},
    params::{ImageToModelParams, RetargetAnimationParams, RigCheckParams, RigModelParams},
    ClientOptions, TripoClient, WaitOptions,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let image_url = env::args().nth(1).unwrap_or_else(|| {
        "https://raw.githubusercontent.com/Tripo-Developer/tripo-rust-sdk/main/assets/character.jpg"
            .to_string()
    });

    let client = TripoClient::new(ClientOptions::default())?;

    // 1. Generate a base 3D model — the P1 line has clean, low-poly topology.
    let model_task_id = client
        .image_to_model(ImageToModelParams {
            model: Some(model_version::P2.to_string()),
            face_limit: Some(5000),
            texture: Some(true),
            ..ImageToModelParams::new(image_url)
        })
        .await?;
    stage("image-to-model", &client, &model_task_id).await?;

    // 2. Check whether the model is riggable and what skeleton type fits.
    let check_task_id = client
        .rig_check(RigCheckParams::new(model_task_id.as_str()))
        .await?;
    let check_task = stage("rig-check", &client, &check_task_id).await?;

    let output = check_task.output.clone().unwrap_or_default();
    let riggable = output.riggable.unwrap_or(false);
    let rig_type = output
        .rig_type
        .clone()
        .unwrap_or_else(|| "biped".to_string());
    if !riggable {
        anyhow::bail!("Model is not riggable (rig_type={rig_type}). Aborting.");
    }
    println!("  -> riggable=true, rig_type={rig_type}");

    // 3. Attach the skeleton (use Mixamo naming so it drops into Unity/Unreal).
    let rig_task_id = client
        .rig_model(RigModelParams {
            model: Some(rig_model::V3_0.to_string()),
            rig_type: Some(rig_type),
            spec: Some("mixamo".to_string()),
            ..RigModelParams::new(model_task_id.as_str())
        })
        .await?;
    stage("rig", &client, &rig_task_id).await?;

    // 4. Retarget preset animations.
    let anim_task_id = client
        .retarget_animation(RetargetAnimationParams {
            animations: Some(vec![
                "preset:idle".to_string(),
                "preset:walk".to_string(),
                "preset:run".to_string(),
            ]),
            out_format: Some("glb".to_string()),
            ..RetargetAnimationParams::new(rig_task_id.as_str())
        })
        .await?;
    let anim_task = stage("retarget", &client, &anim_task_id).await?;

    let urls = anim_task
        .output
        .as_ref()
        .and_then(|o| o.model_urls.clone())
        .unwrap_or_default();
    println!("\nAnimated model URLs:");
    for (i, u) in urls.iter().enumerate() {
        println!("  [{i}] {u}");
    }

    if let Some(downloaded) = client.download_model(&anim_task).await? {
        let filename = downloaded.filename(&format!("character-{anim_task_id}"));
        fs::write(&filename, &downloaded.data).await?;
        println!("> saved {filename} ({} bytes)", downloaded.data.len());
    }

    Ok(())
}

async fn stage(
    label: &str,
    client: &TripoClient,
    task_id: &str,
) -> anyhow::Result<tripo3d_sdk::Task> {
    println!("\n[{label}] task_id={task_id}");
    let task = client
        .wait_for_task_with_progress(task_id, WaitOptions::default(), |t| {
            print!("\r  {} — {}%   ", t.status, t.progress.unwrap_or(0));
        })
        .await?;
    println!();
    Ok(task)
}
