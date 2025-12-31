//! MCP 端点的虚拟文档 Handlers
//!
//! 这些 handlers 仅用于 OpenAPI 文档生成，不会实际处理 MCP 请求
//! MCP 请求由 RMCP 框架的 StreamableHttpService 处理

use axum::{
    extract::{Query, State, Extension},
    response::Json,
    http::StatusCode,
};
use utoipa::path;
use serde_json::json;

use crate::server::models::*;
use crate::server::middleware::auth::AuthUser;
use super::chat::ServerState;

/// 获取 MCP 服务器信息
///
/// 这个端点用于获取 MCP 服务器的基本信息和能力。
/// 实际的 MCP 协议通信通过 RMCP 框架处理。
#[utoipa::path(
    get,
    path = "/mcp",
    tag = "mcp",
    summary = "获取MCP服务器信息",
    description = "获取MCP服务器的基本信息、可用工具列表和能力声明。\n\n**注意**: 这个文档仅用于说明，实际的MCP协议通信使用不同的消息格式。",
    responses(
        (status = 200, description = "MCP服务器信息", body = McpInitializeResponse,
         example = json!({
             "protocolVersion": "2024-11-05",
             "capabilities": {
                 "tools": {
                     "listChanged": true
                 },
                 "logging": {}
             },
             "serverInfo": {
                 "name": "TopMat-LLM MCP Server",
                 "version": "0.1.0",
                 "protocol_version": "2024-11-05",
                 "title": "TopMat-LLM",
                 "website_url": "https://lab.topmaterial-tech.com/"
             }
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn mcp_info_handler(
    Extension(_auth_user): Extension<AuthUser>,
    State(_state): State<ServerState>,
) -> Result<Json<McpInitializeResponse>, ErrorResponse> {
    // 这是一个虚拟 handler，仅用于文档生成
    Err(ErrorResponse {
        error: "not_implemented".to_string(),
        message: "请使用 MCP 协议客户端访问 /mcp 端点".to_string(),
        details: Some(json!({
            "note": "这是一个文档端点，实际 MCP 通信使用不同的协议格式",
            "mcp_protocol": "Model Context Protocol",
            "transport": "HTTP + JSON-RPC 2.0"
        })),
        timestamp: chrono::Local::now(),
    })
}

/// 获取 MCP 工具列表
///
/// 列出 MCP 服务器支持的所有工具。
#[utoipa::path(
    get,
    path = "/mcp/tools",
    tag = "mcp",
    summary = "获取MCP工具列表",
    description = "获取MCP服务器支持的所有工具的列表和详细信息。",
    params(
        ("category" = Option<String>, Query, description = "按类别过滤工具", example = "simulation")
    ),
    responses(
        (status = 200, description = "工具列表", body = serde_json::Value,
         example = json!({
             "tools": [
                 {
                     "name": "calpha_mesh_simulation",
                     "description": "执行 CalphaMesh 材料科学模拟",
                     "input_schema": {
                         "type": "object",
                         "properties": {
                             "composition": {
                                 "type": "string",
                                 "description": "材料成分"
                             },
                             "temperature": {
                                 "type": "number",
                                 "description": "模拟温度 (K)"
                             }
                         },
                         "required": ["composition"]
                     },
                     "category": "simulation"
                 },
                 {
                     "name": "onnx_model_inference",
                     "description": "使用 ONNX 模型进行推理",
                     "input_schema": {
                         "type": "object",
                         "properties": {
                             "model_name": {
                                 "type": "string",
                                 "description": "模型名称"
                             },
                             "input_data": {
                                 "type": "array",
                                 "items": {"type": "number"},
                                 "description": "输入数据"
                             }
                         },
                         "required": ["model_name", "input_data"]
                     },
                     "category": "ml"
                 }
             ]
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn mcp_tools_list_handler(
    Extension(_auth_user): Extension<AuthUser>,
    State(_state): State<ServerState>,
    Query(_params): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    // 虚拟 handler，返回示例错误信息
    Err(ErrorResponse {
        error: "not_implemented".to_string(),
        message: "请使用 MCP 协议客户端访问工具列表".to_string(),
        details: Some(json!({
            "mcp_method": "tools/list",
            "usage": "通过 MCP 协议的 JSON-RPC 2.0 调用"
        })),
        timestamp: chrono::Local::now(),
    })
}

/// 调用 MCP 工具
///
/// 执行指定的 MCP 工具。
#[utoipa::path(
    post,
    path = "/mcp/tools/call",
    tag = "mcp",
    summary = "调用MCP工具",
    description = "执行指定的MCP工具并返回结果。\n\n**重要**: 这只是文档示例。实际的MCP工具调用需要通过MCP协议的JSON-RPC 2.0格式进行。",
    request_body(
        content = McpToolCallRequest,
        description = "工具调用请求",
        example = json!({
            "name": "calpha_mesh_simulation",
            "arguments": {
                "composition": "Fe-18Cr-12Ni",
                "temperature": 1273.15,
                "time": 3600
            }
        })
    ),
    responses(
        (status = 200, description = "工具执行结果", body = McpToolCallResponse,
         example = json!({
             "content": [
                 {
                     "type": "text",
                     "text": "CalphaMesh 模拟完成。相分数: Austrite 0.65, Ferrite 0.35。"
                 },
                 {
                     "type": "data",
                     "data": {
                         "simulation_id": "sim_123456",
                         "phase_fractions": {
                             "austenite": 0.65,
                             "ferrite": 0.35
                         },
                         "execution_time_ms": 1250
                     }
                 }
             ],
             "isError": false
         })),
        (status = 400, description = "工具参数错误", body = ErrorResponse),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn mcp_tool_call_handler(
    Extension(_auth_user): Extension<AuthUser>,
    State(_state): State<ServerState>,
    Json(_request): Json<McpToolCallRequest>,
) -> Result<Json<McpToolCallResponse>, ErrorResponse> {
    // 虚拟 handler
    Err(ErrorResponse {
        error: "not_implemented".to_string(),
        message: "请使用 MCP 协议客户端调用工具".to_string(),
        details: Some(json!({
            "mcp_method": "tools/call",
            "protocol": "JSON-RPC 2.0 over HTTP",
            "content_type": "application/json"
        })),
        timestamp: chrono::Local::now(),
    })
}

/// SSE MCP 连接信息
///
/// 通过 Server-Sent Events 建立 MCP 连接的文档说明。
#[utoipa::path(
    get,
    path = "/sse",
    tag = "mcp",
    summary = "SSE MCP连接信息",
    description = "通过Server-Sent Events协议建立MCP连接的文档说明。\n\n**注意**: 这只是连接信息文档，实际的SSE连接需要使用EventSource API。",
    responses(
        (status = 200, description = "SSE连接信息", body = serde_json::Value,
         example = json!({
             "message": "MCP SSE 服务端点",
             "protocol": "Server-Sent Events",
             "connection_url": "/sse",
             "usage": {
                 "javascript": "const eventSource = new EventSource('/sse');",
                 "notes": [
                     "使用 EventSource API 建立 SSE 连接",
                     "支持双向通信通过 HTTP POST 到 /sse/message",
                     "自动重连和心跳保活"
                 ]
             },
             "mcp_capabilities": {
                 "tools": true,
                 "logging": true,
                 "streaming": true
             }
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn sse_info_handler(
    Extension(_auth_user): Extension<AuthUser>,
    State(_state): State<ServerState>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    // 返回 SSE 连接信息
    Ok(Json(json!({
        "message": "MCP SSE 服务端点",
        "protocol": "Server-Sent Events",
        "connection_url": "/sse",
        "usage": {
            "javascript": "const eventSource = new EventSource('/sse');",
            "notes": [
                "使用 EventSource API 建立 SSE 连接",
                "支持双向通信通过 HTTP POST 到 /sse/message",
                "自动重连和心跳保活"
            ]
        },
        "mcp_capabilities": {
            "tools": true,
            "logging": true,
            "streaming": true
        }
    })))
}

/// SSE 消息发送
///
/// 通过 SSE 连接发送 MCP 消息的文档说明。
#[utoipa::path(
    post,
    path = "/sse/message",
    tag = "mcp",
    summary = "SSE消息发送",
    description = "向SSE连接发送MCP消息的文档说明。",
    request_body(
        content = serde_json::Value,
        description = "MCP消息 (JSON-RPC 2.0格式)",
        example = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "calpha_mesh_simulation",
                "arguments": {
                    "composition": "Fe-18Cr-12Ni"
                }
            }
        })
    ),
    responses(
        (status = 200, description = "消息发送确认", body = serde_json::Value,
         example = json!({
             "message": "消息已发送到SSE连接",
             "status": "sent",
             "protocol": "JSON-RPC 2.0"
         })),
        (status = 401, description = "未授权 - API Key 缺失或无效", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn sse_message_handler(
    Extension(_auth_user): Extension<AuthUser>,
    State(_state): State<ServerState>,
    Json(_message): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    // 虚拟 handler
    Ok(Json(json!({
        "message": "消息已发送到SSE连接",
        "status": "sent",
        "protocol": "JSON-RPC 2.0",
        "note": "实际使用时，消息会通过 SSE 连接异步返回响应"
    })))
}