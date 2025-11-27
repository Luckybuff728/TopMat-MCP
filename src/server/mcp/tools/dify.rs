//! Dify API 集成 MCP 工具
//!
//! 提供与 Dify API 集成的工具，支持钢铁知识库、硬质合金知识库查询以及 Al 合金正向设计-IDME

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error as StdError;
use rig::{
    completion::ToolDefinition,
    tool::Tool,
};

use reqwest;

// Dify API 基础 URL
const DIFY_API_URL: &str = "http://111.22.21.99:10003/v1/workflows/run";

// ==================== 错误类型 ====================

#[derive(Debug)]
pub enum DifyError {
    HttpError(String),
    ApiError { status: u16, message: String },
    JsonError(String),
    InvalidRequest(String),
    MissingParameter(String),
}

impl std::fmt::Display for DifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DifyError::HttpError(msg) => write!(f, "HTTP请求失败: {}", msg),
            DifyError::ApiError { status, message } => {
                write!(f, "API错误 (状态码 {}): {}", status, message)
            }
            DifyError::JsonError(msg) => write!(f, "JSON序列化/反序列化错误: {}", msg),
            DifyError::InvalidRequest(msg) => write!(f, "无效请求: {}", msg),
            DifyError::MissingParameter(param) => write!(f, "缺少必需参数: {}", param),
        }
    }
}

impl StdError for DifyError {}

// ==================== 请求/响应结构体 ====================

/// RAG 请求参数
#[derive(Debug, Deserialize)]
pub struct DifyQueryRequest {
    pub input: String,
    pub user: Option<String>,
}

/// Dify API 请求体
#[derive(Serialize)]
struct DifyRequest {
    inputs: HashMap<String, serde_json::Value>,
    response_mode: String,
    user: String,
}

/// Dify 流式响应结构
#[derive(Deserialize)]
struct DifyStreamResponse {
    event: String,
    #[serde(rename = "data")]
    response_data: Option<DifyStreamData>,
}

#[derive(Deserialize)]
struct DifyStreamData {
    text: Option<String>,
}

/// Dify 阻塞响应结构
#[derive(Deserialize)]
struct DifyResponse {
    data: Option<DifyResponseData>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct DifyResponseData {
    outputs: Option<serde_json::Value>,
}

// ==================== 工具实现 ====================

/// 钢铁知识库 RAG 查询工具
#[derive(Deserialize, Serialize)]
pub struct SteelRagQuery;

impl Default for SteelRagQuery {
    fn default() -> Self {
        Self
    }
}

impl Tool for SteelRagQuery {
    const NAME: &'static str = "steel_rag";
    type Error = DifyError;
    type Args = DifyQueryRequest;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "steel_rag".to_string(),
            description: "根据钢铁知识库检索相关信息并回答问题".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "要查询的问题或文本内容"
                    },
                    "user": {
                        "type": "string",
                        "description": "用户标识符（可选）",
                        "default": "default-user"
                    }
                },
                "required": ["input"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = "app-nzguJbMslrsTFV7HXefcilK6"; // 钢铁知识库 API Key
        call_dify_workflow(api_key, args.input, args.user.unwrap_or_else(|| "default-user".to_string())).await
    }
}

/// 硬质合金知识库 RAG 查询工具
#[derive(Deserialize, Serialize)]
pub struct CementedCarbideRagQuery;

impl Default for CementedCarbideRagQuery {
    fn default() -> Self {
        Self
    }
}

impl Tool for CementedCarbideRagQuery {
    const NAME: &'static str = "cemented_carbide_rag";
    type Error = DifyError;
    type Args = DifyQueryRequest;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "cemented_carbide_rag".to_string(),
            description: "根据硬质合金知识库检索相关信息并回答问题".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "要查询的问题或文本内容"
                    },
                    "user": {
                        "type": "string",
                        "description": "用户标识符（可选）",
                        "default": "default-user"
                    }
                },
                "required": ["input"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = "app-c4PnkeaIM2pWLBzVelQtt0oy"; // 硬质合金知识库 API Key
        call_dify_workflow(api_key, args.input, args.user.unwrap_or_else(|| "default-user".to_string())).await
    }
}

/// Al 合金正向设计-IDME 工具
#[derive(Deserialize, Serialize)]
pub struct AlIdmeWorkflow;

impl Default for AlIdmeWorkflow {
    fn default() -> Self {
        Self
    }
}

impl Tool for AlIdmeWorkflow {
    const NAME: &'static str = "Al_idme_workflow";
    type Error = DifyError;
    type Args = DifyQueryRequest;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "Al_idme_workflow".to_string(),
            description: "Al合金正向设计-IDME: 提供铝合金的成分和工艺，用于Al合金的组织结构以及性能预测".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "输入文本内容（铝合金成分和工艺要求）"
                    },
                    "user": {
                        "type": "string",
                        "description": "用户标识符（可选）",
                        "default": "default-user"
                    }
                },
                "required": ["input"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = "app-Vjb2R86PqSCNR6S5w1TUc7a9"; // Al_IDME API Key
        call_dify_workflow_demand(api_key, args.input, args.user.unwrap_or_else(|| "default-user".to_string())).await
    }
}

// ==================== 核心函数 ====================

/// 调用 Dify Workflow API 的通用函数
async fn call_dify_workflow(
    api_key: &str,
    input_text: String,
    user: String,
) -> Result<String, DifyError> {
    let client = reqwest::Client::new();
    let response_mode = "streaming";

    // 构建请求体
    let mut inputs = HashMap::new();
    inputs.insert("question".to_string(), json!(input_text));

    let request_body = DifyRequest {
        inputs,
        response_mode: response_mode.to_string(),
        user,
    };

    let request_body_str = serde_json::to_string(&request_body)
        .map_err(|e| DifyError::JsonError(format!("Failed to serialize request: {}", e)))?;

    // 发送 HTTP 请求
    let response = client
        .post(DIFY_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(request_body_str)
        .send()
        .await
        .map_err(|e| DifyError::HttpError(e.to_string()))?;

    // 检查响应状态
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(DifyError::ApiError {
            status: status.as_u16(),
            message: error_text,
        });
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| DifyError::HttpError(e.to_string()))?;

    // 处理流式响应
    handle_streaming_response(&response_text)
}

/// 调用 Dify Workflow API 的函数（用于 AL_IDME，使用 demand 字段）
async fn call_dify_workflow_demand(
    api_key: &str,
    input_text: String,
    user: String,
) -> Result<String, DifyError> {
    let client = reqwest::Client::new();
    let response_mode = "streaming";

    // 构建请求体（使用 demand 字段）
    let mut inputs = HashMap::new();
    inputs.insert("demand".to_string(), json!(input_text));

    let request_body = DifyRequest {
        inputs,
        response_mode: response_mode.to_string(),
        user,
    };

    let request_body_str = serde_json::to_string(&request_body)
        .map_err(|e| DifyError::JsonError(format!("Failed to serialize request: {}", e)))?;

    // 发送 HTTP 请求
    let response = client
        .post(DIFY_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(request_body_str)
        .send()
        .await
        .map_err(|e| DifyError::HttpError(e.to_string()))?;

    // 检查响应状态
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(DifyError::ApiError {
            status: status.as_u16(),
            message: error_text,
        });
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| DifyError::HttpError(e.to_string()))?;

    // 处理流式响应
    handle_streaming_response(&response_text)
}

/// 处理 Dify 流式响应
fn handle_streaming_response(response: &str) -> Result<String, DifyError> {
    let lines: Vec<&str> = response.lines().collect();
    let mut text_chunks = Vec::new();

    for line in lines {
        if line.starts_with("data: ") {
            let data = &line[6..];
            if data == "[DONE]" {
                break;
            }

            // 解析 Dify 流式响应格式
            if let Ok(json_data) = serde_json::from_str::<DifyStreamResponse>(data) {
                match json_data.event.as_str() {
                    "text_chunk" => {
                        if let Some(response_data) = json_data.response_data {
                            if let Some(text) = response_data.text {
                                text_chunks.push(text);
                            }
                        }
                    }
                    "workflow_finished" => {
                        // 工作流完成
                        break;
                    }
                    _ => {
                        // 忽略其他事件
                    }
                }
            } else {
                // 如果解析失败，继续处理下一行
                continue;
            }
        }
    }

    let combined_text = text_chunks.join("");

    if combined_text.is_empty() {
        return Err(DifyError::InvalidRequest("No response content received".to_string()));
    }

    Ok(combined_text)
}

/// 处理 Dify 阻塞响应（备用函数）
fn handle_blocking_response(response: &str) -> Result<String, DifyError> {
    if let Ok(json_response) = serde_json::from_str::<serde_json::Value>(response) {
        // 检查是否有错误
        if let Some(_error) = json_response.get("error") {
            let error_msg = json_response
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error");
            return Err(DifyError::ApiError {
                status: 400,
                message: error_msg.to_string(),
            });
        }

        // 提取输出内容
        let output_text = json_response
            .get("data")
            .and_then(|d| d.get("outputs"))
            .and_then(|o| o.as_object())
            .and_then(|obj| {
                // 尝试获取常见的输出字段
                obj.get("text")
                    .or_else(|| obj.get("output"))
                    .or_else(|| obj.get("result"))
                    .or_else(|| obj.get("answer"))
            })
            .and_then(|v| v.as_str())
            .unwrap_or("No output found");

        Ok(output_text.to_string())
    } else {
        // 如果 JSON 解析失败，返回原始响应
        Err(DifyError::JsonError(format!("Failed to parse response: {}", response)))
    }
}

