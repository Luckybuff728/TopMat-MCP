use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 聊天请求结构
#[derive(Debug, Deserialize, Clone)]
pub struct ChatRequest {
    /// 用户输入的消息
    pub message: String,
    /// 是否使用流式响应
    #[serde(default)]
    pub stream: bool,
    /// 使用的模型名称
    #[serde(default = "default_model")]
    pub model: String,
    /// 系统提示词
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// 温度参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 最大token数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 会话ID（用于多轮对话）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// 额外的元数据
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_model() -> String {
    "qwen3:4b".to_string()
}

/// 聊天响应结构（非流式）
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    /// 响应内容
    pub content: String,
    /// 使用的模型
    pub model: String,
    /// Token使用情况
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    /// 会话ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// 响应时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 额外的元数据
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Token使用情况
#[derive(Debug, Serialize)]
pub struct TokenUsage {
    /// 提示词token数
    pub prompt_tokens: u32,
    /// 完成token数
    pub completion_tokens: u32,
    /// 总token数
    pub total_tokens: u32,
}

/// 流式响应的数据块
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum StreamChunk {
    /// 文本内容块
    #[serde(rename = "content")]
    Text {
        /// 文本内容
        text: String,
        /// 是否为最后一个块
        #[serde(default)]
        finished: bool,
    },
    /// 推理过程
    #[serde(rename = "reasoning")]
    Reasoning {
        /// 推理内容
        reasoning: String,
    },
    /// 工具调用
    #[serde(rename = "tool_call")]
    ToolCall {
        /// 工具名称
        name: String,
        /// 工具参数
        arguments: serde_json::Value,
    },
    /// 错误信息
    #[serde(rename = "error")]
    Error {
        /// 错误消息
        message: String,
    },
    /// 最终响应信息
    #[serde(rename = "final")]
    Final {
        /// 完整的响应信息
        response: ChatResponse,
    },
}

/// API错误响应
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// 错误类型
    pub error: String,
    /// 错误消息
    pub message: String,
    /// 错误详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}