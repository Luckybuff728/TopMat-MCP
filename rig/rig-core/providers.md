# Rig 框架 - 添加新的模型供应商指南

本文档详细说明了如何在 Rig 框架中添加新的 AI 模型供应商（Provider）。

## 目录

1. [概述](#概述)
2. [架构理解](#架构理解)
3. [核心 Trait 系统](#核心-trait-系统)
4. [实现步骤](#实现步骤)
5. [文件结构](#文件结构)
6. [代码示例](#代码示例)
7. [测试与验证](#测试与验证)
8. [最佳实践](#最佳实践)

---

## 概述

### 什么是供应商（Provider）？

在 Rig 框架中，供应商是指提供 AI 模型服务的第三方平台，如：
- OpenAI
- Anthropic
- DeepSeek
- Cohere
- Google Gemini
- 等等

### 供应商的核心功能

每个供应商需要实现以下功能（根据 API 支持情况）：
1. **完成（Completion）** - 文本生成和聊天
2. **嵌入（Embeddings）** - 文本向量化
3. **转录（Transcription）** - 语音转文字
4. **图像生成（Image Generation）** - 文本生成图像
5. **音频生成（Audio Generation）** - 文本转语音
6. **流式处理（Streaming）** - 实时流式响应

---

## 架构理解

### Rig 框架的提供商架构

```
rig-core/src/providers/
├── mod.rs                      # 提供商模块导出
├── your_provider/              # 您的供应商目录（如果功能复杂）
│   ├── mod.rs                  # 模块定义和导出
│   ├── client.rs               # 客户端实现
│   ├── completion.rs           # 完成功能实现
│   ├── streaming.rs            # 流式处理实现
│   ├── embeddings.rs           # 嵌入功能实现（可选）
│   └── transcription.rs        # 转录功能实现（可选）
└── your_provider.rs            # 或单文件实现（如果功能简单）
```

### 核心组件

1. **Client** - 提供商客户端
   - 管理 API 密钥和 HTTP 客户端
   - 提供认证和请求构建方法
   - 实现 `ProviderClient` trait

2. **CompletionModel** - 完成模型
   - 实现 `CompletionModel` trait
   - 处理完成请求和响应
   - 支持流式和非流式处理

3. **EmbeddingModel** - 嵌入模型（可选）
   - 实现 `EmbeddingModel` trait
   - 处理文本向量化

4. **其他模型** - 转录、图像生成等（可选）

---

## 核心 Trait 系统

### 1. ProviderClient Trait

所有供应商客户端必须实现的基础 trait：

```rust
pub trait ProviderClient:
    AsCompletion + 
    AsTranscription + 
    AsEmbeddings + 
    AsImageGeneration + 
    AsAudioGeneration + 
    Debug
{
    /// 从环境变量创建客户端
    fn from_env() -> Self where Self: Sized;
    
    /// 从 ProviderValue 创建客户端
    fn from_val(input: ProviderValue) -> Self where Self: Sized;
}
```

### 2. CompletionClient Trait

提供完成功能的客户端需要实现：

```rust
pub trait CompletionClient {
    type CompletionModel;
    
    /// 创建完成模型实例
    fn completion_model(&self, model_name: &str) -> Self::CompletionModel;
}
```

### 3. CompletionModel Trait

完成模型需要实现：

```rust
pub trait CompletionModel {
    type Response;
    type StreamingResponse;
    
    /// 非流式完成请求
    async fn completion(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError>;
    
    /// 流式完成请求
    async fn stream(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>;
}
```

### 4. VerifyClient Trait

客户端验证功能：

```rust
pub trait VerifyClient {
    /// 验证客户端连接和 API 密钥
    async fn verify(&self) -> Result<(), VerifyError>;
}
```

### 5. 转换 Trait

使用宏自动实现转换功能：

```rust
impl_conversion_traits!(
    AsEmbeddings,
    AsTranscription,
    AsImageGeneration,
    AsAudioGeneration for Client
);
```

---

## 实现步骤

### 步骤 1: 规划供应商结构

确定您的供应商需要支持的功能：

- [ ] 完成（Completion）- **必需**
- [ ] 流式处理（Streaming）- **强烈推荐**
- [ ] 嵌入（Embeddings）- 可选
- [ ] 转录（Transcription）- 可选
- [ ] 图像生成（Image Generation）- 可选
- [ ] 音频生成（Audio Generation）- 可选

### 步骤 2: 创建文件结构

#### 选项 A: 单文件实现（简单供应商）

适用于只支持完成功能的简单供应商（如 Perplexity、Groq）：

```bash
rig-core/src/providers/your_provider.rs
```

#### 选项 B: 多文件实现（复杂供应商）

适用于支持多种功能的复杂供应商（如 OpenAI、Cohere）：

```bash
rig-core/src/providers/your_provider/
├── mod.rs          # 模块定义
├── client.rs       # 客户端实现
├── completion.rs   # 完成功能
├── streaming.rs    # 流式处理
└── embeddings.rs   # 嵌入功能（可选）
```

### 步骤 3: 实现客户端（Client）

#### 3.1 定义常量

```rust
// API 基础 URL
const YOUR_PROVIDER_API_BASE_URL: &str = "https://api.yourprovider.com/v1";

// 模型常量
pub const MODEL_NAME_1: &str = "model-name-1";
pub const MODEL_NAME_2: &str = "model-name-2";
```

#### 3.2 实现 ClientBuilder

```rust
pub struct ClientBuilder<'a> {
    api_key: &'a str,
    base_url: &'a str,
    http_client: Option<reqwest::Client>,
}

impl<'a> ClientBuilder<'a> {
    pub fn new(api_key: &'a str) -> Self {
        Self {
            api_key,
            base_url: YOUR_PROVIDER_API_BASE_URL,
            http_client: None,
        }
    }

    pub fn base_url(mut self, base_url: &'a str) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn custom_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn build(self) -> Result<Client, ClientBuilderError> {
        let http_client = if let Some(http_client) = self.http_client {
            http_client
        } else {
            reqwest::Client::builder().build()?
        };

        Ok(Client {
            base_url: self.base_url.to_string(),
            api_key: self.api_key.to_string(),
            http_client,
        })
    }
}
```

#### 3.3 实现 Client 结构体

```rust
#[derive(Clone)]
pub struct Client {
    base_url: String,
    api_key: String,
    http_client: reqwest::Client,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("http_client", &self.http_client)
            .field("api_key", &"<REDACTED>")  // 永远不要输出实际的 API 密钥
            .finish()
    }
}

impl Client {
    /// 创建新的客户端构建器
    pub fn builder(api_key: &str) -> ClientBuilder<'_> {
        ClientBuilder::new(api_key)
    }

    /// 创建新的客户端（简化方法）
    pub fn new(api_key: &str) -> Self {
        Self::builder(api_key)
            .build()
            .expect("Client should build")
    }

    /// POST 请求辅助方法
    pub(crate) fn post(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/{}", self.base_url, path).replace("//", "/");
        self.http_client.post(url).bearer_auth(&self.api_key)
    }

    /// GET 请求辅助方法（如果需要）
    pub(crate) fn get(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/{}", self.base_url, path).replace("//", "/");
        self.http_client.get(url).bearer_auth(&self.api_key)
    }
}
```

#### 3.4 实现 ProviderClient Trait

```rust
impl ProviderClient for Client {
    fn from_env() -> Self {
        let api_key = std::env::var("YOUR_PROVIDER_API_KEY")
            .expect("YOUR_PROVIDER_API_KEY not set");
        Self::new(&api_key)
    }

    fn from_val(input: crate::client::ProviderValue) -> Self {
        let crate::client::ProviderValue::Simple(api_key) = input else {
            panic!("Incorrect provider value type")
        };
        Self::new(&api_key)
    }
}
```

#### 3.5 实现 CompletionClient Trait

```rust
impl CompletionClient for Client {
    type CompletionModel = CompletionModel;

    fn completion_model(&self, model_name: &str) -> CompletionModel {
        CompletionModel {
            client: self.clone(),
            model: model_name.to_string(),
        }
    }
}
```

#### 3.6 实现 VerifyClient Trait

```rust
impl VerifyClient for Client {
    #[cfg_attr(feature = "worker", worker::send)]
    async fn verify(&self) -> Result<(), VerifyError> {
        // 如果 API 提供验证端点
        let response = self.get("/verify").send().await?;
        
        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            reqwest::StatusCode::UNAUTHORIZED => Err(VerifyError::InvalidAuthentication),
            _ => Err(VerifyError::ProviderError(response.text().await?)),
        }
        
        // 如果没有验证端点，直接返回 Ok
        // Ok(())
    }
}
```

#### 3.7 实现转换 Trait

```rust
// 根据您支持的功能选择需要的转换
impl_conversion_traits!(
    AsEmbeddings,          // 如果支持嵌入
    AsTranscription,       // 如果支持转录
    AsImageGeneration,     // 如果支持图像生成
    AsAudioGeneration      // 如果支持音频生成
    for Client
);
```

### 步骤 4: 定义数据结构

#### 4.1 API 响应结构

```rust
// 完成响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

// 选择结构
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    pub index: usize,
    pub message: Message,
    pub finish_reason: String,
}

// 使用情况统计
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Usage {
    pub completion_tokens: u32,
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

// 消息结构
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    User {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Assistant {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(
            default,
            deserialize_with = "json_utils::null_or_vec",
            skip_serializing_if = "Vec::is_empty"
        )]
        tool_calls: Vec<ToolCall>,
    },
    #[serde(rename = "tool")]
    ToolResult {
        tool_call_id: String,
        content: String,
    },
}
```

#### 4.2 工具调用结构（如果支持）

```rust
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ToolCall {
    pub id: String,
    pub index: usize,
    #[serde(default)]
    pub r#type: ToolType,
    pub function: Function,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Function {
    pub name: String,
    #[serde(with = "json_utils::stringified_json")]
    pub arguments: serde_json::Value,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    #[default]
    Function,
}
```

#### 4.3 错误响应结构

```rust
#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ApiResponse<T> {
    Ok(T),
    Err(ApiErrorResponse),
}

impl From<ApiErrorResponse> for CompletionError {
    fn from(err: ApiErrorResponse) -> Self {
        CompletionError::ProviderError(err.message)
    }
}
```

### 步骤 5: 实现 CompletionModel

#### 5.1 定义 CompletionModel 结构

```rust
#[derive(Clone)]
pub struct CompletionModel {
    pub client: Client,
    pub model: String,
}
```

#### 5.2 实现类型转换

```rust
// 从 Rig 的 Message 转换到供应商的 Message
impl TryFrom<message::Message> for Vec<Message> {
    type Error = MessageError;

    fn try_from(message: message::Message) -> Result<Self, Self::Error> {
        match message {
            message::Message::User { content } => {
                // 处理用户消息
                let messages = content
                    .into_iter()
                    .filter_map(|content| match content {
                        message::UserContent::Text(text) => Some(Message::User {
                            content: text.text,
                            name: None,
                        }),
                        _ => None,
                    })
                    .collect();
                Ok(messages)
            }
            message::Message::Assistant { content, .. } => {
                // 处理助手消息
                let mut messages = vec![];
                let mut text_content = String::new();
                let mut tool_calls = vec![];

                for item in content {
                    match item {
                        completion::AssistantContent::Text(text) => {
                            text_content.push_str(&text.text);
                        }
                        completion::AssistantContent::ToolCall(call) => {
                            tool_calls.push(ToolCall::from(call));
                        }
                        _ => {}
                    }
                }

                messages.push(Message::Assistant {
                    content: text_content,
                    name: None,
                    tool_calls,
                });

                Ok(messages)
            }
        }
    }
}

// 从供应商的 CompletionResponse 转换到 Rig 的 CompletionResponse
impl TryFrom<CompletionResponse> for completion::CompletionResponse<CompletionResponse> {
    type Error = CompletionError;

    fn try_from(response: CompletionResponse) -> Result<Self, Self::Error> {
        let choice = response.choices.first().ok_or_else(|| {
            CompletionError::ResponseError("Response contained no choices".to_owned())
        })?;

        let content = match &choice.message {
            Message::Assistant { content, tool_calls, .. } => {
                let mut result = vec![];
                
                if !content.is_empty() {
                    result.push(completion::AssistantContent::text(content));
                }
                
                for call in tool_calls {
                    result.push(completion::AssistantContent::tool_call(
                        &call.id,
                        &call.function.name,
                        call.function.arguments.clone(),
                    ));
                }
                
                Ok(result)
            }
            _ => Err(CompletionError::ResponseError(
                "Response did not contain assistant message".to_owned(),
            )),
        }?;

        let usage = completion::Usage {
            input_tokens: response.usage.prompt_tokens as u64,
            output_tokens: response.usage.completion_tokens as u64,
            total_tokens: response.usage.total_tokens as u64,
        };

        Ok(completion::CompletionResponse {
            choice,
            usage,
            raw_response: response,
        })
    }
}
```

#### 5.3 实现请求构建

```rust
impl CompletionModel {
    fn create_completion_request(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<serde_json::Value, CompletionError> {
        // 构建消息历史
        let mut full_history: Vec<Message> = vec![];

        // 添加系统提示（如果有）
        if let Some(preamble) = &completion_request.preamble {
            full_history.push(Message::System {
                content: preamble.clone(),
                name: None,
            });
        }

        // 添加文档（如果有）
        if let Some(docs) = completion_request.normalized_documents() {
            full_history.extend(
                Vec::<Message>::try_from(docs)?
            );
        }

        // 添加聊天历史
        for msg in completion_request.chat_history {
            full_history.extend(
                Vec::<Message>::try_from(msg)?
            );
        }

        // 构建请求 JSON
        let mut request = json!({
            "model": self.model,
            "messages": full_history,
            "temperature": completion_request.temperature,
        });

        // 添加工具（如果有）
        if !completion_request.tools.is_empty() {
            request["tools"] = json!(
                completion_request.tools
                    .into_iter()
                    .map(ToolDefinition::from)
                    .collect::<Vec<_>>()
            );
        }

        // 合并额外参数
        if let Some(params) = completion_request.additional_params {
            request = json_utils::merge(request, params);
        }

        Ok(request)
    }
}
```

#### 5.4 实现 CompletionModel Trait

```rust
impl completion::CompletionModel for CompletionModel {
    type Response = CompletionResponse;
    type StreamingResponse = StreamingCompletionResponse;

    #[cfg_attr(feature = "worker", worker::send)]
    async fn completion(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<completion::CompletionResponse<CompletionResponse>, CompletionError> {
        let request = self.create_completion_request(completion_request)?;

        // 创建追踪 span
        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat",
                gen_ai.operation.name = "chat",
                gen_ai.provider.name = "your_provider",
                gen_ai.request.model = self.model,
            )
        } else {
            tracing::Span::current()
        };

        async move {
            // 发送请求
            let response = self
                .client
                .post("/chat/completions")
                .json(&request)
                .send()
                .await?;

            // 检查响应状态
            if response.status().is_success() {
                let text = response.text().await?;
                
                // 解析响应
                match serde_json::from_str::<ApiResponse<CompletionResponse>>(&text)? {
                    ApiResponse::Ok(response) => response.try_into(),
                    ApiResponse::Err(err) => Err(CompletionError::ProviderError(err.message)),
                }
            } else {
                Err(CompletionError::ProviderError(response.text().await?))
            }
        }
        .instrument(span)
        .await
    }

    #[cfg_attr(feature = "worker", worker::send)]
    async fn stream(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<
        crate::streaming::StreamingCompletionResponse<Self::StreamingResponse>,
        CompletionError,
    > {
        let mut request = self.create_completion_request(completion_request)?;
        
        // 启用流式传输
        request = merge(
            request,
            json!({"stream": true}),
        );

        let builder = self.client.post("/chat/completions").json(&request);
        
        // 发送流式请求
        send_streaming_request(builder).await
    }
}
```

### 步骤 6: 实现流式处理

#### 6.1 定义流式数据结构

```rust
#[derive(Deserialize, Debug)]
pub struct StreamingDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default, deserialize_with = "json_utils::null_or_vec")]
    tool_calls: Vec<StreamingToolCall>,
}

#[derive(Deserialize, Debug)]
struct StreamingChoice {
    delta: StreamingDelta,
}

#[derive(Deserialize, Debug)]
struct StreamingCompletionChunk {
    choices: Vec<StreamingChoice>,
    usage: Option<Usage>,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct StreamingCompletionResponse {
    pub usage: Usage,
}

impl GetTokenUsage for StreamingCompletionResponse {
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        let mut usage = crate::completion::Usage::new();
        usage.input_tokens = self.usage.prompt_tokens as u64;
        usage.output_tokens = self.usage.completion_tokens as u64;
        usage.total_tokens = self.usage.total_tokens as u64;
        Some(usage)
    }
}
```

#### 6.2 实现 SSE 流式处理

```rust
pub async fn send_streaming_request(
    request_builder: reqwest::RequestBuilder,
) -> Result<
    crate::streaming::StreamingCompletionResponse<StreamingCompletionResponse>,
    CompletionError,
> {
    let mut event_source = request_builder
        .eventsource()
        .expect("Cloning request must succeed");

    let stream = Box::pin(stream! {
        let mut final_usage = Usage::new();
        let mut text_response = String::new();

        while let Some(event_result) = event_source.next().await {
            match event_result {
                Ok(Event::Open) => {
                    tracing::trace!("SSE connection opened");
                    continue;
                }
                Ok(Event::Message(message)) => {
                    // 跳过空消息或 [DONE] 标记
                    if message.data.trim().is_empty() || message.data == "[DONE]" {
                        continue;
                    }

                    // 解析流式块
                    let parsed = serde_json::from_str::<StreamingCompletionChunk>(&message.data);
                    let Ok(data) = parsed else {
                        tracing::debug!("Couldn't parse SSE payload: {:?}", parsed.unwrap_err());
                        continue;
                    };

                    // 处理选择
                    if let Some(choice) = data.choices.first() {
                        let delta = &choice.delta;

                        // 处理文本内容
                        if let Some(content) = &delta.content {
                            text_response += content;
                            yield Ok(crate::streaming::RawStreamingChoice::Message(content.clone()));
                        }

                        // 处理工具调用
                        for tool_call in &delta.tool_calls {
                            if let (Some(id), Some(name)) = (&tool_call.id, &tool_call.function.name) {
                                if let Ok(args) = serde_json::from_str(&tool_call.function.arguments) {
                                    yield Ok(crate::streaming::RawStreamingChoice::ToolCall {
                                        id: id.clone(),
                                        name: name.clone(),
                                        arguments: args,
                                        call_id: None,
                                    });
                                }
                            }
                        }
                    }

                    // 更新使用情况
                    if let Some(usage) = data.usage {
                        final_usage = usage;
                    }
                }
                Err(reqwest_eventsource::Error::StreamEnded) => {
                    break;
                }
                Err(err) => {
                    tracing::error!(?err, "SSE error");
                    yield Err(CompletionError::ResponseError(err.to_string()));
                    break;
                }
            }
        }

        // 生成最终响应
        yield Ok(crate::streaming::RawStreamingChoice::FinalResponse(
            StreamingCompletionResponse { usage: final_usage }
        ));
    });

    Ok(crate::streaming::StreamingCompletionResponse::stream(stream))
}
```

### 步骤 7: 注册供应商

#### 7.1 在 `mod.rs` 中添加模块

编辑 `rig-core/src/providers/mod.rs`：

```rust
// 添加您的供应商模块
pub mod your_provider;
```

#### 7.2 更新文档

在 `mod.rs` 的文档注释中添加您的供应商：

```rust
//! 目前支持以下提供商：
//! - Cohere
//! - OpenAI
//! - Your Provider  // 添加这行
//! ...
```

### 步骤 8: 实现其他功能（可选）

#### 8.1 嵌入（Embeddings）

如果您的供应商支持嵌入功能：

```rust
#[derive(Clone)]
pub struct EmbeddingModel {
    client: Client,
    model: String,
    ndims: usize,
}

impl embeddings::EmbeddingModel for EmbeddingModel {
    const MAX_DOCUMENTS: usize = 96;

    fn ndims(&self) -> usize {
        self.ndims
    }

    #[cfg_attr(feature = "worker", worker::send)]
    async fn embed_texts(
        &self,
        documents: impl IntoIterator<Item = String>,
    ) -> Result<Vec<embeddings::Embedding>, EmbeddingError> {
        let documents = documents.into_iter().collect::<Vec<_>>();

        let response = self
            .client
            .post("/embeddings")
            .json(&json!({
                "model": self.model,
                "input": documents,
            }))
            .send()
            .await?;

        // 解析和返回嵌入向量
        // ...
    }
}

// 实现 EmbeddingsClient trait
impl EmbeddingsClient for Client {
    type EmbeddingModel = EmbeddingModel;

    fn embedding_model(&self, model: &str, ndims: usize) -> EmbeddingModel {
        EmbeddingModel {
            client: self.clone(),
            model: model.to_string(),
            ndims,
        }
    }
}
```

#### 8.2 转录（Transcription）

如果您的供应商支持转录功能：

```rust
#[derive(Clone)]
pub struct TranscriptionModel {
    client: Client,
    model: String,
}

impl transcription::TranscriptionModel for TranscriptionModel {
    type Response = TranscriptionResponse;

    #[cfg_attr(feature = "worker", worker::send)]
    async fn transcription(
        &self,
        request: transcription::TranscriptionRequest,
    ) -> Result<
        transcription::TranscriptionResponse<Self::Response>,
        transcription::TranscriptionError,
    > {
        let data = request.data;

        let body = reqwest::multipart::Form::new()
            .text("model", self.model.clone())
            .part("file", Part::bytes(data).file_name(request.filename));

        let response = self
            .client
            .post("/audio/transcriptions")
            .multipart(body)
            .send()
            .await?;

        // 解析和返回转录结果
        // ...
    }
}
```

---

## 文件结构

### 完整的文件结构示例

#### 单文件实现（your_provider.rs）

```rust
//! Your Provider API 客户端和 Rig 集成
//!
//! # 示例
//! ```
//! use rig::providers::your_provider;
//!
//! let client = your_provider::Client::new("YOUR_API_KEY");
//! let model = client.completion_model(your_provider::MODEL_NAME);
//! ```

// 导入
use crate::{/* ... */};

// ================================================================
// 常量定义
// ================================================================
const YOUR_PROVIDER_API_BASE_URL: &str = "https://api.yourprovider.com/v1";

pub const MODEL_NAME_1: &str = "model-1";
pub const MODEL_NAME_2: &str = "model-2";

// ================================================================
// 客户端实现
// ================================================================
pub struct ClientBuilder<'a> { /* ... */ }
impl<'a> ClientBuilder<'a> { /* ... */ }

#[derive(Clone)]
pub struct Client { /* ... */ }
impl Client { /* ... */ }
impl ProviderClient for Client { /* ... */ }
impl CompletionClient for Client { /* ... */ }
impl VerifyClient for Client { /* ... */ }

// ================================================================
// 数据结构
// ================================================================
pub struct CompletionResponse { /* ... */ }
pub struct Usage { /* ... */ }
pub struct Message { /* ... */ }
pub struct ToolCall { /* ... */ }

// ================================================================
// CompletionModel 实现
// ================================================================
#[derive(Clone)]
pub struct CompletionModel { /* ... */ }
impl CompletionModel { /* ... */ }
impl completion::CompletionModel for CompletionModel { /* ... */ }

// ================================================================
// 流式处理
// ================================================================
pub struct StreamingCompletionResponse { /* ... */ }
pub async fn send_streaming_request(/* ... */) { /* ... */ }

// ================================================================
// 测试
// ================================================================
#[cfg(test)]
mod tests { /* ... */ }
```

#### 多文件实现（your_provider/）

**mod.rs:**
```rust
//! Your Provider API 客户端和 Rig 集成

pub mod client;
pub mod completion;
pub mod streaming;

pub use client::Client;
pub use completion::CompletionModel;

// 模型常量
pub const MODEL_NAME_1: &str = "model-1";
pub const MODEL_NAME_2: &str = "model-2";
```

**client.rs:**
```rust
//! Your Provider 客户端实现

use crate::client::*;

pub struct ClientBuilder<'a> { /* ... */ }
impl<'a> ClientBuilder<'a> { /* ... */ }

#[derive(Clone)]
pub struct Client { /* ... */ }
impl Client { /* ... */ }
impl ProviderClient for Client { /* ... */ }
impl CompletionClient for Client { /* ... */ }
```

**completion.rs:**
```rust
//! Your Provider 完成功能实现

use super::client::Client;
use crate::completion::*;

#[derive(Clone)]
pub struct CompletionModel { /* ... */ }
impl CompletionModel { /* ... */ }
impl completion::CompletionModel for CompletionModel { /* ... */ }
```

**streaming.rs:**
```rust
//! Your Provider 流式处理实现

use crate::streaming::*;

pub struct StreamingCompletionResponse { /* ... */ }
pub async fn send_streaming_request(/* ... */) { /* ... */ }
```

---

## 代码示例

### 完整的最小供应商实现

以下是一个支持基本完成功能的最小供应商实现：

```rust
//! Minimal Provider API 客户端和 Rig 集成

use crate::{
    OneOrMany,
    client::{ClientBuilderError, CompletionClient, ProviderClient, VerifyClient, VerifyError},
    completion::{self, CompletionError, CompletionRequest, message},
    impl_conversion_traits, json_utils,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{Instrument, info_span};

// ================================================================
// 主客户端
// ================================================================
const MINIMAL_API_BASE_URL: &str = "https://api.minimal.com/v1";

pub const MODEL_DEFAULT: &str = "minimal-model-1";

pub struct ClientBuilder<'a> {
    api_key: &'a str,
    base_url: &'a str,
    http_client: Option<reqwest::Client>,
}

impl<'a> ClientBuilder<'a> {
    pub fn new(api_key: &'a str) -> Self {
        Self {
            api_key,
            base_url: MINIMAL_API_BASE_URL,
            http_client: None,
        }
    }

    pub fn base_url(mut self, base_url: &'a str) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn custom_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn build(self) -> Result<Client, ClientBuilderError> {
        let http_client = if let Some(http_client) = self.http_client {
            http_client
        } else {
            reqwest::Client::builder().build()?
        };

        Ok(Client {
            base_url: self.base_url.to_string(),
            api_key: self.api_key.to_string(),
            http_client,
        })
    }
}

#[derive(Clone)]
pub struct Client {
    base_url: String,
    api_key: String,
    http_client: reqwest::Client,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("http_client", &self.http_client)
            .field("api_key", &"<REDACTED>")
            .finish()
    }
}

impl Client {
    pub fn builder(api_key: &str) -> ClientBuilder<'_> {
        ClientBuilder::new(api_key)
    }

    pub fn new(api_key: &str) -> Self {
        Self::builder(api_key).build().expect("Client should build")
    }

    pub(crate) fn post(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/{}", self.base_url, path).replace("//", "/");
        self.http_client.post(url).bearer_auth(&self.api_key)
    }
}

impl ProviderClient for Client {
    fn from_env() -> Self {
        let api_key = std::env::var("MINIMAL_API_KEY").expect("MINIMAL_API_KEY not set");
        Self::new(&api_key)
    }

    fn from_val(input: crate::client::ProviderValue) -> Self {
        let crate::client::ProviderValue::Simple(api_key) = input else {
            panic!("Incorrect provider value type")
        };
        Self::new(&api_key)
    }
}

impl CompletionClient for Client {
    type CompletionModel = CompletionModel;

    fn completion_model(&self, model: &str) -> CompletionModel {
        CompletionModel {
            client: self.clone(),
            model: model.to_string(),
        }
    }
}

impl VerifyClient for Client {
    #[cfg_attr(feature = "worker", worker::send)]
    async fn verify(&self) -> Result<(), VerifyError> {
        Ok(())
    }
}

impl_conversion_traits!(
    AsEmbeddings,
    AsTranscription,
    AsImageGeneration,
    AsAudioGeneration for Client
);

// ================================================================
// 数据结构
// ================================================================
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: usize,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Usage {
    pub completion_tokens: u32,
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String },
}

// ================================================================
// CompletionModel
// ================================================================
#[derive(Clone)]
pub struct CompletionModel {
    client: Client,
    model: String,
}

impl completion::CompletionModel for CompletionModel {
    type Response = CompletionResponse;
    type StreamingResponse = CompletionResponse; // 简化：不支持流式

    #[cfg_attr(feature = "worker", worker::send)]
    async fn completion(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<completion::CompletionResponse<CompletionResponse>, CompletionError> {
        // 构建消息
        let mut messages = vec![];
        if let Some(preamble) = &completion_request.preamble {
            messages.push(Message::System {
                content: preamble.clone(),
            });
        }

        // 简化：只处理文本消息
        for msg in completion_request.chat_history {
            match msg {
                message::Message::User { content } => {
                    for item in content {
                        if let message::UserContent::Text(text) = item {
                            messages.push(Message::User {
                                content: text.text,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        let request = json!({
            "model": self.model,
            "messages": messages,
        });

        let response = self
            .client
            .post("/chat/completions")
            .json(&request)
            .send()
            .await?;

        let response: CompletionResponse = response.json().await?;

        // 转换响应
        let choice = completion::CompletionRequestChoice {
            content: vec![completion::AssistantContent::text(
                &response.choices[0].message.content
            )],
        };

        let usage = completion::Usage {
            input_tokens: response.usage.prompt_tokens as u64,
            output_tokens: response.usage.completion_tokens as u64,
            total_tokens: response.usage.total_tokens as u64,
        };

        Ok(completion::CompletionResponse {
            choice,
            usage,
            raw_response: response,
        })
    }

    #[cfg_attr(feature = "worker", worker::send)]
    async fn stream(
        &self,
        _completion_request: CompletionRequest,
    ) -> Result<
        crate::streaming::StreamingCompletionResponse<Self::StreamingResponse>,
        CompletionError,
    > {
        // 简化：不支持流式处理
        Err(CompletionError::ProviderError(
            "Streaming not supported".to_string(),
        ))
    }
}
```

---

## 测试与验证

### 1. 单元测试

在您的供应商文件中添加测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = Client::builder("test-api-key")
            .base_url("https://test.api.com")
            .build()
            .unwrap();

        assert_eq!(client.base_url, "https://test.api.com");
    }

    #[test]
    fn test_message_serialization() {
        let message = Message::User {
            content: "Hello".to_string(),
            name: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("user"));
    }

    #[tokio::test]
    async fn test_completion() {
        // 需要设置 API 密钥环境变量
        if std::env::var("YOUR_PROVIDER_API_KEY").is_err() {
            return;
        }

        let client = Client::from_env();
        let model = client.completion_model(MODEL_DEFAULT);

        let request = CompletionRequest::new("Say hello");
        let response = model.completion(request).await.unwrap();

        assert!(!response.choice.content.is_empty());
    }
}
```

### 2. 集成测试

在 `rig-core/tests/` 目录下创建集成测试：

```rust
// tests/your_provider_integration.rs

#[cfg(test)]
mod integration_tests {
    use rig::providers::your_provider;

    #[tokio::test]
    async fn test_basic_completion() {
        let api_key = std::env::var("YOUR_PROVIDER_API_KEY")
            .expect("YOUR_PROVIDER_API_KEY not set");

        let client = your_provider::Client::new(&api_key);
        let model = client.completion_model(your_provider::MODEL_DEFAULT);

        // 测试基本完成
        let response = model
            .completion(rig::completion::CompletionRequest::new("Hello"))
            .await
            .unwrap();

        assert!(!response.choice.content.is_empty());
    }

    #[tokio::test]
    async fn test_streaming() {
        let api_key = std::env::var("YOUR_PROVIDER_API_KEY")
            .expect("YOUR_PROVIDER_API_KEY not set");

        let client = your_provider::Client::new(&api_key);
        let model = client.completion_model(your_provider::MODEL_DEFAULT);

        // 测试流式处理
        use futures::StreamExt;
        let mut stream = model
            .stream(rig::completion::CompletionRequest::new("Count to 5"))
            .await
            .unwrap();

        let mut chunks = vec![];
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk);
        }

        assert!(!chunks.is_empty());
    }
}
```

### 3. 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定供应商的测试
cargo test your_provider

# 运行集成测试（需要 API 密钥）
YOUR_PROVIDER_API_KEY=xxx cargo test --test your_provider_integration
```

---

## 最佳实践

### 1. 错误处理

- **使用适当的错误类型**：利用 Rig 提供的错误类型
- **提供详细的错误信息**：帮助用户快速定位问题
- **记录调试信息**：使用 `tracing` 记录关键操作

```rust
// 好的错误处理
match response.status() {
    reqwest::StatusCode::OK => Ok(()),
    reqwest::StatusCode::UNAUTHORIZED => {
        Err(VerifyError::InvalidAuthentication)
    }
    reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
        tracing::error!("Provider internal error");
        Err(VerifyError::ProviderError(response.text().await?))
    }
    _ => {
        tracing::warn!("Unexpected status: {}", response.status());
        response.error_for_status()?;
        Ok(())
    }
}
```

### 2. 性能优化

- **重用 HTTP 客户端**：`reqwest::Client` 是线程安全的
- **使用连接池**：`reqwest::Client` 内置连接池
- **启用 HTTP/2**：如果 API 支持

```rust
let http_client = reqwest::Client::builder()
    .pool_max_idle_per_host(10)  // 设置连接池大小
    .timeout(Duration::from_secs(30))  // 设置超时
    .build()?;
```

### 3. 安全性

- **永远不要记录 API 密钥**：在 Debug 实现中隐藏密钥
- **使用 HTTPS**：确保 API 基础 URL 使用 HTTPS
- **验证响应数据**：不要盲目信任 API 响应

```rust
impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("api_key", &"<REDACTED>")  // 永远隐藏 API 密钥
            .finish()
    }
}
```

### 4. 可扩展性

- **使用构建器模式**：允许灵活配置
- **支持自定义 HTTP 客户端**：允许用户自定义配置
- **提供额外参数支持**：通过 `additional_params` 支持未来的 API 功能

```rust
// 允许用户自定义 HTTP 客户端
let custom_client = reqwest::Client::builder()
    .timeout(Duration::from_secs(60))
    .build()
    .unwrap();

let client = Client::builder("api-key")
    .custom_client(custom_client)
    .build()
    .unwrap();
```

### 5. 文档

- **提供完整的模块文档**：解释供应商的特性和限制
- **添加使用示例**：在文档注释中提供示例代码
- **注释复杂逻辑**：特别是类型转换和流式处理

```rust
//! Your Provider API 客户端和 Rig 集成
//!
//! # 特性
//! - 支持文本完成
//! - 支持流式响应
//! - 支持工具调用
//!
//! # 限制
//! - 最大上下文长度：128K tokens
//! - 不支持图像输入
//!
//! # 示例
//! ```no_run
//! use rig::providers::your_provider;
//!
//! let client = your_provider::Client::new("YOUR_API_KEY");
//! let model = client.completion_model(your_provider::MODEL_DEFAULT);
//!
//! // 基本完成
//! let response = model
//!     .completion(rig::completion::CompletionRequest::new("Hello"))
//!     .await
//!     .unwrap();
//! ```
```

### 6. 追踪和监控

- **使用 tracing spans**：为关键操作创建 span
- **记录重要指标**：令牌使用、延迟等
- **遵循 OpenTelemetry 语义**：使用标准的属性名

```rust
let span = info_span!(
    target: "rig::completions",
    "chat",
    gen_ai.operation.name = "chat",
    gen_ai.provider.name = "your_provider",
    gen_ai.request.model = self.model,
    gen_ai.usage.input_tokens = tracing::field::Empty,
    gen_ai.usage.output_tokens = tracing::field::Empty,
);

// 在操作完成后记录指标
span.record("gen_ai.usage.input_tokens", usage.prompt_tokens);
span.record("gen_ai.usage.output_tokens", usage.completion_tokens);
```

### 7. 兼容性

- **处理 API 版本变化**：为不同 API 版本提供支持
- **向后兼容**：不要破坏现有的公共 API
- **使用 feature flags**：为可选功能使用特性标志

```rust
#[cfg(feature = "your_provider_v2")]
pub mod v2 {
    // V2 API 实现
}

#[cfg(not(feature = "your_provider_v2"))]
pub use v1::*;
```

---

## 常见问题

### Q1: 我的供应商不支持工具调用，怎么办？

A: 在消息转换时忽略工具调用，只处理文本内容。

```rust
message::Message::Assistant { content, .. } => {
    let text = content
        .into_iter()
        .filter_map(|item| match item {
            completion::AssistantContent::Text(t) => Some(t.text),
            _ => None,  // 忽略工具调用
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(vec![Message::Assistant { content: text }])
}
```

### Q2: 如何处理供应商特定的功能？

A: 使用 `additional_params` 字段传递供应商特定的参数。

```rust
// 用户代码
let request = CompletionRequest::new("Hello")
    .additional_param("your_specific_feature", json!(true));

// 您的实现
fn create_completion_request(&self, req: CompletionRequest) -> Value {
    let mut request = json!({
        "model": self.model,
        "messages": messages,
    });

    // 合并额外参数
    if let Some(params) = req.additional_params {
        request = json_utils::merge(request, params);
    }

    request
}
```

### Q3: 流式处理很复杂，有简化的方法吗？

A: 如果您的供应商 API 与 OpenAI 兼容，可以重用 OpenAI 的流式处理实现。

```rust
// 重用 OpenAI 的流式处理
use crate::providers::openai::send_compatible_streaming_request;

impl completion::CompletionModel for CompletionModel {
    async fn stream(&self, req: CompletionRequest) -> Result<...> {
        let builder = self.client.post("/chat/completions").json(&request);
        send_compatible_streaming_request(builder).await
    }
}
```

### Q4: 如何测试没有 API 密钥的情况？

A: 使用 mock 服务器或跳过需要 API 密钥的测试。

```rust
#[tokio::test]
async fn test_completion() {
    // 如果没有 API 密钥，跳过测试
    if std::env::var("YOUR_PROVIDER_API_KEY").is_err() {
        eprintln!("Skipping test: YOUR_PROVIDER_API_KEY not set");
        return;
    }

    // 测试代码...
}
```

### Q5: 如何处理速率限制？

A: 在客户端层面实现重试逻辑。

```rust
impl Client {
    async fn post_with_retry(&self, path: &str) -> Result<Response, Error> {
        let mut retries = 0;
        const MAX_RETRIES: u32 = 3;

        loop {
            let response = self.post(path).send().await?;

            match response.status() {
                StatusCode::TOO_MANY_REQUESTS => {
                    if retries >= MAX_RETRIES {
                        return Err(Error::RateLimitExceeded);
                    }
                    
                    let wait_time = Duration::from_secs(2_u64.pow(retries));
                    tokio::time::sleep(wait_time).await;
                    retries += 1;
                }
                _ => return Ok(response),
            }
        }
    }
}
```

---

## 总结

添加新的模型供应商到 Rig 框架需要：

1. **理解架构** - 了解 Rig 的提供商系统和 trait 结构
2. **实现客户端** - 创建 Client 和 ClientBuilder
3. **实现模型** - 实现 CompletionModel 和其他模型类型
4. **类型转换** - 在 Rig 类型和供应商类型之间转换
5. **流式处理** - 实现 SSE 流式响应处理
6. **测试验证** - 编写单元测试和集成测试
7. **文档完善** - 提供清晰的文档和示例

遵循本指南，您应该能够成功地为 Rig 框架添加新的模型供应商支持！

---

## 参考资源

- [Rig 官方文档](https://github.com/0xPlaygrounds/rig)
- [现有供应商实现](https://github.com/0xPlaygrounds/rig/tree/main/rig-core/src/providers)
- [Rust async 编程](https://rust-lang.github.io/async-book/)
- [reqwest 文档](https://docs.rs/reqwest/)
- [serde 文档](https://serde.rs/)
- [tracing 文档](https://docs.rs/tracing/)

---

**版本**: 1.0  
**最后更新**: 2025-01-09  
**维护者**: Rig Community

