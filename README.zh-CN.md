# tripo3d-sdk（Rust 版）

[English](./README.md) · **简体中文**

Tripo 官方异步 **Rust SDK**，用于访问 [Tripo3D v3 API](https://developers.tripo3d.com/zh/docs/introduction) —— 覆盖 AI 3D 生成的完整能力：文生 3D、图生 3D、多视角生 3D、重贴图、网格编辑、自动绑骨与动画重定向。

- 基于 `tokio` + `reqwest`（使用 rustls，无需 OpenSSL）。
- 通过 `serde` 提供强类型的请求 / 响应模型。
- 对瞬态网络错误 / 5xx 自动重试，支持 `Retry-After`。
- 丰富的 `Error` 枚举（`Error::Api`、`Error::Task`、`Error::Timeout`、`Error::Request`……）。
- `wait_for_task` / `wait_for_task_with_progress` 轮询器。
- 与 [`tripo3d-sdk-js`](../tripo3d-sdk-js)、[`tripo3d-sdk-go`](../tripo3d-sdk-go) 是同源姊妹 SDK —— API 能力一致，各自遵循语言惯例。

> 国内 Base URL：`https://openapi.tripo3d.com/v3`  
> 海外 Base URL：`https://openapi.tripo3d.ai/v3`  
> 本 SDK 面向 **v3** REST 接口，**不是**旧的 `/v2/openapi/task` 接口。  
> 可通过 `base_url` 选择区域（见[客户端参数](#客户端参数)）。

---

## 安装

```toml
[dependencies]
tripo3d-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
```

基于 git / 本地路径进行开发调试时也可以这样写：

```toml
[dependencies]
tripo3d-sdk = { git = "https://github.com/VAST-AI-Research/tripo-rust-sdk.git" }
# tripo3d-sdk = { path = "../tripo3d-sdk-rust" }
```

先在 [Tripo 控制台](https://platform.tripo3d.com/) 创建 API Key 并导出（海外请使用 [platform.tripo3d.ai](https://platform.tripo3d.ai/)）：

```bash
export TRIPO_API_KEY="tsk_..."
```

## 快速开始

```rust
use tripo3d_sdk::{TripoClient, ClientOptions, WaitOptions, params::TextToModelParams, constants::model_version};

#[tokio::main]
async fn main() -> tripo3d_sdk::Result<()> {
    let client = TripoClient::new(ClientOptions::default())?; // 默认读取 TRIPO_API_KEY

    let task_id = client
        .text_to_model(TextToModelParams {
            prompt: "一只可爱的红熊猫，抱着一根竹子".into(),
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

    println!("模型下载地址：{:?}", task.primary_model_url());
    Ok(())
}
```

> ⚠️ 模型 URL 会在任务成功约 **5 分钟后过期**，请及时下载。可用 `client.download_model(&task)`。

---

## 客户端参数

```rust
pub struct ClientOptions {
    pub api_key: Option<String>,      // 默认读取 TRIPO_API_KEY 环境变量
    pub base_url: Option<String>,     // 国内：https://openapi.tripo3d.com/v3 · 海外：https://openapi.tripo3d.ai/v3
    pub timeout: Option<Duration>,    // 单次请求超时，默认 60 秒
    pub retries: Option<u32>,         // 5xx / 网络错误的额外重试次数，默认 2
    pub user_agent: Option<String>,
}
```

---

## API 一览

所有生成类方法都返回一个 `task_id`（`String`），随后可以用 `wait_for_task()` / `wait_for_task_with_progress()` 等待任务终态结果。

### 3D 生成

| 方法 | 端点 | 说明 |
| --- | --- | --- |
| `text_to_model(params)` | `POST /generation/text-to-model` | 文本 → 3D 模型 |
| `image_to_model(params)` | `POST /generation/image-to-model` | 单图 → 3D 模型 |
| `multiview_to_model(params)` | `POST /generation/multiview-to-model` | 4 视角 `[front, left, back, right]` → 3D 模型 |
| `text_to_image(params)` | `POST /generation/text-to-image` | 文生概念图 |
| `image_to_image(params)` | `POST /generation/image-to-image` | 图片风格转换 / 编辑 |
| `image_to_multiview(params)` | `POST /generation/image-to-multiview` | 单图 → 4 视角图集 |
| `edit_multiview(params)` | `POST /generation/edit-multiview` | 编辑已生成的多视角图集 |

### 模型后处理

| 方法 | 端点 | 说明 |
| --- | --- | --- |
| `texture_model(params)` | `POST /models/texture` | 为已有模型重新贴图 |
| `convert_model(params)` | `POST /models/convert` | 转换为 GLTF / FBX / OBJ / STL / USDZ / 3MF |
| `segment_mesh(params)` | `POST /mesh/segment` | 语义分割 |
| `complete_mesh(params)` | `POST /mesh/complete` | 网格补全 / 修复 |
| `decimate_mesh(params)` | `POST /mesh/decimate` | 减面 / 重拓扑 |
| `mesh_uv(params)` | `POST /mesh/uv` | 重新展 UV，并映射原有贴图 / 骨骼 / 动画；额外返回 UV 布局预览图 |

### 动画

| 方法 | 端点 | 说明 |
| --- | --- | --- |
| `rig_check(params)` | `POST /animations/rig-check` | 检查模型是否可绑骨、推荐骨骼类型 |
| `rig_model(params)` | `POST /animations/rig` | 自动绑骨 |
| `retarget_animation(params)` | `POST /animations/retarget` | 应用预设动画 |

### 工具

| 方法 | 端点 | 说明 |
| --- | --- | --- |
| `get_task(task_id)` | `GET /tasks/{task_id}` | 查询单个任务 |
| `list_tasks(task_ids)` | `POST /tasks/list` | 批量查询任务 |
| `wait_for_task(task_id, opts)` | — | 轮询直到任务进入终态 |
| `wait_for_task_with_progress(task_id, opts, cb)` | — | 同上，附带进度回调 |
| `upload_file(bytes, filename, content_type)` | `POST /files` | 上传文件，返回 `file_token` |
| `get_balance()` | `GET /account/balance` | 查询账户余额 |
| `download_model(&task)` | — | 把任务的主模型 URL 下载为 `Vec<u8>` |

---

## 传入图片 / 文件

所有接收图片或模型的接口都通过 [`FileInput`] 传入。裸字符串会原样透传，由服务端推断它是什么 —— 公开 URL、`file_token`，或是需要复用其产物的上游任务 `task_id`。如果不想依赖推断，可以用显式变体：

```rust
use tripo3d_sdk::{FileInput, FileDescriptor, ObjectRef};

let a: FileInput = "https://example.com/hero.png".into();   // 公开 URL
let b: FileInput = "8f2a4c...".into();                       // file_token
let e: FileInput = previous_task_id.as_str().into();         // 复用上游任务的产物
let c: FileInput = FileDescriptor { url: Some("https://example.com/a.png".into()), ..Default::default() }.into();
let d: FileInput = FileDescriptor {
    object: Some(ObjectRef { bucket: "tripo-data".into(), key: "uploads/abc.png".into() }),
    ..Default::default()
}.into();
```

上传本地文件获取 `file_token`：

```rust
let bytes = tokio::fs::read("./hero.png").await?;
let uploaded = client.upload_file(bytes, "hero.png", Some("image/png")).await?;

let task_id = client
    .image_to_model(tripo3d_sdk::params::ImageToModelParams::new(uploaded.file_token))
    .await?;
```

任务串联无需下载再上传，直接把上游 `task_id` 传进去即可：

```rust
use tripo3d_sdk::constants::{image_model, model_version};
use tripo3d_sdk::params::{ImageToModelParams, TextToImageParams};

let image_id = client
    .text_to_image(TextToImageParams {
        model: Some(image_model::SEEDREAM_V5.to_string()),
        ..TextToImageParams::new("一个低面数木质藏宝箱")
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

---

## 端到端流水线：游戏就绪角色

```rust
use tripo3d_sdk::{
    ClientOptions, TripoClient, WaitOptions,
    constants::model_version,
    params::{ImageToModelParams, RigCheckParams, RigModelParams, RetargetAnimationParams},
};

let client = TripoClient::new(ClientOptions::default())?;

// 1. 图生 3D（P 系列低面拓扑，游戏/移动端友好）
let model_id = client
    .image_to_model(ImageToModelParams {
        model: Some(model_version::P2.to_string()),
        face_limit: Some(5000),
        texture: Some(true),
        ..ImageToModelParams::new("https://example.com/hero.png")
    })
    .await?;
client.wait_for_task(&model_id, WaitOptions::default()).await?;

// 2. 检查是否可绑骨
let check_id = client.rig_check(RigCheckParams::new(model_id.as_str())).await?;
let check = client.wait_for_task(&check_id, WaitOptions::default()).await?;
let output = check.output.clone().unwrap_or_default();
assert!(output.riggable.unwrap_or(false), "该模型不可绑骨");

// 3. 自动绑骨（Mixamo 命名 → 可直接导入 Unity / Unreal）
let rig_id = client
    .rig_model(RigModelParams {
        rig_type: output.rig_type.clone(),
        spec: Some("mixamo".into()),
        ..RigModelParams::new(model_id.as_str())
    })
    .await?;
client.wait_for_task(&rig_id, WaitOptions::default()).await?;

// 4. 烘焙预设动画
let anim_id = client
    .retarget_animation(RetargetAnimationParams {
        animations: Some(vec!["preset:idle".into(), "preset:walk".into(), "preset:run".into()]),
        out_format: Some("glb".into()),
        ..RetargetAnimationParams::new(rig_id.as_str())
    })
    .await?;
let anim = client.wait_for_task(&anim_id, WaitOptions::default()).await?;

println!("带动画的 GLB URLs：{:?}", anim.output.and_then(|o| o.model_urls));
```

**开发者小贴士：**
- 绑骨前**务必**先调 `rig_check`，可以拿到推荐的 `rig_type` 并避免直接调 `rig` 失败。
- 轮询频率建议 **2s** 一次，**不要超过 1 次/秒**，否则可能被限流。
- Unity / Unreal 使用 `spec: "mixamo"`；自建流水线用 `spec: "tripo"`。
- 一次 `retarget_animation` 最多传 **5 个** 预设动画（超过会在本地直接报错，不会发出请求）。

---

## 错误处理

```rust
use tripo3d_sdk::Error;

match client.text_to_model(params).await {
    Ok(task_id) => { /* ... */ }
    Err(Error::Api { code, message, suggestion, .. }) => {
        eprintln!("API 错误 {code}：{message:?} — {suggestion:?}");
    }
    Err(Error::Task { task }) => {
        eprintln!("任务 {} 失败：{:?}", task.task_id, task.error_message);
    }
    Err(Error::Timeout { task_id, timeout_ms }) => {
        eprintln!("任务 {task_id} 在 {timeout_ms}ms 内未完成");
    }
    Err(Error::Request { status, body, .. }) => {
        eprintln!("传输错误：HTTP {status:?} — {body:?}");
    }
    Err(e) => eprintln!("{e}"),
}
```

### 常见错误码

| code | 含义 | 建议 |
| --- | --- | --- |
| `0` | 成功 | — |
| `1xxx` | 参数 / 认证错误 | 检查 `api_key` 与请求参数 |
| `2010` | 积分不足 | 前往控制台充值 |
| `429` | 请求过于频繁 | 降低并发或延长轮询间隔（SDK 会自动退避重试） |
| `5xx` | 服务端错误 | SDK 会自动重试，仍失败请稍后再试 |

---

### 重试与重复提交

任务创建接口按提交次数计费,因此 SDK 绝不会重放一个服务端可能已经收下的请求。只有在**能证明请求从未被处理**时才重试——连接被拒绝、DNS 解析失败,或服务端明确返回 `429` / `503`。而语义不明的失败(发送途中连接被重置、超时、`500` / `502` / `504`)会让非幂等请求立即失败;幂等的读请求则照常重试。

任务创建失败且状态不明时,错误上会带标记,让你能区分"确定失败"和"状态不确定":

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

此时应先去任务列表核对,不要盲目重发——盲目重试正是重复扣费的根源。

## 常量枚举

```rust
use tripo3d_sdk::{TaskStatus, Animation, RigType, RigSpec, OutputFormat};
use tripo3d_sdk::constants::{image_model, model_version};

TaskStatus::Success;
Animation::Walk.as_str();          // "preset:walk"
RigType::Biped;                    // 序列化为 "biped"
RigSpec::Mixamo;                   // 序列化为 "mixamo"
model_version::H3_1;               // "v3.1-20260211"
model_version::P2;                 // "P2-20260801"
image_model::SEEDREAM_V5;          // "seedream_v5"
image_model::CHAT_IMAGE_2_5_SUNBURST; // "chat_image_2.5_sunburst"
OutputFormat::Fbx;                 // 序列化为 "FBX"
```

### 3D 生成模型

| 常量 | 取值 | 说明 |
| --- | --- | --- |
| `model_version::H3_1` | `v3.1-20260211` | 最新，质量最佳（默认） |
| `model_version::H3_0` | `v3.0-20250812` | 稳定版，支持高级特性 |
| `model_version::H2_5` | `v2.5-20250123` | 旧版本，不支持 `geometry_quality` |
| `model_version::P1` | `P1-20260311` | 低面数，干净拓扑 |
| `model_version::P2` | `P2-20260801` | 新一代 P 系列，支持四边面输出。preview 版 |

P 系列中只有 `model_version::P2` 支持 `quad`，传给 `P1` 会返回 `400`。P1 同样不支持 `smart_low_poly`、`generate_parts` 和 `geometry_quality`。

另外，开启 `quad` 会把输出格式强制为 **FBX** 而非 GLB，所以请从 `model_url` 推导扩展名，不要假定是 `.glb` —— 用 `downloaded.filename(name)` 即可。

### 生图模型

用于 `text_to_image` 与 `image_to_image`。

| 常量 | 取值 | 说明 |
| --- | --- | --- |
| `image_model::SEEDREAM_V5` | `seedream_v5` | 最强编辑、风格迁移与多图融合 |
| `image_model::BANANA` | `banana` | 快速 |
| `image_model::BANANA_PRO` | `banana_pro` | 更高质量 |
| `image_model::BANANA2` | `banana2` | 最新快速选项 |
| `image_model::CHAT_IMAGE_2` | `chat_image_2` | 质量最佳 |
| `image_model::CHAT_IMAGE_2_5_FLARE` | `chat_image_2.5_flare` | 2.5 系列速度档 |
| `image_model::CHAT_IMAGE_2_5_SUNBURST` | `chat_image_2.5_sunburst` | 2.5 系列精修档 |

部分参数是分模型的：`quality` 仅 `chat_image_2` 和两个 2.5 模型支持（其它模型传入会直接报错），`background` 仅两个 2.5 模型支持，`aspect_ratio` 仅 banana 系列支持 —— seedream 和 chat_image 请改用 `size` 控制出图尺寸。

`chat_image_1` 与 `chat_image_1.5` 已被有意移除：它们将分别于 2026-10-23 和 2026-12-01 下线。迁移期间如果仍需使用，可直接传字符串字面量。

---

## 运行示例

```bash
export TRIPO_API_KEY="tsk_..."

cargo run --example text_to_model -- "一个木质藏宝箱"
cargo run --example image_to_model -- ./hero.png
cargo run --example rig_and_animate -- https://example.com/hero.png
```

## 开发

```bash
cargo build
cargo test      # 完全 hermetic —— 用 wiremock 模拟 HTTP 层，无需真实 API Key
```

源码结构：

```
src/
  lib.rs         # 公共导出
  client.rs      # TripoClient —— 所有 API 方法
  http.rs        # reqwest 封装（重试 + envelope 解析）
  error.rs       # Error 枚举
  constants.rs   # 枚举（TaskStatus、Animation、RigType……）
  models.rs      # Task、TaskOutput、Balance、FileDescriptor……
  params.rs      # 每个端点的请求参数结构体
examples/        # 端到端示例
tests/           # 基于 wiremock 的集成测试
```

---

## 相关链接

- API 文档：https://developers.tripo3d.com/zh/docs/introduction
- API 端点（国内）：`https://openapi.tripo3d.com/v3`
- API 端点（海外）：`https://openapi.tripo3d.ai/v3`

## 许可协议

MIT —— 见 `LICENSE`。
