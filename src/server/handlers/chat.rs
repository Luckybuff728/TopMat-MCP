use axum::{
    extract::{Json, State, Request, Extension},
    response::{IntoResponse, Response},
    body::Bytes,
};
use tracing::{info, error, warn};
use rig::client::{ProviderClient, CompletionClient};

use crate::server::models::*;
use crate::server::auth::AuthClient;
use crate::server::database::DatabaseConnection;
use crate::server::middleware::auth::AuthUser;

/// 服务器状态
#[derive(Clone)]
pub struct ServerState {
    /// 标记是否配置了通义千问
    pub qwen_available: bool,
    /// 鉴权客户端
    pub auth_client: AuthClient,
    /// 数据库连接
    pub database: DatabaseConnection,
}

impl ServerState {
    pub async fn new(database: DatabaseConnection) -> Self {
        // 检查是否配置了通义千问API密钥
        let qwen_available = match std::env::var("DASHSCOPE_API_KEY") {
            Ok(_) => {
                tracing::info!("通义千问 API 密钥已配置，通义千问模型可用");
                true
            }
            Err(_) => {
                tracing::warn!("未配置 DASHSCOPE_API_KEY，通义千问模型将不可用。请在 .env 文件中配置 API 密钥。");
                false
            }
        };

        // 创建鉴权客户端
        let auth_api_url = std::env::var("AUTH_API_URL").ok();
        let auth_client = AuthClient::new(auth_api_url, database.clone());

        tracing::info!("鉴权客户端已初始化");

        Self {
            qwen_available,
            auth_client,
            database
        }
    }
}

/// 处理聊天请求
pub async fn chat_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Json(request): Json<ChatRequest>,
) -> Result<axum::response::Response, ErrorResponse> {
    info!("收到聊天请求: model={}, stream={}, message={}, user_id={}",
          request.model, request.stream, request.message, auth_user.user_id);

    // 为新对话生成UUID
    let mut request_with_id = request.clone();
    if request_with_id.conversation_id.is_none() {
        request_with_id.conversation_id = Some(crate::server::models::generate_conversation_id());
    }
    tracing::info!("使用对话ID: {}", request_with_id.conversation_id.as_ref().unwrap());
    // 处理聊天请求，获取ChatResponse用于保存助手消息
    let (response, chat_response) = crate::server::model_router::get_model_router().handle_chat_request_with_response(request_with_id.clone()).await?;

    // 先保存聊天数据（创建对话和用户消息）
    let db_clone = state.database.clone();
    let request_clone = request_with_id.clone();
    let user_id = auth_user.user_id as i64;
    let conversation_id_for_assistant = match save_chat_request_data_with_retry(&db_clone, &request_clone, user_id, 3).await {
        Ok((attempts, conversation_id)) => {
            info!("Chat request data saved successfully after {} attempts for user {}, conversation_id: {}", attempts, user_id, conversation_id);
            conversation_id
        }
        Err(e) => {
            error!("Failed to save chat request data after multiple retries: {}", e);
            // 如果保存聊天数据失败，我们仍然要返回响应，但不保存助手消息
            return Ok(response);
        }
    };

    // 异步保存助手消息到数据库（确保对话已创建）
    let db_clone = state.database.clone();
    let mut chat_response_clone = chat_response.clone();
    chat_response_clone.conversation_id = conversation_id_for_assistant.clone();
    let user_id = auth_user.user_id as i64;
    tokio::spawn(async move {
        match save_assistant_message_with_retry(&db_clone, &chat_response_clone, user_id, 3).await {
            Ok(attempts) => {
                info!("Assistant message saved successfully after {} attempts for conversation {}", attempts, conversation_id_for_assistant);
            }
            Err(e) => {
                error!("Failed to save assistant message after multiple retries: {}", e);
            }
        }
    });

    Ok(response)
}

/// 带重试机制的保存聊天请求数据到数据库
async fn save_chat_request_data_with_retry(
    db: &DatabaseConnection,
    request: &ChatRequest,
    user_id: i64,
    max_retries: u32,
) -> Result<(u32, String), Box<dyn std::error::Error + Send + Sync>> {
    let mut attempts = 0;

    for attempt in 1..=max_retries {
        attempts = attempt;
        match save_chat_request_data(db, request, user_id).await {
            Ok(conversation_id) => return Ok((attempts, conversation_id)),
            Err(e) => {
                warn!("Attempt {} failed to save chat data for user {}: {}", attempt, user_id, e);
                if attempt < max_retries {
                    // 指数退避：1s, 2s, 4s...
                    let delay = std::time::Duration::from_millis(1000 * (1 << (attempt - 1)));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(format!("Failed to save chat data after {} attempts for user {}", max_retries, user_id).into())
}

/// 保存聊天请求数据到数据库
async fn save_chat_request_data(
    db: &DatabaseConnection,
    request: &ChatRequest,
    user_id: i64,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {

    // 现在request.conversation_id总是存在，只需要检查数据库中是否有记录
    let conversation_id = request.conversation_id.as_ref().unwrap().clone();

    // 检查这个对话在数据库中是否存在
    let conversation_exists: bool = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) > 0
        FROM conversations
        WHERE conversation_id = ? AND user_id = ?
        "#
    )
    .bind(&conversation_id)
    .bind(user_id)
    .fetch_one(db.pool())
    .await?;

    if conversation_exists {
        // 使用现有对话，更新时间戳
        sqlx::query(
            r#"
            UPDATE conversations
            SET updated_at = datetime('now'), message_count = message_count + 1
            WHERE conversation_id = ? AND user_id = ?
            "#
        )
        .bind(&conversation_id)
        .bind(user_id)
        .execute(db.pool())
        .await?;
    } else {
        // 数据库中没有记录，创建新对话
        sqlx::query(
            r#"
            INSERT INTO conversations (conversation_id, user_id, title, model, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))
            "#
        )
        .bind(&conversation_id)
        .bind(user_id)
        .bind(generate_title_from_message(&request.message))
        .bind(&request.model)
        .execute(db.pool())
        .await?;
    }

    // 保存用户消息
    sqlx::query(
        r#"
        INSERT INTO messages (conversation_id, role, content, model, created_at)
        VALUES (?1, ?2, ?3, ?4, datetime('now'))
        "#
    )
    .bind(&conversation_id)
    .bind("user")
    .bind(&request.message)
    .bind(&request.model)
    .execute(db.pool())
    .await?;

    info!("Saved chat request data for conversation {}", conversation_id);
    Ok(conversation_id)
}

/// 从用户消息生成对话标题
fn generate_title_from_message(message: &str) -> String {
    let title = if message.chars().count() > 50 {
        // 安全地截取字符，避免UTF-8字符边界错误
        let truncated: String = message.chars().take(50).collect();
        format!("{}...", truncated)
    } else {
        message.to_string()
    };
    title.replace('\n', " ")
}


/// 带重试机制的保存助手消息到数据库
async fn save_assistant_message_with_retry(
    db: &DatabaseConnection,
    chat_response: &ChatResponse,
    user_id: i64,
    max_retries: u32,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let mut attempts = 0;

    for attempt in 1..=max_retries {
        attempts = attempt;
        match save_assistant_message(db, chat_response, user_id).await {
            Ok(_) => return Ok(attempts),
            Err(e) => {
                warn!("Attempt {} failed to save assistant message for user {}: {}", attempt, user_id, e);
                if attempt < max_retries {
                    // 指数退避：1s, 2s, 4s...
                    let delay = std::time::Duration::from_millis(1000 * (1 << (attempt - 1)));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(format!("Failed to save assistant message after {} attempts for user {}", max_retries, user_id).into())
}

/// 保存助手消息到数据库
async fn save_assistant_message(
    db: &DatabaseConnection,
    chat_response: &ChatResponse,
    user_id: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 验证用户是否有权限保存到此对话
    let conversation_exists: bool = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) > 0
        FROM conversations
        WHERE conversation_id = ? AND user_id = ?
        "#
    )
    .bind(&chat_response.conversation_id)
    .bind(user_id)
    .fetch_one(db.pool())
    .await?;

    if !conversation_exists {
        return Err(format!("Conversation {} not found or access denied for user {}",
            chat_response.conversation_id, user_id).into());
    }

    // 保存助手消息
    let (prompt_tokens, completion_tokens, total_tokens) = if let Some(usage) = &chat_response.usage {
        (usage.prompt_tokens as i32, usage.completion_tokens as i32, usage.total_tokens as i32)
    } else {
        (0, 0, 0)
    };

    sqlx::query(
        r#"
        INSERT INTO messages (conversation_id, role, content, model, prompt_tokens, completion_tokens, total_tokens, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))
        "#
    )
    .bind(&chat_response.conversation_id)
    .bind("assistant")
    .bind(&chat_response.content)
    .bind(&chat_response.model)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total_tokens)
    .execute(db.pool())
    .await?;

    // 更新对话的最后更新时间
    sqlx::query(
        r#"
        UPDATE conversations
        SET updated_at = datetime('now'), message_count = message_count + 1
        WHERE conversation_id = ? AND user_id = ?
        "#
    )
    .bind(&chat_response.conversation_id)
    .bind(user_id)
    .execute(db.pool())
    .await?;

    info!("Saved assistant message for conversation {}", chat_response.conversation_id);
    Ok(())
}
