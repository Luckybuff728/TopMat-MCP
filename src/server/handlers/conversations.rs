use axum::{
    extract::{Query, State, Path, Extension},
    response::Json,
};
use tracing::{info, error};
use sqlx::Row;

use crate::server::models::*;
use crate::server::middleware::AuthUser;
use super::chat::ServerState;

/// 获取用户对话列表
pub async fn list_conversations_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Query(params): Query<ListConversationsQuery>,
) -> Result<Json<ConversationListResponse>, ErrorResponse> {
    info!("获取对话列表: limit={}, offset={}, user_id={}",
          params.limit, params.offset, auth_user.user_id);

    // 从认证信息中获取真实用户ID
    let user_id = auth_user.user_id as i64;

    // 构建基本查询SQL
    let mut sql = String::from(
        "SELECT conversation_id, user_id, title, model, message_count, summary, created_at, updated_at \
         FROM conversations WHERE user_id = ?"
    );

    // 添加搜索条件
    if let Some(ref search) = params.search {
        sql.push_str(" AND (title LIKE ? OR summary LIKE ?)");
    }

    // 添加排序和分页
    sql.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");

    // 执行查询
    let mut query = sqlx::query(&sql);
    query = query.bind(user_id);

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
            conversation_id: Some(row.try_get::<i64, _>("conversation_id").unwrap_or(0) as i32),
            user_id: row.try_get::<i64, _>("user_id").unwrap_or(0) as i32,
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
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Json(request): Json<CreateConversationRequest>,
) -> Result<Json<CreateConversationResponse>, ErrorResponse> {
    info!("创建对话: title={:?}", request.title);

    // 获取用户ID
    let user_id = auth_user.user_id as i64;

    // 开始数据库事务
    let mut tx = state.database.pool().begin().await.map_err(|e| {
        error!("开始事务失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "创建对话失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Utc::now(),
        }
    })?;

    // 创建对话
    let conversation_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO conversations (user_id, title, model, created_at, updated_at)
        VALUES (?1, ?2, ?3, datetime('now'), datetime('now'))
        RETURNING conversation_id
        "#
    )
    .bind(user_id)
    .bind(&request.title)
    .bind("qwen-plus") // 默认模型，后续可以从请求中获取
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        error!("创建对话失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "创建对话失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Utc::now(),
        }
    })?;

    // 如果有初始消息，创建消息
    let first_message = if let Some(initial_message) = &request.initial_message {
        let message_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO messages (conversation_id, role, content, model, created_at)
            VALUES (?1, ?2, ?3, ?4, datetime('now'))
            RETURNING message_id
            "#
        )
        .bind(conversation_id)
        .bind("user")
        .bind(initial_message)
        .bind("qwen-plus")
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            error!("创建初始消息失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "创建初始消息失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Utc::now(),
            }
        })?;

        Some(Message {
            id: Some(message_id as i32),
            conversation_id: conversation_id as i32,
            role: "user".to_string(),
            content: initial_message.clone(),
            model: Some("qwen-plus".to_string()),
            usage: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        })
    } else {
        None
    };

    // 提交事务
    tx.commit().await.map_err(|e| {
        error!("提交事务失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "创建对话失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Utc::now(),
        }
    })?;

    // 查询创建的对话
    let row = sqlx::query(
        r#"
        SELECT conversation_id, user_id, title, model, message_count, summary, created_at, updated_at
        FROM conversations
        WHERE conversation_id = ?
        "#
    )
    .bind(conversation_id)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| {
        error!("查询创建的对话失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "查询创建的对话失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Utc::now(),
        }
    })?;

    let conversation = Conversation {
        conversation_id: Some(row.try_get::<i64, _>("conversation_id").unwrap_or(0) as i32),
        user_id: row.try_get::<i64, _>("user_id").unwrap_or(0) as i32,
        title: row.try_get::<Option<String>, _>("title").ok().flatten(),
        model: row.try_get::<String, _>("model").unwrap_or_default(),
        message_count: Some(row.try_get::<i32, _>("message_count").unwrap_or(0)),
        summary: row.try_get::<Option<String>, _>("summary").ok().flatten(),
        created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
            .unwrap_or_else(|_| chrono::Utc::now()),
    };

    info!("成功创建对话: conversation_id={}, title={:?}", conversation_id, request.title);

    Ok(Json(CreateConversationResponse {
        conversation,
        first_message,
    }))
}

/// 获取对话详情
pub async fn get_conversation_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Path(conversation_id): Path<i32>,
) -> Result<Json<Conversation>, ErrorResponse> {
    info!("获取对话详情: conversation_id={}, user_id={}", conversation_id, auth_user.user_id);

    // 查询对话详情
    let row = sqlx::query(
        r#"
        SELECT conversation_id, user_id, title, model, message_count, summary, created_at, updated_at
        FROM conversations
        WHERE conversation_id = ? AND user_id = ?
        "#
    )
    .bind(conversation_id as i64)
    .bind(auth_user.user_id as i64)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| {
        error!("查询对话详情失败: {}", e);
        if e.to_string().contains("no rows") {
            ErrorResponse {
                error: "conversation_not_found".to_string(),
                message: "对话不存在".to_string(),
                details: Some(serde_json::json!({
                    "conversation_id": conversation_id
                })),
                timestamp: chrono::Utc::now(),
            }
        } else {
            ErrorResponse {
                error: "database_error".to_string(),
                message: "查询对话详情失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Utc::now(),
            }
        }
    })?;

    let conversation = Conversation {
        conversation_id: Some(row.try_get::<i64, _>("conversation_id").unwrap_or(0) as i32),
        user_id: row.try_get::<i64, _>("user_id").unwrap_or(0) as i32,
        title: row.try_get::<Option<String>, _>("title").ok().flatten(),
        model: row.try_get::<String, _>("model").unwrap_or_default(),
        message_count: Some(row.try_get::<i32, _>("message_count").unwrap_or(0)),
        summary: row.try_get::<Option<String>, _>("summary").ok().flatten(),
        created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
            .unwrap_or_else(|_| chrono::Utc::now()),
    };

    Ok(Json(conversation))
}

/// 更新对话标题
pub async fn update_conversation_title_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Path(conversation_id): Path<i32>,
    Json(request): Json<UpdateConversationTitleRequest>,
) -> Result<Json<Conversation>, ErrorResponse> {
    info!("更新对话标题: conversation_id={}, new_title={}, user_id={}",
          conversation_id, request.title, auth_user.user_id);

    // 执行更新操作
    let result = sqlx::query(
        r#"
        UPDATE conversations
        SET title = ?, updated_at = datetime('now')
        WHERE conversation_id = ? AND user_id = ?
        "#
    )
    .bind(&request.title)
    .bind(conversation_id as i64)
    .bind(auth_user.user_id as i64)
    .execute(state.database.pool())
    .await
    .map_err(|e| {
        error!("更新对话标题失败: {}", e);
        if e.to_string().contains("no rows") {
            ErrorResponse {
                error: "conversation_not_found".to_string(),
                message: "对话不存在".to_string(),
                details: Some(serde_json::json!({
                    "conversation_id": conversation_id
                })),
                timestamp: chrono::Utc::now(),
            }
        } else {
            ErrorResponse {
                error: "database_error".to_string(),
                message: "更新对话标题失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Utc::now(),
            }
        }
    })?;

    if result.rows_affected() == 0 {
        return Err(ErrorResponse {
            error: "conversation_not_found".to_string(),
            message: "对话不存在".to_string(),
            details: Some(serde_json::json!({
                "conversation_id": conversation_id
            })),
            timestamp: chrono::Utc::now(),
        });
    }

    // 查询更新后的对话详情
    let row = sqlx::query(
        r#"
        SELECT conversation_id, user_id, title, model, message_count, summary, created_at, updated_at
        FROM conversations
        WHERE conversation_id = ?
        "#
    )
    .bind(conversation_id as i64)
    .fetch_one(state.database.pool())
    .await
    .map_err(|e| {
        error!("查询更新后的对话详情失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "查询更新后的对话详情失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Utc::now(),
        }
    })?;

    let conversation = Conversation {
        conversation_id: Some(row.try_get::<i64, _>("conversation_id").unwrap_or(0) as i32),
        user_id: row.try_get::<i64, _>("user_id").unwrap_or(0) as i32,
        title: row.try_get::<Option<String>, _>("title").ok().flatten(),
        model: row.try_get::<String, _>("model").unwrap_or_default(),
        message_count: Some(row.try_get::<i32, _>("message_count").unwrap_or(0)),
        summary: row.try_get::<Option<String>, _>("summary").ok().flatten(),
        created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
            .unwrap_or_else(|_| chrono::Utc::now()),
    };

    info!("成功更新对话标题: conversation_id={}, title={:?}", conversation_id, request.title);

    Ok(Json(conversation))
}

/// 删除对话
pub async fn delete_conversation_handler(
    Extension(auth_user): Extension<AuthUser>,
    State(state): State<ServerState>,
    Path(conversation_id): Path<i32>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    info!("删除对话: conversation_id={}, user_id={}", conversation_id, auth_user.user_id);

    // 开始事务进行级联删除
    let mut tx = state.database.pool().begin().await.map_err(|e| {
        error!("开始事务失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "删除对话失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Utc::now(),
        }
    })?;

    // 首先删除该对话的所有消息
    let delete_messages_result = sqlx::query(
        "DELETE FROM messages WHERE conversation_id = ?"
    )
    .bind(conversation_id as i64)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error!("删除对话消息失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "删除对话消息失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Utc::now(),
        }
    })?;

    // 删除对话本身
    let delete_conversation_result = sqlx::query(
        "DELETE FROM conversations WHERE conversation_id = ? AND user_id = ?"
    )
    .bind(conversation_id as i64)
    .bind(auth_user.user_id as i64)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error!("删除对话失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "删除对话失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Utc::now(),
        }
    })?;

    // 提交事务
    tx.commit().await.map_err(|e| {
        error!("提交删除事务失败: {}", e);
        ErrorResponse {
            error: "database_error".to_string(),
            message: "删除对话失败".to_string(),
            details: Some(serde_json::json!({
                "error": e.to_string()
            })),
            timestamp: chrono::Utc::now(),
        }
    })?;

    if delete_conversation_result.rows_affected() == 0 {
        return Err(ErrorResponse {
            error: "conversation_not_found".to_string(),
            message: "对话不存在".to_string(),
            details: Some(serde_json::json!({
                "conversation_id": conversation_id
            })),
            timestamp: chrono::Utc::now(),
        });
    }

    let deleted_messages_count = delete_messages_result.rows_affected();
    info!("成功删除对话: conversation_id={}, deleted_messages={}, user_id={}",
          conversation_id, deleted_messages_count, auth_user.user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "对话删除成功",
        "conversation_id": conversation_id,
        "deleted_messages_count": deleted_messages_count,
        "timestamp": chrono::Utc::now()
    })))
}