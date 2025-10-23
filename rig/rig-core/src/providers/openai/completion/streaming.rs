// 导入完成模块的错误类型、完成请求和获取令牌使用量 trait
use crate::completion::{CompletionError, CompletionRequest, GetTokenUsage};
// 导入 JSON 工具模块
use crate::json_utils;
// 导入 JSON 工具模块的合并函数
use crate::json_utils::merge;
// 导入 OpenAI 完成模块的完成模型和使用情况类型
use crate::providers::openai::completion::{CompletionModel, Usage};
// 导入流式处理模块
use crate::streaming;
// 导入流式处理模块的原始流式选择类型
use crate::streaming::RawStreamingChoice;
// 导入异步流宏
use async_stream::stream;
// 导入 Future 流的扩展方法
use futures::StreamExt;
// 导入请求构建器类型
use reqwest::RequestBuilder;
// 导入事件源的事件类型
use reqwest_eventsource::Event;
// 导入请求构建器的事件源扩展方法
use reqwest_eventsource::RequestBuilderExt;
// 导入序列化和反序列化宏
use serde::{Deserialize, Serialize};
// 导入 JSON 宏
use serde_json::json;
// 导入哈希映射集合
use std::collections::HashMap;
// 导入跟踪模块的调试和信息跨度宏
use tracing::{debug, info_span};
// 导入跟踪 Future 的工具化 trait
use tracing_futures::Instrument;

// ================================================================
// OpenAI 完成流式 API
// ================================================================
// OpenAI 完成流式 API 模块
// 提供与 OpenAI 聊天完成流式 API 的交互功能

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 流式函数结构体
pub struct StreamingFunction {
    // 使用默认值的函数名称（可选）
    #[serde(default)]
    pub name: Option<String>,
    // 使用默认值的函数参数
    #[serde(default)]
    pub arguments: String,
}

// 派生 Debug、Serialize、Deserialize 和 Clone trait
#[derive(Debug, Serialize, Deserialize, Clone)]
// 流式工具调用结构体
pub struct StreamingToolCall {
    // 工具调用索引
    pub index: usize,
    // 工具调用 ID（可选）
    pub id: Option<String>,
    // 流式函数
    pub function: StreamingFunction,
}

// 派生 Deserialize 和 Debug trait
#[derive(Deserialize, Debug)]
// 流式增量结构体
struct StreamingDelta {
    // 使用默认值的内容（可选）
    #[serde(default)]
    content: Option<String>,
    // 使用默认值和自定义反序列化器的工具调用列表
    #[serde(default, deserialize_with = "json_utils::null_or_vec")]
    tool_calls: Vec<StreamingToolCall>,
}

// 派生 Deserialize 和 Debug trait
#[derive(Deserialize, Debug)]
// 流式选择结构体
struct StreamingChoice {
    // 流式增量
    delta: StreamingDelta,
}

// 派生 Deserialize 和 Debug trait
#[derive(Deserialize, Debug)]
// 流式完成块结构体
struct StreamingCompletionChunk {
    // 选择列表
    choices: Vec<StreamingChoice>,
    // 使用情况（可选）
    usage: Option<Usage>,
}

// 派生 Clone、Serialize 和 Deserialize trait
#[derive(Clone, Serialize, Deserialize)]
// 流式完成响应结构体
pub struct StreamingCompletionResponse {
    // 使用情况
    pub usage: Usage,
}

// 为 StreamingCompletionResponse 实现 GetTokenUsage trait
impl GetTokenUsage for StreamingCompletionResponse {
    // 获取令牌使用情况
    fn token_usage(&self) -> Option<crate::completion::Usage> {
        // 创建新的使用情况
        let mut usage = crate::completion::Usage::new();
        // 设置输入令牌数
        usage.input_tokens = self.usage.prompt_tokens as u64;
        // 设置输出令牌数
        usage.output_tokens = self.usage.total_tokens as u64 - self.usage.prompt_tokens as u64;
        // 设置总令牌数
        usage.total_tokens = self.usage.total_tokens as u64;
        Some(usage)
    }
}

// CompletionModel 结构体的实现
impl CompletionModel {
    // 流式方法（包可见）
    pub(crate) async fn stream(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<streaming::StreamingCompletionResponse<StreamingCompletionResponse>, CompletionError>
    {
        // 转换完成请求
        let request = super::CompletionRequest::try_from((self.model.clone(), completion_request))?;
        // 将请求消息转换为 JSON 字符串
        let request_messages = serde_json::to_string(&request.messages)
            .expect("Converting to JSON from a Rust struct shouldn't fail");
        // 将请求转换为 JSON 值
        let mut request_as_json = serde_json::to_value(request).expect("this should never fail");

        // 合并流式选项
        request_as_json = merge(
            request_as_json,
            json!({"stream": true, "stream_options": {"include_usage": true}}),
        );

        // 创建请求构建器
        let builder = self.client.post("/chat/completions").json(&request_as_json);

        // 创建跟踪跨度
        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat",
                gen_ai.operation.name = "chat",
                gen_ai.provider.name = "openai",
                gen_ai.request.model = self.model,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = self.model,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = request_messages,
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };

        // 使用跨度工具化发送兼容的流式请求
        tracing::Instrument::instrument(send_compatible_streaming_request(builder), span).await
    }
}

// 发送兼容的流式请求函数
pub async fn send_compatible_streaming_request(
    request_builder: RequestBuilder,
) -> Result<streaming::StreamingCompletionResponse<StreamingCompletionResponse>, CompletionError> {
    // 获取当前跟踪跨度
    let span = tracing::Span::current();
    // Build the request with proper headers for SSE
    // 使用适当的 SSE 头部构建请求
    let mut event_source = request_builder
        .eventsource()
        .expect("Cloning request must always succeed");

    // 创建异步流
    let stream = Box::pin(stream! {
        // 获取当前跟踪跨度
        let span = tracing::Span::current();
        // 创建最终使用情况
        let mut final_usage = Usage::new();

        // Track in-progress tool calls
        // 跟踪进行中的工具调用
        let mut tool_calls: HashMap<usize, (String, String, String)> = HashMap::new();

        // 创建文本内容字符串
        let mut text_content = String::new();

        // 处理事件源事件
        while let Some(event_result) = event_source.next().await {
            // 匹配事件结果
            match event_result {
                // SSE 连接打开事件
                Ok(Event::Open) => {
                    tracing::trace!("SSE connection opened");
                    continue;
                }
                // SSE 消息事件
                Ok(Event::Message(message)) => {
                    // 跳过空数据或完成标记
                    if message.data.trim().is_empty() || message.data == "[DONE]" {
                        continue;
                    }

                    // 尝试解析流式完成块
                    let data = serde_json::from_str::<StreamingCompletionChunk>(&message.data);
                    let Ok(data) = data else {
                        let err = data.unwrap_err();
                        debug!("Couldn't serialize data as StreamingCompletionChunk: {:?}", err);
                        continue;
                    };

                    // 处理第一个选择
                    if let Some(choice) = data.choices.first() {
                        // 获取增量数据
                        let delta = &choice.delta;

                        // Tool calls
                        // 工具调用
                        if !delta.tool_calls.is_empty() {
                            // 处理每个工具调用
                            for tool_call in &delta.tool_calls {
                                // 克隆函数信息
                                let function = tool_call.function.clone();

                                // Start of tool call
                                // 工具调用开始
                                if function.name.is_some() && function.arguments.is_empty() {
                                    // 获取工具调用 ID
                                    let id = tool_call.id.clone().unwrap_or_default();
                                    // 插入工具调用到映射中
                                    tool_calls.insert(
                                        tool_call.index,
                                        (id, function.name.clone().unwrap(), "".to_string()),
                                    );
                                }
                                // tool call partial (ie, a continuation of a previously received tool call)
                                // name: None or Empty String
                                // arguments: Some(String)
                                // 工具调用部分（即，先前接收的工具调用的继续）
                                // name: None 或空字符串
                                // arguments: Some(String)
                                else if function.name.clone().is_none_or(|s| s.is_empty())
                                    && !function.arguments.is_empty()
                                {
                                    // 如果存在部分工具调用
                                    if let Some((id, name, arguments)) =
                                        tool_calls.get(&tool_call.index)
                                    {
                                        // 获取新参数
                                        let new_arguments = &tool_call.function.arguments;
                                        // 合并参数
                                        let arguments = format!("{arguments}{new_arguments}");
                                        // 更新工具调用映射
                                        tool_calls.insert(
                                            tool_call.index,
                                            (id.clone(), name.clone(), arguments),
                                        );
                                    } else {
                                        debug!("Partial tool call received but tool call was never started.");
                                    }
                                }
                                // Complete tool call
                                // 完整的工具调用
                                else {
                                    // 获取工具调用 ID
                                    let id = tool_call.id.clone().unwrap_or_default();
                                    // 获取工具调用名称
                                    let name = function.name.expect("tool call should have a name");
                                    // 获取工具调用参数
                                    let arguments = function.arguments;
                                    // 尝试解析参数为 JSON
                                    let Ok(arguments) = serde_json::from_str(&arguments) else {
                                        debug!("Couldn't serialize '{arguments}' as JSON");
                                        continue;
                                    };

                                    // 产生工具调用结果
                                    yield Ok(streaming::RawStreamingChoice::ToolCall {
                                        id,
                                        name,
                                        arguments,
                                        call_id: None,
                                    });
                                }
                            }
                        }

                        // Message content
                        // 消息内容
                        if let Some(content) = &choice.delta.content {
                            // 追加内容到文本内容
                            text_content += content;
                            // 产生消息结果
                            yield Ok(streaming::RawStreamingChoice::Message(content.clone()))
                        }
                    }

                    // Usage updates
                    // 使用情况更新
                    if let Some(usage) = data.usage {
                        // 更新最终使用情况
                        final_usage = usage.clone();
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

        // 创建工具调用向量
        let mut vec_toolcalls = vec![];

        // Flush any tool calls that weren't fully yielded
        // 刷新任何未完全产生的工具调用
        for (_, (id, name, arguments)) in tool_calls {
            // 尝试解析参数为 JSON 值
            let Ok(arguments) = serde_json::from_str::<serde_json::Value>(&arguments) else {
                continue;
            };

            // 添加到工具调用向量
            vec_toolcalls.push(super::ToolCall {
                r#type: super::ToolType::Function,
                id: id.clone(),
                function: super::Function {
                    name: name.clone(), arguments: arguments.clone()
                },
            });

            // 产生工具调用结果
            yield Ok(RawStreamingChoice::ToolCall {
                id,
                name,
                arguments,
                call_id: None,
            });
        }

        // 创建消息输出
        let message_output = super::Message::Assistant {
            content: vec![super::AssistantContent::Text { text: text_content }],
            refusal: None,
            audio: None,
            name: None,
            tool_calls: vec_toolcalls
        };

        // 记录遥测数据
        span.record("gen_ai.usage.input_tokens", final_usage.prompt_tokens);
        span.record("gen_ai.usage.output_tokens", final_usage.total_tokens - final_usage.prompt_tokens);
        span.record("gen_ai.output.messages", serde_json::to_string(&vec![message_output]).expect("Converting from a Rust struct should always convert to JSON without failing"));

        // 产生最终响应
        yield Ok(RawStreamingChoice::FinalResponse(StreamingCompletionResponse {
            usage: final_usage.clone()
        }));
    }.instrument(span));

    // 返回流式完成响应
    Ok(streaming::StreamingCompletionResponse::stream(stream))
}
