//! 此模块提供使用 LLM 从文本中提取结构化数据的高级抽象。
//!
//! 注意：目标结构必须实现 `serde::Deserialize`、`serde::Serialize` 和
//! `schemars::JsonSchema` trait。这些可以使用 `derive` 宏轻松派生。
//!
//! # 示例
//! ```
//! use rig::providers::openai;
//!
//! // 初始化 OpenAI 客户端
//! let openai = openai::Client::new("your-open-ai-api-key");
//!
//! // 定义您要提取的数据结构
//! #[derive(serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
//! struct Person {
//!    name: Option<String>,
//!    age: Option<u8>,
//!    profession: Option<String>,
//! }
//!
//! // 创建提取器
//! let extractor = openai.extractor::<Person>(openai::GPT_4O)
//!     .build();
//!
//! // 从文本中提取结构化数据
//! let person = extractor.extract("John Doe is a 30 year old doctor.")
//!     .await
//!     .expect("Failed to extract data from text");
//! ```

// 导入类型标记类型
use std::marker::PhantomData;

// 导入 JSON Schema 相关类型
use schemars::{JsonSchema, schema_for};
// 导入序列化和反序列化相关类型
use serde::{Deserialize, Serialize};
// 导入 JSON 值类型
use serde_json::json;

// 导入代理相关类型
use crate::{
    // 导入代理和代理构建器
    agent::{Agent, AgentBuilder},
    // 导入完成、完成错误、完成模型、工具定义类型
    completion::{Completion, CompletionError, CompletionModel, ToolDefinition},
    // 导入助手内容、消息、工具调用、工具选择、工具函数类型
    message::{AssistantContent, Message, ToolCall, ToolChoice, ToolFunction},
    // 导入工具 trait
    tool::Tool,
};

// 定义提交工具的名称常量
const SUBMIT_TOOL_NAME: &str = "submit";

// 派生 Debug 和 thiserror::Error trait
#[derive(Debug, thiserror::Error)]
// 提取错误枚举
pub enum ExtractionError {
    // 没有提取到数据
    #[error("No data extracted")]
    NoData,

    // 反序列化提取的数据失败
    #[error("Failed to deserialize the extracted data: {0}")]
    DeserializationError(#[from] serde_json::Error),

    // 完成错误
    #[error("CompletionError: {0}")]
    CompletionError(#[from] CompletionError),
}

/// Extractor for structured data from text
// 从文本中提取结构化数据的提取器
pub struct Extractor<M, T>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // T 必须实现 JsonSchema、Deserialize、Send、Sync trait
    T: JsonSchema + for<'a> Deserialize<'a> + Send + Sync,
{
    // 代理实例
    agent: Agent<M>,
    // 类型标记，用于在编译时跟踪类型 T
    _t: PhantomData<T>,
    // 重试次数
    retries: u64,
}

// 为 Extractor 实现方法
impl<M, T> Extractor<M, T>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // T 必须实现 JsonSchema、Deserialize、Send、Sync trait
    T: JsonSchema + for<'a> Deserialize<'a> + Send + Sync,
{
    /// Attempts to extract data from the given text with a number of retries.
    ///
    /// The function will retry the extraction if the initial attempt fails or
    /// if the model does not call the `submit` tool.
    ///
    /// The number of retries is determined by the `retries` field on the Extractor struct.
    // 尝试从给定文本中提取数据，支持多次重试
    // 如果初始尝试失败或模型没有调用 `submit` 工具，函数将重试提取
    // 重试次数由 Extractor 结构体上的 `retries` 字段确定
    pub async fn extract(&self, text: impl Into<Message> + Send) -> Result<T, ExtractionError> {
        // 初始化最后一个错误
        let mut last_error = None;
        // 将输入转换为消息
        let text_message = text.into();

        // 循环重试，从 0 到重试次数
        for i in 0..=self.retries {
            // 记录调试信息
            tracing::debug!(
                "Attempting to extract JSON. Retries left: {retries}",
                retries = self.retries - i
            );
            // 克隆消息用于尝试
            let attempt_text = text_message.clone();
            // 尝试提取 JSON 数据
            match self.extract_json(attempt_text, vec![]).await {
                // 成功提取数据，返回结果
                Ok(data) => return Ok(data),
                // 提取失败，记录错误并继续重试
                Err(e) => {
                    tracing::warn!("Attempt {i} to extract JSON failed: {e:?}. Retrying...");
                    last_error = Some(e);
                }
            }
        }

        // If the loop finishes without a successful extraction, return the last error encountered.
        // 如果循环结束时没有成功提取，返回遇到的最后一个错误
        Err(last_error.unwrap_or(ExtractionError::NoData))
    }

    /// Attempts to extract data from the given text with a number of retries.
    ///
    /// The function will retry the extraction if the initial attempt fails or
    /// if the model does not call the `submit` tool.
    ///
    /// The number of retries is determined by the `retries` field on the Extractor struct.
    // 尝试从给定文本中提取数据，支持多次重试和聊天历史
    // 如果初始尝试失败或模型没有调用 `submit` 工具，函数将重试提取
    // 重试次数由 Extractor 结构体上的 `retries` 字段确定
    pub async fn extract_with_chat_history(
        &self,
        // 要提取的文本
        text: impl Into<Message> + Send,
        // 聊天历史消息
        chat_history: Vec<Message>,
    ) -> Result<T, ExtractionError> {
        // 初始化最后一个错误
        let mut last_error = None;
        // 将输入转换为消息
        let text_message = text.into();

        // 循环重试，从 0 到重试次数
        for i in 0..=self.retries {
            // 记录调试信息
            tracing::debug!(
                "Attempting to extract JSON. Retries left: {retries}",
                retries = self.retries - i
            );
            // 克隆消息用于尝试
            let attempt_text = text_message.clone();
            // 尝试提取 JSON 数据，传入聊天历史
            match self.extract_json(attempt_text, chat_history.clone()).await {
                // 成功提取数据，返回结果
                Ok(data) => return Ok(data),
                // 提取失败，记录错误并继续重试
                Err(e) => {
                    tracing::warn!("Attempt {i} to extract JSON failed: {e:?}. Retrying...");
                    last_error = Some(e);
                }
            }
        }

        // If the loop finishes without a successful extraction, return the last error encountered.
        // 如果循环结束时没有成功提取，返回遇到的最后一个错误
        Err(last_error.unwrap_or(ExtractionError::NoData))
    }

    // 异步提取 JSON 数据的内部方法
    async fn extract_json(
        &self,
        // 要提取的文本
        text: impl Into<Message> + Send,
        // 消息历史
        messages: Vec<Message>,
    ) -> Result<T, ExtractionError> {
        // 调用代理完成请求并发送
        let response = self.agent.completion(text, messages).await?.send().await?;

        // 检查是否有调用提交工具
        if !response.choice.iter().any(|x| {
            // 匹配工具调用内容
            let AssistantContent::ToolCall(ToolCall {
                function: ToolFunction { name, .. },
                ..
            }) = x
            else {
                return false;
            };

            // 检查是否是提交工具
            name == SUBMIT_TOOL_NAME
        }) {
            // 如果没有调用提交工具，记录警告
            tracing::warn!(
                "The submit tool was not called. If this happens more than once, please ensure the model you are using is powerful enough to reliably call tools."
            );
        }

        // 提取提交工具的参数
        let arguments = response
            .choice
            .into_iter()
            // We filter tool calls to look for submit tool calls
            // 我们过滤工具调用以查找提交工具调用
            .filter_map(|content| {
                // 匹配工具调用内容
                if let AssistantContent::ToolCall(ToolCall {
                    function: ToolFunction { arguments, name },
                    ..
                }) = content
                {
                    // 如果是提交工具，返回参数
                    if name == SUBMIT_TOOL_NAME {
                        Some(arguments)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // 检查是否有多个提交调用
        if arguments.len() > 1 {
            tracing::warn!(
                "Multiple submit calls detected, using the last one. Providers / agents should only ensure one submit call."
            );
        }

        // 获取第一个参数，如果没有则返回错误
        let raw_data = if let Some(arg) = arguments.into_iter().next() {
            arg
        } else {
            return Err(ExtractionError::NoData);
        };

        // 将 JSON 值反序列化为目标类型
        Ok(serde_json::from_value(raw_data)?)
    }

    // 获取内部代理的引用
    pub async fn get_inner(&self) -> &Agent<M> {
        &self.agent
    }

    // 获取内部代理的所有权
    pub async fn into_inner(self) -> Agent<M> {
        self.agent
    }
}

/// Builder for the Extractor
// 提取器的构建器
pub struct ExtractorBuilder<M, T>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // T 必须实现 JsonSchema、Deserialize、Serialize、Send、Sync trait，且生命周期为 'static
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
{
    // 代理构建器
    agent_builder: AgentBuilder<M>,
    // 类型标记，用于在编译时跟踪类型 T
    _t: PhantomData<T>,
    // 重试次数（可选）
    retries: Option<u64>,
}

// 为 ExtractorBuilder 实现方法
impl<M, T> ExtractorBuilder<M, T>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
    // T 必须实现 JsonSchema、Deserialize、Serialize、Send、Sync trait，且生命周期为 'static
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync + 'static,
{
    // 创建新的提取器构建器
    pub fn new(model: M) -> Self {
        Self {
            // 创建代理构建器并设置前导文本和工具
            agent_builder: AgentBuilder::new(model)
                .preamble("\
                    You are an AI assistant whose purpose is to extract structured data from the provided text.\n\
                    You will have access to a `submit` function that defines the structure of the data to extract from the provided text.\n\
                    Use the `submit` function to submit the structured data.\n\
                    Be sure to fill out every field and ALWAYS CALL THE `submit` function, even with default values!!!.
                ")
                .tool(SubmitTool::<T> {_t: PhantomData})
                .tool_choice(ToolChoice::Required),
            // 初始化重试次数为 None
            retries: None,
            // 初始化类型标记
            _t: PhantomData,
        }
    }

    /// Add additional preamble to the extractor
    // 为提取器添加额外的前导文本
    pub fn preamble(mut self, preamble: &str) -> Self {
        // 向代理构建器添加额外的前导文本
        self.agent_builder = self.agent_builder.append_preamble(&format!(
            "\n=============== ADDITIONAL INSTRUCTIONS ===============\n{preamble}"
        ));
        // 返回 self 以支持链式调用
        self
    }

    /// Add a context document to the extractor
    // 为提取器添加上下文文档
    pub fn context(mut self, doc: &str) -> Self {
        // 向代理构建器添加上下文文档
        self.agent_builder = self.agent_builder.context(doc);
        // 返回 self 以支持链式调用
        self
    }

    // 设置附加参数
    pub fn additional_params(mut self, params: serde_json::Value) -> Self {
        // 向代理构建器添加附加参数
        self.agent_builder = self.agent_builder.additional_params(params);
        // 返回 self 以支持链式调用
        self
    }

    /// Set the maximum number of tokens for the completion
    // 设置完成的最大令牌数
    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        // 设置代理构建器的最大令牌数
        self.agent_builder = self.agent_builder.max_tokens(max_tokens);
        // 返回 self 以支持链式调用
        self
    }

    /// Set the maximum number of retries for the extractor.
    // 设置提取器的最大重试次数
    pub fn retries(mut self, retries: u64) -> Self {
        // 设置重试次数
        self.retries = Some(retries);
        // 返回 self 以支持链式调用
        self
    }

    /// Set the `tool_choice` option for the inner Agent.
    // 为内部代理设置 `tool_choice` 选项
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        // 设置代理构建器的工具选择
        self.agent_builder = self.agent_builder.tool_choice(choice);
        // 返回 self 以支持链式调用
        self
    }

    /// Build the Extractor
    // 构建提取器
    pub fn build(self) -> Extractor<M, T> {
        Extractor {
            // 构建代理
            agent: self.agent_builder.build(),
            // 设置类型标记
            _t: PhantomData,
            // 设置重试次数，默认为 0
            retries: self.retries.unwrap_or(0),
        }
    }
}

// 派生 Deserialize 和 Serialize trait
#[derive(Deserialize, Serialize)]
// 提交工具结构体
struct SubmitTool<T>
where
    // T 必须实现 JsonSchema、Deserialize、Serialize、Send、Sync trait
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync,
{
    // 类型标记，用于在编译时跟踪类型 T
    _t: PhantomData<T>,
}

// 派生 Debug 和 thiserror::Error trait
#[derive(Debug, thiserror::Error)]
// 提交错误
#[error("SubmitError")]
struct SubmitError;

// 为 SubmitTool 实现 Tool trait
impl<T> Tool for SubmitTool<T>
where
    // T 必须实现 JsonSchema、Deserialize、Serialize、Send、Sync trait
    T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync,
{
    // 工具名称常量
    const NAME: &'static str = SUBMIT_TOOL_NAME;
    // 错误类型
    type Error = SubmitError;
    // 参数类型
    type Args = T;
    // 输出类型
    type Output = T;

    // 获取工具定义
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            // 工具名称
            name: Self::NAME.to_string(),
            // 工具描述
            description: "Submit the structured data you extracted from the provided text."
                .to_string(),
            // 工具参数（使用 T 的 JSON Schema）
            parameters: json!(schema_for!(T)),
        }
    }

    // 调用工具
    async fn call(&self, data: Self::Args) -> Result<Self::Output, Self::Error> {
        // 直接返回输入数据
        Ok(data)
    }
}
