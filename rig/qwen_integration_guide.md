# 通义千问（Qwen）Rig 集成使用指南

本文档说明如何在 Rig 框架中使用通义千问（Qwen）供应商。

## 目录

1. [快速开始](#快速开始)
2. [基本用法](#基本用法)
3. [流式输出](#流式输出)
4. [工具调用](#工具调用)
5. [高级功能](#高级功能)
6. [支持的模型](#支持的模型)
7. [配置选项](#配置选项)
8. [示例代码](#示例代码)

---

## 快速开始

### 1. 安装依赖

在您的 `Cargo.toml` 中添加：

```toml
[dependencies]
rig-core = "0.21.0"
tokio = { version = "1", features = ["full"] }
```

### 2. 设置 API 密钥

```bash
export DASHSCOPE_API_KEY=your_api_key_here
```

### 3. 基本示例

```rust
use rig::{
    completion::Prompt,
    providers::qwen,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从环境变量创建客户端
    let client = qwen::Client::from_env();

    // 创建模型
    let model = client.completion_model(qwen::QWEN_PLUS);

    // 发送提示
    let response = model.prompt("你好，请介绍一下自己").await?;

    println!("回复：{}", response);

    Ok(())
}
```

---

## 基本用法

### 创建客户端

有三种方式创建通义千问客户端：

#### 方式 1：从环境变量创建

```rust
use rig::providers::qwen;

let client = qwen::Client::from_env();
```

#### 方式 2：直接传入 API 密钥

```rust
let client = qwen::Client::new("your_api_key");
```

#### 方式 3：使用构建器（高级配置）

```rust
let client = qwen::Client::builder("your_api_key")
    .base_url("https://custom.api.url")  // 自定义基础 URL
    .build()?;
```

### 创建完成模型

```rust
// 使用预定义的模型常量
let qwen_plus = client.completion_model(qwen::QWEN_PLUS);
let qwen_max = client.completion_model(qwen::QWEN_MAX);
let qwen_turbo = client.completion_model(qwen::QWEN_TURBO);

// 或使用自定义模型名称
let custom_model = client.completion_model("qwen-plus-2025-07-14");
```

### 发送简单提示

```rust
use rig::completion::Prompt;

let response = model.prompt("你是谁？").await?;
println!("{}", response);
```

### 多轮对话

```rust
use rig::completion::Chat;

let response = model.chat(
    "继续上面的话题",
    vec![
        Message::user("你好"),
        Message::assistant("你好！我是通义千问。"),
    ]
).await?;
```

---

## 流式输出

通义千问支持流式输出，可以实时获取生成的内容。

### 基本流式输出

```rust
use futures::StreamExt;
use rig::completion::CompletionRequest;

// 创建请求
let request = CompletionRequest::new("讲一个故事");

// 获取流式响应
let mut stream = model.stream(request).await?;

// 处理流式数据
while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(content) => {
            print!("{}", content);
            std::io::stdout().flush()?;
        }
        Err(e) => {
            eprintln!("错误：{}", e);
            break;
        }
    }
}
```

### 流式输出的优势

1. **更好的用户体验** - 用户可以立即看到生成的内容
2. **降低延迟感知** - 即使总时间相同，流式输出让用户感觉更快
3. **实时反馈** - 可以在生成过程中中断或调整

---

## 工具调用

通义千问支持 Function Calling，允许模型调用外部工具。

### 定义工具

```rust
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct WeatherArgs {
    location: String,
}

#[derive(Debug)]
struct WeatherTool;

impl Tool for WeatherTool {
    const NAME: &'static str = "get_current_weather";

    type Error = String;
    type Args = WeatherArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "查询指定城市的天气".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "城市名称"
                    }
                },
                "required": ["location"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 实现天气查询逻辑
        Ok(format!("{}的天气是晴天，温度22度", args.location))
    }
}
```

### 使用工具

```rust
use rig::agent::AgentBuilder;

// 创建带工具的代理
let agent = client
    .agent(qwen::QWEN_PLUS)
    .preamble("你是一个有用的助手，可以查询天气信息。")
    .tool(WeatherTool)
    .build();

// 使用代理
let response = agent.prompt("杭州今天天气怎么样？").await?;
println!("{}", response);
```

---

## 高级功能

### 1. 思考模式（QwQ 模型）

QwQ 模型支持深度推理，会输出思考过程。

```rust
// 使用 QwQ 模型
let qwq_model = client.completion_model(qwen::QWQ_PLUS);

let request = CompletionRequest::new("解决这个数学问题：2x + 5 = 15");

let response = qwq_model.completion(request).await?;

// 响应中会包含推理内容
for content in response.choice.iter() {
    match content {
        AssistantContent::Reasoning(reasoning) => {
            println!("思考过程：{:?}", reasoning.reasoning);
        }
        AssistantContent::Text(text) => {
            println!("最终答案：{}", text.text);
        }
        _ => {}
    }
}
```

### 2. 自定义参数

通过 `additional_params` 传递通义千问特定的参数。

```rust
let request = CompletionRequest::new("你好")
    .temperature(0.7)
    .additional_param("top_k", json!(20))
    .additional_param("repetition_penalty", json!(1.05))
    .additional_param("enable_thinking", json!(true))  // 启用思考模式
    .build();

let response = model.completion(request).await?;
```

### 3. 结构化输出

强制模型输出 JSON 格式。

```rust
let request = CompletionRequest::new("列出三个水果的名称和颜色")
    .preamble("请按照 JSON 格式输出")
    .additional_param("response_format", json!({"type": "json_object"}))
    .build();

let response = model.completion(request).await?;
```

### 4. 联网搜索

启用互联网搜索功能。

```rust
let request = CompletionRequest::new("明天杭州天气如何？")
    .additional_param("enable_search", json!(true))
    .additional_param("search_options", json!({
        "enable_source": true,
        "enable_citation": true,
        "search_strategy": "turbo"
    }))
    .build();

let response = model.completion(request).await?;
```

---

## 支持的模型

### 商业版模型

| 模型常量 | 模型名称 | 说明 |
|---------|---------|------|
| `QWEN_PLUS` | qwen-plus | 平衡性能，适合日常对话 |
| `QWEN_PLUS_LATEST` | qwen-plus-latest | 最新版 Plus 模型 |
| `QWEN_MAX` | qwen-max | 最强性能，适合复杂推理 |
| `QWEN_MAX_LATEST` | qwen-max-latest | 最新版 Max 模型 |
| `QWEN_TURBO` | qwen-turbo | 快速响应，经济实惠 |
| `QWEN_TURBO_LATEST` | qwen-turbo-latest | 最新版 Turbo 模型 |
| `QWEN_FLASH` | qwen-flash | 极速响应 |
| `QWEN3_MAX` | qwen3-max | Qwen3 最强版本 |
| `QWQ_PLUS` | qwq-plus | 深度推理模型 |

### 使用示例

```rust
// 日常对话 - 使用 Plus
let plus_model = client.completion_model(qwen::QWEN_PLUS);

// 复杂推理 - 使用 Max
let max_model = client.completion_model(qwen::QWEN_MAX);

// 快速响应 - 使用 Turbo
let turbo_model = client.completion_model(qwen::QWEN_TURBO);

// 深度推理 - 使用 QwQ
let qwq_model = client.completion_model(qwen::QWQ_PLUS);
```

---

## 配置选项

### 温度（Temperature）

控制生成文本的多样性。

```rust
let request = CompletionRequest::new("写一首诗")
    .temperature(0.9)  // 0.0-2.0，越高越有创意
    .build();
```

### 最大令牌数（Max Tokens）

限制输出长度。

```rust
let request = CompletionRequest::new("介绍一下北京")
    .additional_param("max_tokens", json!(100))
    .build();
```

### 停止词（Stop）

指定停止生成的词语。

```rust
let request = CompletionRequest::new("数到10")
    .additional_param("stop", json!(["5", "\n\n"]))
    .build();
```

### 系统提示（Preamble）

设置模型的角色和行为。

```rust
let request = CompletionRequest::new("你好")
    .preamble("你是一个专业的技术顾问，擅长解答编程问题。")
    .build();
```

---

## 示例代码

### 示例 1：基本对话

```rust
use rig::{completion::Prompt, providers::qwen};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = qwen::Client::from_env();
    let model = client.completion_model(qwen::QWEN_PLUS);

    let response = model.prompt("你是谁？").await?;
    println!("{}", response);

    Ok(())
}
```

**运行**：
```bash
DASHSCOPE_API_KEY=your_key cargo run --example qwen_basic
```

### 示例 2：流式输出

```rust
use futures::StreamExt;
use rig::{completion::CompletionRequest, providers::qwen};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = qwen::Client::from_env();
    let model = client.completion_model(qwen::QWEN_PLUS);

    let request = CompletionRequest::new("讲一个故事");
    let mut stream = model.stream(request).await?;

    while let Some(chunk) = stream.next().await {
        if let Ok(content) = chunk {
            print!("{}", content);
            std::io::stdout().flush()?;
        }
    }

    Ok(())
}
```

**运行**：
```bash
DASHSCOPE_API_KEY=your_key cargo run --example qwen_streaming
```

### 示例 3：工具调用

```rust
use rig::{agent::AgentBuilder, providers::qwen, tool::Tool};

// ... 定义工具 ...

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = qwen::Client::from_env();

    let agent = client
        .agent(qwen::QWEN_PLUS)
        .tool(WeatherTool)
        .build();

    let response = agent.prompt("杭州天气怎么样？").await?;
    println!("{}", response);

    Ok(())
}
```

**运行**：
```bash
DASHSCOPE_API_KEY=your_key cargo run --example qwen_tools
```

---

## 通义千问特有功能

### 1. 思考模式

Qwen3 和 QwQ 模型支持思考模式，会输出推理过程。

```rust
let request = CompletionRequest::new("解决数学问题")
    .additional_param("enable_thinking", json!(true))
    .additional_param("thinking_budget", json!(1000))
    .build();

let response = model.completion(request).await?;

// 处理推理内容
for content in response.choice.iter() {
    match content {
        AssistantContent::Reasoning(reasoning) => {
            println!("思考过程：");
            for step in &reasoning.reasoning {
                println!("  - {}", step);
            }
        }
        AssistantContent::Text(text) => {
            println!("最终答案：{}", text.text);
        }
        _ => {}
    }
}
```

### 2. 联网搜索

启用互联网搜索获取实时信息。

```rust
let request = CompletionRequest::new("明天杭州天气如何？")
    .additional_param("enable_search", json!(true))
    .additional_param("search_options", json!({
        "enable_source": true,
        "enable_citation": true,
        "citation_format": "[<number>]",
        "search_strategy": "turbo"
    }))
    .build();
```

### 3. 结构化输出

强制模型输出 JSON 格式。

```rust
let request = CompletionRequest::new("列出三个水果")
    .preamble("请按照 JSON 格式输出")
    .additional_param("response_format", json!({"type": "json_object"}))
    .build();
```

### 4. 增量输出控制

在流式模式下控制输出方式。

```rust
let request = CompletionRequest::new("写一篇文章")
    .additional_param("incremental_output", json!(true))  // 增量输出（推荐）
    .build();

let mut stream = model.stream(request).await?;
```

---

## API 端点说明

通义千问使用不同的端点：

- **文本生成**：`/text-generation/generation`
- **多模态生成**：`/multimodal-generation/generation`（用于图像、视频、音频输入）

当前 Rig 实现主要支持文本生成端点。

---

## 错误处理

### 常见错误

```rust
use rig::completion::CompletionError;

match model.prompt("你好").await {
    Ok(response) => println!("{}", response),
    Err(CompletionError::ProviderError(msg)) => {
        eprintln!("API 错误：{}", msg);
    }
    Err(CompletionError::RequestError(msg)) => {
        eprintln!("请求错误：{}", msg);
    }
    Err(e) => {
        eprintln!("其他错误：{}", e);
    }
}
```

### 重试逻辑

```rust
use std::time::Duration;

async fn completion_with_retry(
    model: &CompletionModel,
    request: CompletionRequest,
    max_retries: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut retries = 0;

    loop {
        match model.completion(request.clone()).await {
            Ok(response) => return Ok(response.choice.to_string()),
            Err(e) if retries < max_retries => {
                eprintln!("请求失败，重试中... ({}/{})", retries + 1, max_retries);
                tokio::time::sleep(Duration::from_secs(2_u64.pow(retries))).await;
                retries += 1;
            }
            Err(e) => return Err(e.into()),
        }
    }
}
```

---

## 性能优化

### 1. 客户端重用

```rust
// ✅ 好的做法：重用客户端
let client = qwen::Client::from_env();
let model1 = client.completion_model(qwen::QWEN_PLUS);
let model2 = client.completion_model(qwen::QWEN_MAX);

// ❌ 不好的做法：每次创建新客户端
let model1 = qwen::Client::from_env().completion_model(qwen::QWEN_PLUS);
let model2 = qwen::Client::from_env().completion_model(qwen::QWEN_MAX);
```

### 2. 使用流式输出

对于长文本生成，使用流式输出可以提升用户体验。

```rust
// 长文本生成 - 使用流式
let mut stream = model.stream(request).await?;
```

### 3. 控制输出长度

合理设置 max_tokens 以控制成本。

```rust
let request = CompletionRequest::new("简单介绍一下")
    .additional_param("max_tokens", json!(100))
    .build();
```

---

## 与其他供应商的对比

### 通义千问 vs OpenAI

| 特性 | 通义千问 | OpenAI |
|------|---------|--------|
| API 结构 | 嵌套结构（input/parameters） | 扁平结构 |
| 流式标识 | Header: X-DashScope-SSE | 参数: stream |
| 思考模式 | enable_thinking | 不支持 |
| 联网搜索 | enable_search | 不支持 |
| 增量输出 | incremental_output | 默认增量 |

### 通义千问 vs DeepSeek

| 特性 | 通义千问 | DeepSeek |
|------|---------|----------|
| 推理内容 | reasoning_content | reasoning_content |
| 工具调用 | 支持 | 支持 |
| 流式处理 | SSE | SSE |
| 特殊功能 | 联网搜索、OCR | 无 |

---

## 故障排查

### 问题 1：认证失败

**错误**：`DASHSCOPE_API_KEY not set`

**解决**：
```bash
export DASHSCOPE_API_KEY=your_api_key
```

### 问题 2：流式输出没有响应

**原因**：某些模型（如 QwQ）只支持流式输出

**解决**：使用 `stream()` 方法而不是 `completion()`

```rust
// QwQ 模型必须使用流式
let qwq_model = client.completion_model(qwen::QWQ_PLUS);
let mut stream = qwq_model.stream(request).await?;
```

### 问题 3：工具调用失败

**原因**：需要设置 `result_format` 为 `"message"`

**解决**：这在 Rig 实现中已自动设置，无需手动配置

---

## 完整示例

### 综合示例：智能助手

```rust
use rig::{
    agent::AgentBuilder,
    completion::{Chat, Prompt},
    providers::qwen,
    tool::Tool,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

// 定义计算器工具
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct CalculatorArgs {
    expression: String,
}

#[derive(Debug)]
struct CalculatorTool;

impl Tool for CalculatorTool {
    const NAME: &'static str = "calculate";
    type Error = String;
    type Args = CalculatorArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "计算数学表达式".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "数学表达式，如 '2+2' 或 '10*5'"
                    }
                },
                "required": ["expression"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 简单的计算实现
        Ok(format!("计算结果：{}", args.expression))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let client = qwen::Client::from_env();

    // 创建智能助手
    let assistant = client
        .agent(qwen::QWEN_PLUS)
        .preamble("你是一个智能助手，可以帮助用户进行计算。")
        .tool(CalculatorTool)
        .build();

    println!("=== 智能助手示例 ===\n");

    // 简单对话
    let response1 = assistant.prompt("你好！").await?;
    println!("助手：{}\n", response1);

    // 使用工具
    let response2 = assistant.prompt("帮我计算 25 * 4").await?;
    println!("助手：{}\n", response2);

    // 多轮对话
    let response3 = assistant.chat(
        "那 100 除以 4 呢？",
        vec![
            Message::user("帮我计算 25 * 4"),
            Message::assistant(&response2),
        ]
    ).await?;
    println!("助手：{}\n", response3);

    Ok(())
}
```

---

## 注意事项

### 1. API 密钥安全

- ❌ 不要在代码中硬编码 API 密钥
- ✅ 使用环境变量或配置文件
- ✅ 不要提交包含 API 密钥的文件到版本控制

### 2. 速率限制

- 通义千问有速率限制，请合理控制请求频率
- 实现重试逻辑处理 429 错误
- 考虑使用批处理减少请求次数

### 3. Token 消耗

- 使用 `max_tokens` 控制输出长度
- 使用 `stop` 参数在合适时停止生成
- 监控 `usage` 字段了解 Token 消耗

### 4. 模型选择

- **Plus**：日常对话和通用任务
- **Max**：复杂推理和专业领域
- **Turbo**：简单任务和成本敏感场景
- **Flash**：极速响应需求
- **QwQ**：需要深度推理的任务

---

## 参考资源

- [通义千问官方文档](https://help.aliyun.com/zh/model-studio/)
- [API 详细文档](qwen_api.md)
- [Rig 框架文档](https://github.com/0xPlaygrounds/rig)
- [示例代码](rig-core/examples/)

---

## 更新日志

### v1.0.0 (2025-01-09)

- ✅ 实现基本完成功能
- ✅ 实现流式输出
- ✅ 支持工具调用
- ✅ 支持思考模式（QwQ）
- ✅ 支持推理内容输出
- ✅ 完整的错误处理
- ✅ OpenTelemetry 追踪集成

### 未来计划

- [ ] 支持多模态输入（图像、视频、音频）
- [ ] 支持 OCR 功能
- [ ] 支持语音识别
- [ ] 支持深入研究模型
- [ ] 支持上下文缓存

---

**版本**: 1.0.0  
**最后更新**: 2025-01-09  
**维护者**: Rig Community

