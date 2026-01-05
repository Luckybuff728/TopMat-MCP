//! MCP数据存储中间件
//!
//! 负责记录MCP工具调用和会话信息，支持HTTP和SSE传输

use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, body::Body,
};
use bytes::Bytes;
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::Row;
use tracing::{info, error, debug, warn};

use crate::server::{
    database::DatabaseConnection,
    handlers::chat::ServerState,
    middleware::auth::AuthUser,
    models::ErrorResponse,
};

// 本地定义MCP相关数据结构，避免依赖models模块
/// 创建MCP会话请求
#[derive(Debug)]
pub struct CreateMcpSessionRequest {
    pub session_id: String,
    pub user_id: i64,
    pub transport_type: String,
    pub client_info: Option<Value>,
}

/// 创建MCP工具调用请求
#[derive(Debug)]
pub struct CreateMcpToolCallRequest {
    pub user_id: i64,
    pub session_id: Option<String>,
    pub tool_name: String,
    pub request_arguments: Option<String>,
    pub response_result: Option<String>,
    pub execution_time_ms: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub transport_type: String,
    pub endpoint: String,
}

/// MCP数据存储中间件
pub struct McpStorage;

/// MCP会话上下文
#[derive(Clone, Debug)]
pub struct McpSessionContext {
    pub session_id: String,
    pub user_id: i64,
    pub transport_type: String,
    pub endpoint: String,
}

impl McpStorage {
    /// MCP数据存储中间件入口
    pub async fn store_mcp_data(
        State(state): State<ServerState>,
        mut request: Request,
        next: Next,
    ) -> Result<Response, ErrorResponse> {
        // 只处理 MCP 相关端点
        let path = request.uri().path();
        let method = request.method().clone();

        if !path.starts_with("/mcp") && !path.starts_with("/sse") {
            return Ok(next.run(request).await);
        }

        debug!("McpStorage: 拦截MCP请求 {} {}", method, path);

        // 确定传输类型和端点
        let (transport_type, endpoint) = if path.starts_with("/mcp") {
            ("http".to_string(), "/mcp".to_string())
        } else {
            ("sse".to_string(), path.to_string())
        };

        // 尝试获取或创建会话上下文
        let session_context = if let Some(auth_user) = request.extensions().get::<AuthUser>() {
            // 生成会话ID（如果没有的话）
            let session_id = self::generate_session_id(&request, &(auth_user.user_id as i64));
            debug!("McpStorage: 使用会话ID: {}", session_id);

            // 创建MCP会话上下文
            let context = McpSessionContext {
                session_id: session_id.clone(),
                user_id: auth_user.user_id as i64,
                transport_type: transport_type.clone(),
                endpoint: endpoint.clone(),
            };

            // 记录会话信息
            if let Err(e) = Self::upsert_session(
                &state.database,
                &context,
                Some(extract_client_info(&request)),
            ).await {
                error!("Failed to upsert MCP session: {}", e);
            }

            Some(context)
        } else {
            None
        };

        // 如果有会话上下文，将其注入请求扩展
        if let Some(ref context) = session_context {
            request.extensions_mut().insert(context.clone());
        }

        // 对于POST请求，尝试解析并记录工具调用
        if method == Method::POST {
            let (parts, body) = request.into_parts();
            let bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    error!("Failed to read MCP request body: {}", e);
                    return Err(ErrorResponse {
                        error: "invalid_request".to_string(),
                        message: "无法读取请求体".to_string(),
                        details: Some(serde_json::json!({
                            "error": format!("{}", e)
                        })),
                        timestamp: chrono::Local::now(),
                    });
                }
            };

            // 尝试解析MCP请求
            if let Ok(mcp_request) = serde_json::from_slice::<Value>(&bytes) {
                debug!("McpStorage: 解析到MCP请求: {}", serde_json::to_string_pretty(&mcp_request).unwrap_or_default());

                // 重建请求并传递给下一个处理器
                let response = {
                    let mut new_request = Request::from_parts(parts, Body::from(bytes.clone()));
                    if let Some(ref context) = session_context {
                        new_request.extensions_mut().insert(context.clone());
                    }
                    next.run(new_request).await
                };

                // 如果是工具调用且有会话上下文，记录调用结果
                if let (Some(method_name), Some(ref context)) = (
                    mcp_request.get("method").and_then(|m| m.as_str()),
                    session_context
                ) {
                    if method_name == "tools/call" {
                        // 记录工具调用请求
                        Self::record_tool_call_request(
                            &state.database,
                            context,
                            &mcp_request,
                        ).await;

                        // 尝试从响应中提取结果并记录
                        return Self::handle_tool_call_response(
                            response,
                            &state.database,
                            context,
                            &mcp_request,
                        ).await;
                    }
                }

                return Ok(response);
            } else {
                // 重建请求并继续处理
                let new_request = Request::from_parts(parts, Body::from(bytes));
                return Ok(next.run(new_request).await);
            }
        }

        // 非POST请求直接处理
        Ok(next.run(request).await)
    }

    /// 创建或更新MCP会话
    async fn upsert_session(
        db: &DatabaseConnection,
        context: &McpSessionContext,
        client_info: Option<Value>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client_info_json = client_info.and_then(|v| serde_json::to_string(&v).ok());

        sqlx::query(
            r#"
            INSERT INTO mcp_sessions
            (session_id, user_id, transport_type, client_info, last_activity_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (session_id) DO UPDATE SET
                last_activity_at = NOW(),
                client_info = EXCLUDED.client_info
            "#
        )
        .bind(&context.session_id)
        .bind(context.user_id)
        .bind(&context.transport_type)
        .bind(client_info_json)
        .execute(db.pool())
        .await?;

        info!("McpStorage: 已更新MCP会话 {}", context.session_id);
        Ok(())
    }

    /// 记录工具调用请求，返回插入的记录 ID
    async fn record_tool_call_request(
        db: &DatabaseConnection,
        context: &McpSessionContext,
        mcp_request: &Value,
    ) -> Result<Option<i64>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(params) = mcp_request.get("params") {
            if let (Some(tool_name), Some(arguments)) = (
                params.get("name").and_then(|n| n.as_str()),
                params.get("arguments")
            ) {
                let arguments_json = serde_json::to_string(arguments).ok();

                let result = sqlx::query(
                    r#"
                    INSERT INTO mcp_tool_calls
                    (user_id, session_id, tool_name, request_arguments, status, transport_type, endpoint)
                    VALUES ($1, $2, $3, $4, 'pending', $5, $6)
                    RETURNING id
                    "#
                )
                .bind(context.user_id)
                .bind(&context.session_id)
                .bind(tool_name)
                .bind(arguments_json)
                .bind(&context.transport_type)
                .bind(&context.endpoint)
                .fetch_one(db.pool())
                .await?;

                let row_id: i64 = result.try_get("id").unwrap_or(0);
                info!("McpStorage: 已记录工具调用请求: {} (session: {}, id: {})", tool_name, context.session_id, row_id);
                return Ok(Some(row_id));
            }
        }
        Ok(None)
    }

    /// 处理工具调用响应并记录结果
    async fn handle_tool_call_response(
        mut response: Response,
        db: &DatabaseConnection,
        context: &McpSessionContext,
        original_request: &Value,
    ) -> Result<Response, ErrorResponse> {
        // 检查响应状态
        if !response.status().is_success() {
            // 记录失败的响应
            if let Some(tool_name) = original_request
                .get("params")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
            {
                Self::record_tool_call_result(
                    db,
                    context,
                    tool_name,
                    None,
                    None,
                    "error",
                    Some(&format!("HTTP错误: {}", response.status())),
                ).await;
            }
            return Ok(response);
        }

        // 提取响应体
        let (parts, body) = response.into_parts();
        let bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                error!("Failed to read MCP response body: {}", e);
                return Ok(Response::from_parts(parts, Body::from(Bytes::new())));
            }
        };

        // 尝试提取 SSE 格式中的 JSON 数据
        // SSE 格式: "data: {...json...}\n\nid: xxx"
        let json_bytes: bytes::Bytes = {
            let content = String::from_utf8_lossy(&bytes);
            if content.starts_with("data: ") {
                // 提取 "data: " 后面的 JSON 内容
                if let Some(json_start) = content.find("data: ") {
                    let json_part = &content[json_start + 6..];
                    // 找到第一个换行符作为 JSON 结束
                    if let Some(json_end) = json_part.find('\n') {
                        bytes::Bytes::from(json_part[..json_end].trim().to_string())
                    } else {
                        bytes::Bytes::from(json_part.trim().to_string())
                    }
                } else {
                    bytes.clone()
                }
            } else {
                bytes.clone()
            }
        };

        // 解析MCP响应
        match serde_json::from_slice::<Value>(&json_bytes) {
            Ok(mcp_response) => {

            // 检查是否是工具调用响应
            if let (Some(id), Some(result)) = (
                original_request.get("id"),
                mcp_response.get("result")
            ) {
                if let Some(tool_name) = original_request
                    .get("params")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                {
                    let result_json = serde_json::to_string(result).ok();

                    // 记录成功的工具调用结果
                    match Self::record_tool_call_result(
                        db,
                        context,
                        tool_name,
                        result_json,
                        None, // 执行时间需要在实际调用处测量
                        "success",
                        None,
                    ).await {
                        Ok(_) => {},
                        Err(e) => error!("McpStorage: 更新工具调用结果失败: {} - {}", tool_name, e),
                    }
                }
            }

            // 检查是否有错误响应
            if let Some(error) = mcp_response.get("error") {
                if let Some(tool_name) = original_request
                    .get("params")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                {
                    let error_message = error.get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("未知错误");

                    match Self::record_tool_call_result(
                        db,
                        context,
                        tool_name,
                        None,
                        None,
                        "error",
                        Some(error_message),
                    ).await {
                        Ok(_) => warn!("McpStorage: 工具调用失败: {} - {}", tool_name, error_message),
                        Err(e) => error!("McpStorage: 更新工具调用错误记录失败: {} - {}", tool_name, e),
                    }
                }
            }
            },
            Err(_) => {
                // SSE响应解析失败，静默处理
            }
        }

        // 重建响应
        Ok(Response::from_parts(parts, Body::from(bytes)))
    }

    /// 记录工具调用结果（通过 ID 更新指定记录）
    async fn record_tool_call_result(
        db: &DatabaseConnection,
        context: &McpSessionContext,
        tool_name: &str,
        result: Option<String>,
        execution_time_ms: Option<i32>,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        // 更新最近一条 pending 记录（通过 session_id + tool_name + status 匹配，按 id 降序取最新）
        let query_result = sqlx::query(
            r#"
            UPDATE mcp_tool_calls
            SET response_result = $1,
                execution_time_ms = $2,
                status = $3,
                error_message = $4
            WHERE id = (
                SELECT id FROM mcp_tool_calls 
                WHERE session_id = $5 AND tool_name = $6 AND status = 'pending'
                ORDER BY id DESC LIMIT 1
            )
            "#
        )
        .bind(result)
        .bind(execution_time_ms)
        .bind(status)
        .bind(error_message)
        .bind(&context.session_id)
        .bind(tool_name)
        .execute(db.pool())
        .await?;

        Ok(query_result.rows_affected())
    }

    /// 异步记录工具调用（用于MCP服务器内部调用）
    pub async fn record_tool_call_async(
        db: &DatabaseConnection,
        user_id: i64,
        session_id: Option<String>,
        tool_name: &str,
        arguments: &Value,
        result: &Value,
        execution_time_ms: i32,
        status: &str,
        error_message: Option<&str>,
        transport_type: &str,
        endpoint: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let arguments_json = serde_json::to_string(arguments).ok();
        let result_json = serde_json::to_string(result).ok();

        sqlx::query(
            r#"
            INSERT INTO mcp_tool_calls
            (user_id, session_id, tool_name, request_arguments, response_result,
             execution_time_ms, status, error_message, transport_type, endpoint)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(user_id)
        .bind(session_id)
        .bind(tool_name)
        .bind(arguments_json)
        .bind(result_json)
        .bind(execution_time_ms)
        .bind(status)
        .bind(error_message)
        .bind(transport_type)
        .bind(endpoint)
        .execute(db.pool())
        .await?;

        Ok(())
    }
}

/// 生成会话ID
fn generate_session_id(request: &Request, user_id: &i64) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // 1. 优先使用客户端提供的会话ID（Cherry Studio 发送全小写 header）
    if let Some(session_id) = request.headers().get("mcp-session-id")
        .and_then(|v| v.to_str().ok()) {
            info!("McpStorage: 使用客户端提供的会话ID: {}", session_id);
        return session_id.to_string();
    }

    // 2. 基于稳定标识符生成会话ID（不包含时间戳）
    let mut hasher = DefaultHasher::new();
    
    // 对用户ID进行哈希
    user_id.hash(&mut hasher);
    
    // 对路径进行哈希（区分 /mcp 和 /sse）
    request.uri().path().hash(&mut hasher);
    
    // 对用户代理进行哈希（同一客户端产生相同的ID）
    request.headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .hash(&mut hasher);

    // 生成稳定的会话ID（格式：mcp_用户ID_哈希值）
    format!("mcp_{}_{}", user_id, hasher.finish())
}

/// 提取客户端信息
fn extract_client_info(request: &Request) -> Value {
    let mut client_info = serde_json::Map::new();

    if let Some(user_agent) = request.headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok()) {
        client_info.insert("user_agent".to_string(), Value::String(user_agent.to_string()));
    }

    if let Some(forwarded_for) = request.headers()
        .get("x-forwarded-for")
        .or_else(|| request.headers().get("x-real-ip"))
        .and_then(|v| v.to_str().ok()) {
        client_info.insert("client_ip".to_string(), Value::String(forwarded_for.to_string()));
    }

    client_info.insert("request_path".to_string(), Value::String(request.uri().path().to_string()));
    client_info.insert("request_method".to_string(), Value::String(request.method().to_string()));

    Value::Object(client_info)
}