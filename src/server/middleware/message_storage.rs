use axum::{
    Extension,
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use bytes::Bytes;
use futures_util::TryStreamExt;
use http_body_util::BodyExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::server::{
    database::DatabaseConnection,
    handlers::chat::ServerState,
    middleware::auth::AuthUser,
    models::{
        ChatRequest, ChatResponse, StreamChunk, ToolCallInfo, ToolFunctionCall,
        generate_conversation_id,
    },
};
use rig::OneOrMany;
use rig::message::{AssistantContent, Message as RigMessage, ToolResultContent, UserContent};

/// 消息存储中间件
/// 负责拦截和保存聊天请求和响应
pub struct MessageStorage;

/// 用于在请求处理过程中传递的消息上下文
#[derive(Clone, Debug)]
pub struct MessageContext {
    pub request_body: ChatRequest,
    pub conversation_id: String,
    pub user_id: i64,
    pub should_save: bool,
}

impl MessageStorage {
    /// 创建消息存储中间件
    pub async fn store_messages(
        State(state): State<ServerState>,
        Extension(auth_user): Extension<AuthUser>,
        request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        // 只处理 /v1/chat 端点
        let path = request.uri().path();
        if !path.contains("/v1/chat") {
            return Ok(next.run(request).await);
        }

        debug!("MessageStorage: 拦截聊天请求");

        // 提取并解析请求体
        let (parts, body) = request.into_parts();
        let bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                error!("Failed to read request body: {}", e);
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        // 解析 ChatRequest
        let mut chat_request: ChatRequest = match serde_json::from_slice(&bytes) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse chat request: {}", e);
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        // 确保对话ID存在（如果没有则生成新的）
        let conversation_id = chat_request
            .conversation_id
            .clone()
            .unwrap_or_else(generate_conversation_id);

        // 将确定的 conversation_id 设置回请求中，确保一致性
        chat_request.conversation_id = Some(conversation_id.clone());

        info!("MessageStorage: 处理对话 {} 的请求", conversation_id);

        // 保存用户消息
        let db = state.database.clone();
        let user_id = auth_user.user_id as i64;

        if let Err(e) = save_user_message(&db, &chat_request, user_id, &conversation_id).await {
            error!("Failed to save user message: {}", e);
            // 继续处理请求，即使保存失败
        }

        // 创建消息上下文
        let message_context = MessageContext {
            request_body: chat_request.clone(),
            conversation_id: conversation_id.clone(),
            user_id,
            should_save: true,
        };

        // 重建请求，使用更新后的 chat_request（包含确定的 conversation_id）
        let updated_bytes = serde_json::to_vec(&chat_request).unwrap_or(bytes.to_vec());
        let mut new_request = Request::from_parts(parts, Body::from(updated_bytes));
        new_request.extensions_mut().insert(message_context.clone());

        // 调用下一个处理器
        let response = next.run(new_request).await;

        // 处理响应
        if chat_request.stream {
            // 流式响应：包装响应流以捕获内容
            handle_streaming_response(response, message_context, db).await
        } else {
            // 非流式响应：直接解析响应体
            handle_normal_response(response, message_context, db).await
        }
    }
}

/// 处理非流式响应
async fn handle_normal_response(
    response: Response,
    context: MessageContext,
    db: DatabaseConnection,
) -> Result<Response, StatusCode> {
    // 检查响应状态
    if !response.status().is_success() {
        return Ok(response);
    }

    // 提取响应体
    let (parts, body) = response.into_parts();
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!("Failed to read response body: {}", e);
            return Ok(Response::from_parts(parts, Body::from(Bytes::new())));
        }
    };

    // 尝试解析为 ChatResponse
    if let Ok(mut chat_response) = serde_json::from_slice::<ChatResponse>(&bytes) {
        // 1. 处理中间消息 (工具调用和结果)
        if let Some(intermediate_val) = chat_response.metadata.remove("_intermediate_messages") {
            if let Ok(intermediate_msgs) =
                serde_json::from_value::<Vec<RigMessage>>(intermediate_val)
            {
                // 跳过第一个消息，因为它是用户原始 prompt，已经在中间件开始时保存过了
                for msg in intermediate_msgs.into_iter().skip(1) {
                    match msg {
                        RigMessage::Assistant { content, .. } => {
                            let mut tool_calls = Vec::new();
                            for item in content.iter() {
                                if let AssistantContent::ToolCall(tc) = item {
                                    tool_calls.push(ToolCallInfo {
                                        id: tc.id.clone(),
                                        call_type: "function".to_string(),
                                        function: ToolFunctionCall {
                                            name: tc.function.name.clone(),
                                            arguments: tc.function.arguments.to_string(),
                                        },
                                    });
                                }
                            }

                            if !tool_calls.is_empty() {
                                let temp_assistant_resp = ChatResponse {
                                    content: None,
                                    reasoning_content: None,
                                    tool_calls: Some(tool_calls),
                                    model: chat_response.model.clone(),
                                    usage: None,
                                    conversation_id: context.conversation_id.clone(),
                                    timestamp: chrono::Local::now(),
                                    metadata: HashMap::new(),
                                };
                                if let Err(e) = save_assistant_message(
                                    &db,
                                    &temp_assistant_resp,
                                    context.user_id,
                                    &context.conversation_id,
                                )
                                .await
                                {
                                    error!("Failed to save intermediate assistant message: {}", e);
                                }
                            }
                        }
                        RigMessage::User { content } => {
                            for item in content.iter() {
                                if let UserContent::ToolResult(tr) = item {
                                    let result_str = tr
                                        .content
                                        .iter()
                                        .filter_map(|tc| {
                                            if let ToolResultContent::Text(t) = tc {
                                                Some(t.text.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n");

                                    if let Err(e) = save_tool_message(
                                        &db,
                                        &tr.id,
                                        &result_str,
                                        context.user_id,
                                        &context.conversation_id,
                                    )
                                    .await
                                    {
                                        error!("Failed to save intermediate tool message: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. 保存最终助手消息 (检查content是否存在且不为空)
        if chat_response
            .content
            .as_ref()
            .is_some_and(|s| !s.is_empty())
        {
            if let Err(e) = save_assistant_message(
                &db,
                &chat_response,
                context.user_id,
                &context.conversation_id,
            )
            .await
            {
                error!("Failed to save assistant message: {}", e);
            } else {
                info!(
                    "MessageStorage: 已保存最终助手消息到对话 {}",
                    context.conversation_id
                );
            }
        }

        // 更新返回给客户端的 bytes (已移除 _intermediate_messages)
        if let Ok(updated_bytes) = serde_json::to_vec(&chat_response) {
            return Ok(Response::from_parts(parts, Body::from(updated_bytes)));
        }
    }

    // 重建响应
    Ok(Response::from_parts(parts, Body::from(bytes)))
}

/// 处理流式响应
async fn handle_streaming_response(
    response: Response,
    context: MessageContext,
    db: DatabaseConnection,
) -> Result<Response, StatusCode> {
    // 检查响应状态
    if !response.status().is_success() {
        return Ok(response);
    }

    // 检查是否是 SSE 响应
    let is_sse = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/event-stream"))
        .unwrap_or(false);

    if !is_sse {
        return Ok(response);
    }

    debug!("MessageStorage: 处理流式响应");

    // 创建一个新的流来包装原始响应流
    let (parts, body) = response.into_parts();

    // 使用共享状态来收集流式内容
    let collected_content = Arc::new(Mutex::new(String::new()));
    let collected_reasoning = Arc::new(Mutex::new(String::new())); // 改为 String，累积推理内容
    let collected_tool_calls = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let final_response = Arc::new(Mutex::new(None));

    let db_clone = db.clone();
    let conversation_id = context.conversation_id.clone();
    let user_id = context.user_id;
    let model = context.request_body.model.clone();

    // 创建包装流
    let wrapped_stream = async_stream::stream! {
        let mut stream = body.into_data_stream();

        while let Some(result) = stream.try_next().await.transpose() {
            match result {
                Ok(chunk) => {
                    // 解析 SSE 数据
                    if let Ok(text) = std::str::from_utf8(&chunk) {
                        // 处理 SSE 事件
                        for line in text.lines() {
                            if let Some(data) = line.strip_prefix("data: ")
                                && let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) {
                                    match &parsed {
                                        StreamChunk::Text { text, .. } => {
                                            let mut content = collected_content.lock().await;
                                            content.push_str(text);
                                        }
                                        StreamChunk::Reasoning { reasoning } => {
                                            let mut reasons = collected_reasoning.lock().await;
                                            reasons.push_str(reasoning);
                                        }
                                        StreamChunk::ToolCall { id, name, arguments } => {
                                            let mut tool_calls = collected_tool_calls.lock().await;
                                            tool_calls.push(serde_json::json!({
                                                "id": id,
                                                "name": name,
                                                "arguments": arguments
                                            }));
                                        }
                                        StreamChunk::ToolResult { id, result } => {
                                            // 1. 如果有待保存的助手内容，先“冲刷”保存当前的助手消息片段
                                            let content = {
                                                let mut c = collected_content.lock().await;
                                                let val = c.clone();
                                                c.clear();
                                                val
                                            };
                                            let reasoning = {
                                                let mut r = collected_reasoning.lock().await;
                                                let val = r.clone();
                                                r.clear();
                                                val
                                            };
                                            let tool_calls_list = {
                                                let mut tc = collected_tool_calls.lock().await;
                                                let val = tc.clone();
                                                tc.clear();
                                                val
                                            };

                                            if !content.is_empty() || !reasoning.is_empty() || !tool_calls_list.is_empty() {
                                                let tool_call_info = if !tool_calls_list.is_empty() {
                                                    Some(tool_calls_list.iter().filter_map(|tc| {
                                                        let tc_id = tc.get("id")?.as_str()?.to_string();
                                                        let name = tc.get("name")?.as_str()?;
                                                        let args = tc.get("arguments")?.to_string();
                                                        Some(crate::server::models::ToolCallInfo {
                                                            id: tc_id,
                                                            call_type: "function".to_string(),
                                                            function: crate::server::models::ToolFunctionCall {
                                                                name: name.to_string(),
                                                                arguments: args,
                                                            },
                                                        })
                                                    }).collect())
                                                } else {
                                                    None
                                                };

                                                let partial_resp = ChatResponse {
                                                    content: if content.is_empty() { None } else { Some(content) },
                                                    reasoning_content: if reasoning.is_empty() { None } else { Some(reasoning) },
                                                    tool_calls: tool_call_info,
                                                    model: model.clone(),
                                                    usage: None,
                                                    conversation_id: conversation_id.clone(),
                                                    timestamp: chrono::Local::now(),
                                                    metadata: std::collections::HashMap::new(),
                                                };

                                                if let Err(e) = save_assistant_message(&db_clone, &partial_resp, user_id, &conversation_id).await {
                                                    error!("Failed to save incremental assistant message: {}", e);
                                                }
                                            }

                                            // 2. 保存该工具结果为一个独立消息 (role: tool)
                                            if let Err(e) = save_tool_message(
                                                &db_clone,
                                                id,
                                                &result.to_string(),
                                                user_id,
                                                &conversation_id,
                                            ).await {
                                                error!("Failed to save tool result message (ID: {}): {}", id, e);
                                            } else {
                                                debug!("MessageStorage: 已保存工具结果消息 (ID: {})", id);
                                            }
                                        }
                                        StreamChunk::Final { response } => {
                                            let mut final_resp = final_response.lock().await;
                                            *final_resp = Some(response.clone());
                                        }
                                        _ => {}
                                    }
                                }
                        }
                    }

                    // 转发原始块
                    yield Ok(chunk);
                }
                Err(e) => {
                    error!("Stream error: {}", e);
                    yield Err(e);
                    break;
                }
            }
        }

        // 流结束后，保存剩余的助手内容（如果有）
        let content = collected_content.lock().await.clone();
        let reasoning = collected_reasoning.lock().await.clone();
        let tool_calls = collected_tool_calls.lock().await.clone();

        if !content.is_empty() || !reasoning.is_empty() || !tool_calls.is_empty() {
            let chat_response = if let Some(final_resp) = final_response.lock().await.as_ref() {
                let mut resp = final_resp.clone();
                if !reasoning.is_empty() {
                    resp.reasoning_content = Some(reasoning.clone());
                }
                if !tool_calls.is_empty() {
                    resp.tool_calls = Some(tool_calls.iter().filter_map(|tc| {
                        let id = tc.get("id")?.as_str()?.to_string();
                        let name = tc.get("name")?.as_str()?;
                        let args = tc.get("arguments")?.to_string();
                        Some(crate::server::models::ToolCallInfo {
                            id,
                            call_type: "function".to_string(),
                            function: crate::server::models::ToolFunctionCall {
                                name: name.to_string(),
                                arguments: args,
                            },
                        })
                    }).collect());
                }
                resp
            } else {
                let tool_call_info = if !tool_calls.is_empty() {
                    Some(tool_calls.iter().filter_map(|tc| {
                        let id = tc.get("id")?.as_str()?.to_string();
                        let name = tc.get("name")?.as_str()?;
                        let args = tc.get("arguments")?.to_string();
                        Some(crate::server::models::ToolCallInfo {
                            id,
                            call_type: "function".to_string(),
                            function: crate::server::models::ToolFunctionCall {
                                name: name.to_string(),
                                arguments: args,
                            },
                        })
                    }).collect())
                } else {
                    None
                };

                ChatResponse {
                    content: if content.is_empty() { None } else { Some(content.clone()) },
                    reasoning_content: if reasoning.is_empty() { None } else { Some(reasoning.clone()) },
                    tool_calls: tool_call_info,
                    model,
                    usage: None,
                    conversation_id: conversation_id.clone(),
                    timestamp: chrono::Local::now(),
                    metadata: std::collections::HashMap::new(),
                }
            };

            if let Err(e) = save_assistant_message(&db_clone, &chat_response, user_id, &conversation_id).await {
                error!("Failed to save final assistant segment: {}", e);
            } else {
                info!("MessageStorage: 已保存最终助手内容到对话 {}", conversation_id);
            }
        }
    };

    // 转换为 Body
    let new_body = Body::from_stream(wrapped_stream);
    Ok(Response::from_parts(parts, new_body))
}

/// 保存用户消息
async fn save_user_message(
    db: &DatabaseConnection,
    request: &ChatRequest,
    user_id: i64,
    conversation_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 确保对话存在
    ensure_conversation_exists(
        db,
        conversation_id,
        user_id,
        &request.message,
        &request.model,
    )
    .await?;

    // 保存用户消息
    info!("MessageStorage: 保存用户消息: {}", &request.message);
    sqlx::query(
        r#"
        INSERT INTO messages (conversation_id, role, content, model, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        "#,
    )
    .bind(conversation_id)
    .bind("user")
    .bind(&request.message)
    .bind(&request.model)
    .execute(db.pool())
    .await?;

    info!("MessageStorage: 已保存用户消息到对话 {}", conversation_id);
    Ok(())
}

/// 保存助手消息
async fn save_assistant_message(
    db: &DatabaseConnection,
    chat_response: &ChatResponse,
    user_id: i64,
    conversation_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 验证对话权限
    let conversation_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM conversations WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_one(db.pool())
    .await?;

    if !conversation_exists {
        return Err(format!(
            "Conversation {} not found or access denied for user {}",
            conversation_id, user_id
        )
        .into());
    }

    // 保存助手消息
    let (prompt_tokens, completion_tokens, total_tokens) = chat_response
        .usage
        .as_ref()
        .map(|u| {
            (
                u.prompt_tokens as i32,
                u.completion_tokens as i32,
                u.total_tokens as i32,
            )
        })
        .unwrap_or((0, 0, 0));

    // 将 metadata 序列化为 JSON 字符串
    let metadata_json = if !chat_response.metadata.is_empty() {
        serde_json::to_string(&chat_response.metadata).ok()
    } else {
        None
    };

    // 将 tool_calls 序列化为 JSON 字符串
    let tool_calls_json = chat_response
        .tool_calls
        .as_ref()
        .map(|tc| serde_json::to_string(tc).unwrap_or_default());

    sqlx::query(
        r#"
        INSERT INTO messages (conversation_id, role, content, reasoning_content, tool_calls, model, prompt_tokens, completion_tokens, total_tokens, metadata, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
        "#
    )
    .bind(conversation_id)
    .bind("assistant")
    .bind(&chat_response.content)
    .bind(&chat_response.reasoning_content)
    .bind(tool_calls_json)
    .bind(&chat_response.model)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total_tokens)
    .bind(metadata_json)
    .execute(db.pool())
    .await?;

    // 更新对话的最后更新时间和消息计数
    sqlx::query(
        "UPDATE conversations SET updated_at = NOW(), message_count = message_count + 2 WHERE conversation_id = $1 AND user_id = $2"
    )
    .bind(conversation_id)
    .bind(user_id)
    .execute(db.pool())
    .await?;

    Ok(())
}

/// 保存工具结果消息
async fn save_tool_message(
    db: &DatabaseConnection,
    tool_call_id: &str,
    content: &str,
    user_id: i64,
    conversation_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 验证对话权限
    let conversation_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM conversations WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_one(db.pool())
    .await?;

    if !conversation_exists {
        return Err(format!(
            "Conversation {} not found or access denied for user {}",
            conversation_id, user_id
        )
        .into());
    }

    // 保存工具消息
    sqlx::query(
        r#"
        INSERT INTO messages (conversation_id, role, content, tool_call_id, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        "#,
    )
    .bind(conversation_id)
    .bind("tool")
    .bind(content)
    .bind(tool_call_id)
    .execute(db.pool())
    .await?;

    // 更新对话的最后更新时间和消息计数
    sqlx::query(
        "UPDATE conversations SET updated_at = NOW(), message_count = message_count + 1 WHERE conversation_id = $1 AND user_id = $2"
    )
    .bind(conversation_id)
    .bind(user_id)
    .execute(db.pool())
    .await?;

    Ok(())
}

/// 确保对话存在
async fn ensure_conversation_exists(
    db: &DatabaseConnection,
    conversation_id: &str,
    user_id: i64,
    message: &str,
    model: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 检查对话是否存在
    let conversation_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM conversations WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_one(db.pool())
    .await?;

    if conversation_exists {
        // 更新现有对话
        sqlx::query(
            "UPDATE conversations SET updated_at = NOW(), message_count = message_count + 1 WHERE conversation_id = $1 AND user_id = $2"
        )
        .bind(conversation_id)
        .bind(user_id)
        .execute(db.pool())
        .await?;
    } else {
        // 先确保用户存在（避免外键约束违规）
        let user_exists: bool = sqlx::query_scalar("SELECT COUNT(*) > 0 FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(db.pool())
            .await?;

        if !user_exists {
            // 创建临时用户记录（将在认证缓存时被更新）
            info!("MessageStorage: 用户 {} 不存在，创建临时用户记录", user_id);
            sqlx::query(
                r#"
                INSERT INTO users (id, username, email, subscription_level, created_at, updated_at)
                VALUES ($1, $2, $3, 'free', NOW(), NOW())
                ON CONFLICT (id) DO NOTHING
                "#,
            )
            .bind(user_id)
            .bind(format!("user_{}", user_id))
            .bind(format!("user_{}@temp.local", user_id))
            .execute(db.pool())
            .await?;
        }

        // 创建新对话
        let title = if message.chars().count() > 50 {
            message.chars().take(50).collect::<String>() + "..."
        } else {
            message.to_string()
        }
        .replace('\n', " ");

        sqlx::query(
            r#"
            INSERT INTO conversations (conversation_id, user_id, title, model, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            "#
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(title)
        .bind(model)
        .execute(db.pool())
        .await?;
    }

    Ok(())
}
