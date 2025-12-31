use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, body::Body,
};
use bytes::Bytes;
use futures_util::TryStreamExt;
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, debug};

use crate::server::{
    database::DatabaseConnection,
    handlers::chat::ServerState,
    middleware::auth::AuthUser,
    models::{ChatRequest, ChatResponse, StreamChunk, generate_conversation_id},
};

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
        mut request: Request,
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
        let conversation_id = chat_request.conversation_id
            .clone()
            .unwrap_or_else(generate_conversation_id);
        
        // 将确定的 conversation_id 设置回请求中，确保一致性
        chat_request.conversation_id = Some(conversation_id.clone());

        info!("MessageStorage: 处理对话 {} 的请求", conversation_id);

        // 保存用户消息
        let db = state.database.clone();
        let user_id = auth_user.user_id as i64;
        
        if let Err(e) = save_user_message(
            &db,
            &chat_request,
            user_id,
            &conversation_id,
        ).await {
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
    if let Ok(chat_response) = serde_json::from_slice::<ChatResponse>(&bytes) {
        // 保存助手消息
        if !chat_response.content.is_empty() {
            if let Err(e) = save_assistant_message(
                &db,
                &chat_response,
                context.user_id,
                &context.conversation_id,
            ).await {
                error!("Failed to save assistant message: {}", e);
            } else {
                info!("MessageStorage: 已保存助手消息到对话 {}", context.conversation_id);
            }
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
    let is_sse = response.headers()
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
    let collected_tool_results = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
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
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) {
                                    match &parsed {
                                        StreamChunk::Text { text, .. } => {
                                            let mut content = collected_content.lock().await;
                                            content.push_str(text);
                                        }
                                        StreamChunk::Reasoning { reasoning } => {
                                            let mut reasons = collected_reasoning.lock().await;
                                            reasons.push_str(reasoning);
                                        }
                                        StreamChunk::ToolCall { name, arguments } => {
                                            let mut tool_calls = collected_tool_calls.lock().await;
                                            tool_calls.push(serde_json::json!({
                                                "name": name,
                                                "arguments": arguments
                                            }));
                                        }
                                        StreamChunk::ToolResult { id, result } => {
                                            let mut tool_results = collected_tool_results.lock().await;
                                            tool_results.push(serde_json::json!({
                                                "id": id,
                                                "result": result
                                            }));
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
        
        // 流结束后保存助手消息（包含所有流式内容）
        let content = collected_content.lock().await.clone();
        let reasoning = collected_reasoning.lock().await.clone();
        let tool_calls = collected_tool_calls.lock().await.clone();
        let tool_results = collected_tool_results.lock().await.clone();
        
        if !content.is_empty() || !reasoning.is_empty() || !tool_calls.is_empty() {
            // 构建完整的元数据
            let mut metadata = std::collections::HashMap::new();
            
            // 将推理内容作为单个字符串保存
            if !reasoning.is_empty() {
                metadata.insert("reasoning".to_string(), serde_json::json!(reasoning));
            }
            if !tool_calls.is_empty() {
                metadata.insert("tool_calls".to_string(), serde_json::json!(tool_calls));
            }
            if !tool_results.is_empty() {
                metadata.insert("tool_results".to_string(), serde_json::json!(tool_results));
            }
            
            let chat_response = if let Some(final_resp) = final_response.lock().await.as_ref() {
                // 使用最终响应，但添加收集的元数据
                let mut resp = final_resp.clone();
                resp.metadata.extend(metadata);
                resp
            } else {
                // 构造一个基本的 ChatResponse
                ChatResponse {
                    content: content.clone(),
                    model,
                    usage: None,
                    conversation_id: conversation_id.clone(),
                    timestamp: chrono::Local::now(),
                    metadata,
                }
            };
            
            if let Err(e) = save_assistant_message(
                &db_clone,
                &chat_response,
                user_id,
                &conversation_id,
            ).await {
                error!("Failed to save streaming assistant message: {}", e);
            } else {
                info!("MessageStorage: 已保存完整的流式助手消息到对话 {} (包含推理、工具调用等)", conversation_id);
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
    ensure_conversation_exists(db, conversation_id, user_id, &request.message, &request.model).await?;

    // 保存用户消息
    info!("MessageStorage: 保存用户消息: {}", &request.message);
    sqlx::query(
        r#"
        INSERT INTO messages (conversation_id, role, content, model, created_at)
        VALUES (?1, ?2, ?3, ?4, datetime('now'))
        "#
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
        "SELECT COUNT(*) > 0 FROM conversations WHERE conversation_id = ? AND user_id = ?"
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_one(db.pool())
    .await?;

    if !conversation_exists {
        return Err(format!("Conversation {} not found or access denied for user {}", conversation_id, user_id).into());
    }

    // 保存助手消息
    let (prompt_tokens, completion_tokens, total_tokens) = chat_response.usage
        .as_ref()
        .map(|u| (u.prompt_tokens as i32, u.completion_tokens as i32, u.total_tokens as i32))
        .unwrap_or((0, 0, 0));

    // 将 metadata 序列化为 JSON 字符串
    let metadata_json = if !chat_response.metadata.is_empty() {
        serde_json::to_string(&chat_response.metadata).ok()
    } else {
        None
    };

    sqlx::query(
        r#"
        INSERT INTO messages (conversation_id, role, content, model, prompt_tokens, completion_tokens, total_tokens, metadata, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))
        "#
    )
    .bind(conversation_id)
    .bind("assistant")
    .bind(&chat_response.content)
    .bind(&chat_response.model)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total_tokens)
    .bind(metadata_json)
    .execute(db.pool())
    .await?;

    // 更新对话的最后更新时间和消息计数
    sqlx::query(
        "UPDATE conversations SET updated_at = datetime('now'), message_count = message_count + 2 WHERE conversation_id = ? AND user_id = ?"
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
        "SELECT COUNT(*) > 0 FROM conversations WHERE conversation_id = ? AND user_id = ?"
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_one(db.pool())
    .await?;

    if conversation_exists {
        // 更新现有对话
        sqlx::query(
            "UPDATE conversations SET updated_at = datetime('now'), message_count = message_count + 1 WHERE conversation_id = ? AND user_id = ?"
        )
        .bind(conversation_id)
        .bind(user_id)
        .execute(db.pool())
        .await?;
    } else {
        // 创建新对话
        let title = if message.chars().count() > 50 {
            message.chars().take(50).collect::<String>() + "..."
        } else {
            message.to_string()
        }.replace('\n', " ");

        sqlx::query(
            r#"
            INSERT INTO conversations (conversation_id, user_id, title, model, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))
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
