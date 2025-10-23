//! OpenAI 响应 API 的流式模块。
//! 请参见 `openai_streaming` 或 `openai_streaming_with_tools` 示例以获取更实用的用法。

// OpenAI 响应 API 流式模块
// 提供与 OpenAI 响应 API 的流式交互功能，包括实时响应处理和流式数据处理

// 导入完成模块的错误类型和获取令牌使用量 trait
use crate::completion::{CompletionError, GetTokenUsage};
// 导入 OpenAI 响应 API 模块的相关类型
use crate::providers::openai::responses_api::{
    ReasoningSummary, ResponsesCompletionModel, ResponsesUsage,
};
// 导入流式处理模块
use crate::streaming;
// 导入流式处理模块的原始流式选择类型
use crate::streaming::RawStreamingChoice;
// 导入异步流宏
use async_stream::stream;
// 导入 Future 流的扩展方法
use futures::StreamExt;
// 导入事件源的事件类型
use reqwest_eventsource::Event;
// 导入请求构建器的事件源扩展方法
use reqwest_eventsource::RequestBuilderExt;
// 导入序列化和反序列化宏
use serde::{Deserialize, Serialize};
// 导入跟踪模块的调试和信息跨度宏
use tracing::{debug, info_span};
// 导入跟踪 Future 的工具化 trait
use tracing_futures::Instrument as _;

// 导入父模块的完成响应和输出类型
use super::{CompletionResponse, Output};

// ================================================================
// OpenAI 响应流式 API
// ================================================================

/// 流式完成块。
/// 流式块可以以两种形式之一出现：
/// - 响应块（其中完成的响应将具有总 token 使用量）
/// - 通常称为增量的项目块。在完成 API 中，这将称为消息增量。
// 流式完成块枚举
// 流式块可以以两种形式之一出现：
// - 响应块（其中完成的响应将具有总令牌使用量）
// - 通常称为增量的项目块。在完成 API 中，这将称为消息增量
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
// 流式完成块枚举
pub enum StreamingCompletionChunk {
    // 响应块
    Response(Box<ResponseChunk>),
    // 增量块
    Delta(ItemChunk),
}

/// The final streaming response from the OpenAI Responses API.
// 来自 OpenAI 响应 API 的最终流式响应
#[derive(Debug, Serialize, Deserialize, Clone)]
// 流式完成响应结构体
pub struct StreamingCompletionResponse {
    /// Token usage
    // 令牌使用情况
    pub usage: ResponsesUsage,
}

// 为 StreamingCompletionResponse 实现 GetTokenUsage trait
impl GetTokenUsage for StreamingCompletionResponse {
    // 获取令牌使用情况
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        // 创建新的使用情况
        let mut usage = crate::completion::Usage::new();
        // 设置输入令牌数
        usage.input_tokens = self.usage.input_tokens;
        // 设置输出令牌数
        usage.output_tokens = self.usage.output_tokens;
        // 设置总令牌数
        usage.total_tokens = self.usage.total_tokens;
        Some(usage)
    }
}

/// A response chunk from OpenAI's response API.
// 来自 OpenAI 响应 API 的响应块
#[derive(Debug, Serialize, Deserialize, Clone)]
// 响应块结构体
pub struct ResponseChunk {
    /// The response chunk type
    // 响应块类型
    #[serde(rename = "type")]
    pub kind: ResponseChunkKind,
    /// The response itself
    // 响应本身
    pub response: CompletionResponse,
    /// The item sequence
    // 项目序列号
    pub sequence_number: u64,
}

/// Response chunk type.
/// Renames are used to ensure that this type gets (de)serialized properly.
// 响应块类型
// 使用重命名以确保此类型正确（反）序列化
#[derive(Debug, Serialize, Deserialize, Clone)]
// 响应块类型枚举
pub enum ResponseChunkKind {
    // 响应已创建
    #[serde(rename = "response.created")]
    ResponseCreated,
    // 响应进行中
    #[serde(rename = "response.in_progress")]
    ResponseInProgress,
    // 响应已完成
    #[serde(rename = "response.completed")]
    ResponseCompleted,
    // 响应失败
    #[serde(rename = "response.failed")]
    ResponseFailed,
    // 响应不完整
    #[serde(rename = "response.incomplete")]
    ResponseIncomplete,
}

/// An item message chunk from OpenAI's Responses API.
/// See
// 来自 OpenAI 响应 API 的项目消息块
#[derive(Debug, Serialize, Deserialize, Clone)]
// 项目块结构体
pub struct ItemChunk {
    /// Item ID. Optional.
    // 项目 ID（可选）
    pub item_id: Option<String>,
    /// The output index of the item from a given streamed response.
    // 来自给定流式响应的项目输出索引
    pub output_index: u64,
    /// The item type chunk, as well as the inner data.
    // 项目类型块以及内部数据
    #[serde(flatten)]
    pub data: ItemChunkKind,
}

/// The item chunk type from OpenAI's Responses API.
// 来自 OpenAI 响应 API 的项目块类型
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
// 项目块类型枚举
pub enum ItemChunkKind {
    // 输出项目已添加
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded(StreamingItemDoneOutput),
    // 输出项目已完成
    #[serde(rename = "response.output_item.done")]
    OutputItemDone(StreamingItemDoneOutput),
    // 内容部分已添加
    #[serde(rename = "response.content_part.added")]
    ContentPartAdded(ContentPartChunk),
    // 内容部分已完成
    #[serde(rename = "response.content_part.done")]
    ContentPartDone(ContentPartChunk),
    // 输出文本增量
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta(DeltaTextChunk),
    // 输出文本已完成
    #[serde(rename = "response.output_text.done")]
    OutputTextDone(OutputTextChunk),
    // 拒绝增量
    #[serde(rename = "response.refusal.delta")]
    RefusalDelta(DeltaTextChunk),
    // 拒绝已完成
    #[serde(rename = "response.refusal.done")]
    RefusalDone(RefusalTextChunk),
    // 函数调用参数增量
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgsDelta(DeltaTextChunk),
    // 函数调用参数已完成
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgsDone(ArgsTextChunk),
    // 推理摘要部分已添加
    #[serde(rename = "response.reasoning_summary_part.added")]
    ReasoningSummaryPartAdded(SummaryPartChunk),
    // 推理摘要部分已完成
    #[serde(rename = "response.reasoning_summary_part.done")]
    ReasoningSummaryPartDone(SummaryPartChunk),
    // 推理摘要文本已添加
    #[serde(rename = "response.reasoning_summary_text.added")]
    ReasoningSummaryTextAdded(SummaryTextChunk),
    // 推理摘要文本已完成
    #[serde(rename = "response.reasoning_summary_text.done")]
    ReasoningSummaryTextDone(SummaryTextChunk),
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 流式项目完成输出结构体
pub struct StreamingItemDoneOutput {
    // 序列号
    pub sequence_number: u64,
    // 输出项目
    pub item: Output,
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 内容部分块结构体
pub struct ContentPartChunk {
    // 内容索引
    pub content_index: u64,
    // 序列号
    pub sequence_number: u64,
    // 内容部分块部分
    pub part: ContentPartChunkPart,
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
// 内容部分块部分枚举
pub enum ContentPartChunkPart {
    // 输出文本
    OutputText { text: String },
    // 摘要文本
    SummaryText { text: String },
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 增量文本块结构体
pub struct DeltaTextChunk {
    // 内容索引
    pub content_index: u64,
    // 序列号
    pub sequence_number: u64,
    // 增量文本
    pub delta: String,
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 输出文本块结构体
pub struct OutputTextChunk {
    // 内容索引
    pub content_index: u64,
    // 序列号
    pub sequence_number: u64,
    // 文本内容
    pub text: String,
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 拒绝文本块结构体
pub struct RefusalTextChunk {
    // 内容索引
    pub content_index: u64,
    // 序列号
    pub sequence_number: u64,
    // 拒绝文本
    pub refusal: String,
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 参数文本块结构体
pub struct ArgsTextChunk {
    // 内容索引
    pub content_index: u64,
    // 序列号
    pub sequence_number: u64,
    // 参数值
    pub arguments: serde_json::Value,
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 摘要部分块结构体
pub struct SummaryPartChunk {
    // 摘要索引
    pub summary_index: u64,
    // 序列号
    pub sequence_number: u64,
    // 摘要部分块部分
    pub part: SummaryPartChunkPart,
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 摘要文本块结构体
pub struct SummaryTextChunk {
    // 摘要索引
    pub summary_index: u64,
    // 序列号
    pub sequence_number: u64,
    // 增量文本
    pub delta: String,
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
// 摘要部分块部分枚举
pub enum SummaryPartChunkPart {
    // 摘要文本
    SummaryText { text: String },
}

// ResponsesCompletionModel 的实现
impl ResponsesCompletionModel {
    // 流式方法（包可见）
    pub(crate) async fn stream(
        &self,
        completion_request: crate::completion::CompletionRequest,
    ) -> Result<streaming::StreamingCompletionResponse<StreamingCompletionResponse>, CompletionError>
    {
        // 创建完成请求并启用流式
        let mut request = self.create_completion_request(completion_request)?;
        request.stream = Some(true);

        // 创建请求构建器
        let request_builder = self.client.post("/responses").json(&request);

        // 创建跟踪跨度
        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat_streaming",
                gen_ai.operation.name = "chat_streaming",
                gen_ai.provider.name = tracing::field::Empty,
                gen_ai.request.model = tracing::field::Empty,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = tracing::field::Empty,
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };
        // 记录提供商名称和模型
        span.record("gen_ai.provider.name", "openai");
        span.record("gen_ai.request.model", &self.model);
        // 记录输入消息
        span.record(
            "gen_ai.input.messages",
            serde_json::to_string(&request.input).expect("This should always work"),
        );
        // Build the request with proper headers for SSE
        // 使用适当的 SSE 头部构建请求
        let mut event_source = request_builder
            .eventsource()
            .expect("Cloning request must always succeed");

        // 创建异步流
        let stream = Box::pin(stream! {
            // 创建最终使用情况
            let mut final_usage = ResponsesUsage::new();

            // 创建工具调用向量
            let mut tool_calls: Vec<RawStreamingChoice<StreamingCompletionResponse>> = Vec::new();
            // 创建组合文本字符串
            let mut combined_text = String::new();
            // 获取当前跟踪跨度
            let span = tracing::Span::current();

            // 处理事件源事件
            while let Some(event_result) = event_source.next().await {
                // 匹配事件结果
                match event_result {
                    // SSE 连接打开事件
                    Ok(Event::Open) => {
                        tracing::trace!("SSE connection opened");
                        tracing::info!("OpenAI stream started");
                        continue;
                    }
                    // SSE 消息事件
                    Ok(Event::Message(message)) => {
                        // Skip heartbeat messages or empty data
                        // 跳过心跳消息或空数据
                        if message.data.trim().is_empty() {
                            continue;
                        }

                        // 尝试解析流式完成块
                        let data = serde_json::from_str::<StreamingCompletionChunk>(&message.data);

                        let Ok(data) = data else {
                            let err = data.unwrap_err();
                            debug!("Couldn't serialize data as StreamingCompletionResponse: {:?}", err);
                            continue;
                        };

                        // 处理增量块
                        if let StreamingCompletionChunk::Delta(chunk) = &data {
                            match &chunk.data {
                                // 输出项目已完成
                                ItemChunkKind::OutputItemDone(message) => {
                                    match message {
                                        // 函数调用输出
                                        StreamingItemDoneOutput {  item: Output::FunctionCall(func), .. } => {
                                            tool_calls.push(streaming::RawStreamingChoice::ToolCall { id: func.id.clone(), call_id: Some(func.call_id.clone()), name: func.name.clone(), arguments: func.arguments.clone() });
                                        }

                                        // 推理输出
                                        StreamingItemDoneOutput {  item: Output::Reasoning {  summary, id }, .. } => {
                                            // 转换推理摘要为字符串
                                            let reasoning = summary
                                                .iter()
                                                .map(|x| {
                                                    let ReasoningSummary::SummaryText { text } = x;
                                                    text.to_owned()
                                                })
                                                .collect::<Vec<String>>()
                                                .join("\n");
                                            // 产生推理结果
                                            yield Ok(streaming::RawStreamingChoice::Reasoning { reasoning, id: Some(id.to_string()) })
                                        }
                                        // 其他输出类型，继续处理
                                        _ => continue
                                    }
                                }
                                // 输出文本增量
                                ItemChunkKind::OutputTextDelta(delta) => {
                                    // 追加增量文本到组合文本
                                    combined_text.push_str(&delta.delta);
                                    // 产生消息结果
                                    yield Ok(streaming::RawStreamingChoice::Message(delta.delta.clone()))
                                }
                                // 拒绝增量
                                ItemChunkKind::RefusalDelta(delta) => {
                                    // 追加增量文本到组合文本
                                    combined_text.push_str(&delta.delta);
                                    // 产生消息结果
                                    yield Ok(streaming::RawStreamingChoice::Message(delta.delta.clone()))
                                }

                                // 其他类型，继续处理
                                _ => { continue }
                            }
                        }

                        // 处理响应块
                        if let StreamingCompletionChunk::Response(chunk) = data {
                            // 检查是否是响应完成块
                            if let ResponseChunk { kind: ResponseChunkKind::ResponseCompleted, response, .. } = *chunk {
                                // 记录输出消息
                                span.record("gen_ai.output.messages", serde_json::to_string(&response.output).unwrap());
                                // 记录响应 ID 和模型
                                span.record("gen_ai.response.id", response.id);
                                span.record("gen_ai.response.model", response.model);
                                // 更新使用情况
                                if let Some(usage) = response.usage {
                                    final_usage = usage;
                                }
                            } else {
                                // 其他响应类型，继续处理
                                continue;
                            }
                        }
                    }
                    // 流结束错误
                    Err(reqwest_eventsource::Error::StreamEnded) => {
                        break;
                    }
                    // 其他错误
                    Err(error) => {
                        // 记录错误
                        tracing::error!(?error, "SSE error");
                        // 产生错误结果
                        yield Err(CompletionError::ResponseError(error.to_string()));
                        break;
                    }
                }
            }

            // Ensure event source is closed when stream ends
            // 确保流结束时关闭事件源
            event_source.close();

            // 产生所有工具调用
            for tool_call in &tool_calls {
                yield Ok(tool_call.to_owned())
            }

            // 记录使用情况
            span.record("gen_ai.usage.input_tokens", final_usage.input_tokens);
            span.record("gen_ai.usage.output_tokens", final_usage.output_tokens);
            // 记录流完成信息
            tracing::info!("OpenAI stream finished");

            // 产生最终响应
            yield Ok(RawStreamingChoice::FinalResponse(StreamingCompletionResponse {
                usage: final_usage.clone()
            }));
        }.instrument(span));

        // 返回流式完成响应
        Ok(streaming::StreamingCompletionResponse::stream(stream))
    }
}
