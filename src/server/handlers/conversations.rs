use axum::{
    extract::{Query, State, Path},
    response::Json,
};
use tracing::{info, error};
use sqlx::Row;

use crate::server::models::*;
use super::chat::ServerState;

/// 获取用户对话列表
pub async fn list_conversations_handler(
    State(state): State<ServerState>,
    Query(params): Query<ListConversationsQuery>,
) -> Result<Json<ConversationListResponse>, ErrorResponse> {
    info!("获取对话列表: limit={}, offset={}, session_id={:?}",
          params.limit, params.offset, params.session_id);

    // 目前暂时使用用户ID = 1，实际应该从认证信息中获取
    let user_id = 1i64;

    // 构建基本查询SQL
    let mut sql = String::from(
        "SELECT id, user_id, session_id, title, model, message_count, summary, created_at, updated_at \
         FROM conversations WHERE user_id = ?"
    );

    // 添加可选的session_id过滤
    if let Some(ref session_id) = params.session_id {
        sql.push_str(" AND session_id = ?");
    }

    // 添加搜索条件
    if let Some(ref search) = params.search {
        sql.push_str(" AND (title LIKE ? OR summary LIKE ?)");
    }

    // 添加排序和分页
    sql.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");

    // 执行查询
    let mut query = sqlx::query(&sql);
    query = query.bind(user_id);

    if let Some(ref session_id) = params.session_id {
        query = query.bind(session_id);
    }

    if let Some(ref search) = params.search {
        query = query.bind(format!("%{}%", search));
        query = query.bind(format!("%{}%", search));
    }

    query = query.bind(params.limit);
    query = query.bind(params.offset);

    let rows = query
        .fetch_all(state.database.pool())
        .await
        .map_err(|e| {
            error!("查询对话列表失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "查询对话列表失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Utc::now(),
            }
        })?;

    // 转换为API模型
    let conversations: Vec<Conversation> = rows
        .into_iter()
        .map(|row| Conversation {
            id: Some(row.try_get::<i64, _>("id").unwrap_or(0) as i32),
            user_id: row.try_get::<i64, _>("user_id").unwrap_or(0) as i32,
            session_id: row.try_get::<Option<String>, _>("session_id").ok().flatten(),
            title: row.try_get::<Option<String>, _>("title").ok().flatten(),
            model: row.try_get::<String, _>("model").unwrap_or_default(),
            message_count: Some(row.try_get::<i32, _>("message_count").unwrap_or(0)),
            summary: row.try_get::<Option<String>, _>("summary").ok().flatten(),
            created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
        .collect();

    // 获取总数
    let total_sql = "SELECT COUNT(*) FROM conversations WHERE user_id = ?";
    let mut total_query = sqlx::query(total_sql);
    total_query = total_query.bind(user_id);

    let total_row = total_query
        .fetch_one(state.database.pool())
        .await
        .map_err(|e| {
            error!("查询对话总数失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "查询对话总数失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Utc::now(),
            }
        })?;

    let total: i64 = total_row.try_get::<i64, _>(0).unwrap_or(0);

    let page_size = params.limit.clamp(1, 100);
    let page = (params.offset / page_size) + 1;
    let total_pages = (total + page_size - 1) / page_size;

    Ok(Json(ConversationListResponse {
        conversations,
        total,
        page,
        page_size,
        total_pages,
    }))
}

/// 创建新对话
pub async fn create_conversation_handler(
    State(_state): State<ServerState>,
    Json(request): Json<CreateConversationRequest>,
) -> Json<CreateConversationResponse> {
    info!("创建对话: session_id={:?}, title={:?}",
          request.session_id, request.title);

    // TODO: 实现实际的数据库插入
    let mock_conversation = Conversation {
        id: Some(3),
        user_id: 1, // TODO: 从鉴权信息获取真实用户ID
        session_id: request.session_id,
        title: request.title.clone(),
        model: "qwen-plus".to_string(), // 默认模型
        message_count: Some(0),
        summary: Some("新创建的对话".to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let first_message = if let Some(initial_message) = &request.initial_message {
        Some(Message {
            id: Some(10),
            conversation_id: 3,
            role: "user".to_string(),
            content: initial_message.clone(),
            model: None,
            usage: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        })
    } else {
        None
    };

    Json(CreateConversationResponse {
        conversation: mock_conversation,
        first_message,
    })
}

/// 获取对话详情
pub async fn get_conversation_handler(
    State(_state): State<ServerState>,
    Path(conversation_id): Path<i32>,
) -> Json<Conversation> {
    info!("获取对话详情: conversation_id={}", conversation_id);

    // TODO: 实现实际的数据库查询
    Json(Conversation {
        id: Some(conversation_id),
        user_id: 1,
        session_id: Some("session_123".to_string()),
        title: Some("关于Rust编程的讨论".to_string()),
        model: "qwen-plus".to_string(),
        message_count: Some(5),
        summary: Some("关于Rust编程语言特性和最佳实践的讨论".to_string()),
        created_at: chrono::Utc::now() - chrono::Duration::hours(2),
        updated_at: chrono::Utc::now() - chrono::Duration::minutes(30),
    })
}

/// 更新对话标题
pub async fn update_conversation_title_handler(
    State(_state): State<ServerState>,
    Path(conversation_id): Path<i32>,
    Json(request): Json<UpdateConversationTitleRequest>,
) -> Json<Conversation> {
    info!("更新对话标题: conversation_id={}, new_title={}",
          conversation_id, request.title);

    // TODO: 实现实际的数据库更新
    Json(Conversation {
        id: Some(conversation_id),
        user_id: 1,
        session_id: Some("session_123".to_string()),
        title: Some(request.title),
        model: "qwen-plus".to_string(),
        message_count: Some(5),
        summary: Some("对话内容已更新".to_string()),
        created_at: chrono::Utc::now() - chrono::Duration::hours(2),
        updated_at: chrono::Utc::now(),
    })
}

/// 删除对话
pub async fn delete_conversation_handler(
    State(_state): State<ServerState>,
    Path(conversation_id): Path<i32>,
) -> Json<serde_json::Value> {
    info!("删除对话: conversation_id={}", conversation_id);

    // TODO: 实现实际的数据库删除
    Json(serde_json::json!({
        "success": true,
        "message": "对话删除成功",
        "conversation_id": conversation_id,
        "timestamp": chrono::Utc::now()
    }))
}