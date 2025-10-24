use axum::{
    extract::{Query, State, Path},
    response::Json,
};
use tracing::{info, error};
use sqlx::Row;

use crate::server::models::*;
use super::chat::ServerState;

/// 获取对话的消息历史
pub async fn list_messages_handler(
    State(state): State<ServerState>,
    Path(conversation_id): Path<i32>,
    Query(params): Query<ListMessagesQuery>,
) -> Result<Json<MessageListResponse>, ErrorResponse> {
    info!("获取消息历史: conversation_id={}, limit={}, offset={}",
          conversation_id, params.limit, params.offset);

    // 构建基本查询SQL
    let mut sql = String::from(
        "SELECT id, conversation_id, role, content, model, prompt_tokens, completion_tokens, total_tokens, metadata, created_at \
         FROM messages WHERE conversation_id = ?"
    );

    // 添加消息ID过滤（如果指定了before参数）
    if let Some(before_id) = params.before {
        sql.push_str(" AND id < ?");
    }

    // 添加排序和分页
    sql.push_str(" ORDER BY created_at ASC, id ASC LIMIT ? OFFSET ?");

    // 执行查询
    let mut query = sqlx::query(&sql);
    query = query.bind(conversation_id as i64);

    if let Some(before_id) = params.before {
        query = query.bind(before_id as i64);
    }

    query = query.bind(params.limit);
    query = query.bind(params.offset);

    let rows = query
        .fetch_all(state.database.pool())
        .await
        .map_err(|e| {
            error!("查询消息历史失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "查询消息历史失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Utc::now(),
            }
        })?;

    // 转换为API模型
    let messages: Vec<Message> = rows
        .into_iter()
        .map(|row| {
            let metadata_str: Option<String> = row.try_get::<Option<String>, _>("metadata").ok().flatten();
            let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

            Message {
                id: Some(row.try_get::<i64, _>("id").unwrap_or(0) as i32),
                conversation_id: row.try_get::<i64, _>("conversation_id").unwrap_or(0) as i32,
                role: row.try_get::<String, _>("role").unwrap_or_default(),
                content: row.try_get::<String, _>("content").unwrap_or_default(),
                model: row.try_get::<Option<String>, _>("model").ok().flatten(),
                usage: if row.try_get::<Option<i32>, _>("prompt_tokens").ok().flatten().is_some() {
                    Some(TokenUsage {
                        prompt_tokens: row.try_get::<Option<i32>, _>("prompt_tokens").ok().flatten().unwrap_or(0) as u32,
                        completion_tokens: row.try_get::<Option<i32>, _>("completion_tokens").ok().flatten().unwrap_or(0) as u32,
                        total_tokens: row.try_get::<Option<i32>, _>("total_tokens").ok().flatten().unwrap_or(0) as u32,
                    })
                } else {
                    None
                },
                metadata,
                created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .unwrap_or_else(|_| chrono::Utc::now()),
            }
        })
        .collect();

    // 获取总数
    let total_sql = "SELECT COUNT(*) FROM messages WHERE conversation_id = ?";
    let mut total_query = sqlx::query(total_sql);
    total_query = total_query.bind(conversation_id as i64);

    let total_row = total_query
        .fetch_one(state.database.pool())
        .await
        .map_err(|e| {
            error!("查询消息总数失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "查询消息总数失败".to_string(),
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
    let has_more = (params.offset + page_size) < total;

    Ok(Json(MessageListResponse {
        messages,
        conversation_id,
        total,
        page,
        page_size,
        total_pages,
        has_more,
    }))
}

/// 获取单个消息详情
pub async fn get_message_handler(
    State(state): State<ServerState>,
    Path((conversation_id, message_id)): Path<(i32, i32)>,
) -> Result<Json<Message>, ErrorResponse> {
    info!("获取消息详情: conversation_id={}, message_id={}",
          conversation_id, message_id);

    let sql = "SELECT id, conversation_id, role, content, model, prompt_tokens, completion_tokens, total_tokens, metadata, created_at \
              FROM messages WHERE id = ? AND conversation_id = ?";

    let row = sqlx::query(sql)
        .bind(message_id as i64)
        .bind(conversation_id as i64)
        .fetch_one(state.database.pool())
        .await
        .map_err(|e| {
            error!("查询消息详情失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "查询消息详情失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Utc::now(),
            }
        })?;

    let metadata_str: Option<String> = row.try_get::<Option<String>, _>("metadata").ok().flatten();
    let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

    let message = Message {
        id: Some(row.try_get::<i64, _>("id").unwrap_or(0) as i32),
        conversation_id: row.try_get::<i64, _>("conversation_id").unwrap_or(0) as i32,
        role: row.try_get::<String, _>("role").unwrap_or_default(),
        content: row.try_get::<String, _>("content").unwrap_or_default(),
        model: row.try_get::<Option<String>, _>("model").ok().flatten(),
        usage: if row.try_get::<Option<i32>, _>("prompt_tokens").ok().flatten().is_some() {
            Some(TokenUsage {
                prompt_tokens: row.try_get::<Option<i32>, _>("prompt_tokens").ok().flatten().unwrap_or(0) as u32,
                completion_tokens: row.try_get::<Option<i32>, _>("completion_tokens").ok().flatten().unwrap_or(0) as u32,
                total_tokens: row.try_get::<Option<i32>, _>("total_tokens").ok().flatten().unwrap_or(0) as u32,
            })
        } else {
            None
        },
        metadata,
        created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .unwrap_or_else(|_| chrono::Utc::now()),
    };

    Ok(Json(message))
}

/// 删除消息
pub async fn delete_message_handler(
    State(state): State<ServerState>,
    Path((conversation_id, message_id)): Path<(i32, i32)>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    info!("删除消息: conversation_id={}, message_id={}",
          conversation_id, message_id);

    let sql = "DELETE FROM messages WHERE id = ? AND conversation_id = ?";

    let result = sqlx::query(sql)
        .bind(message_id as i64)
        .bind(conversation_id as i64)
        .execute(state.database.pool())
        .await
        .map_err(|e| {
            error!("删除消息失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "删除消息失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Utc::now(),
            }
        })?;

    let rows_affected = result.rows_affected();

    if rows_affected == 0 {
        return Err(ErrorResponse {
            error: "not_found".to_string(),
            message: "消息不存在".to_string(),
            details: Some(serde_json::json!({
                "message_id": message_id,
                "conversation_id": conversation_id
            })),
            timestamp: chrono::Utc::now(),
        });
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "消息删除成功",
        "message_id": message_id,
        "conversation_id": conversation_id,
        "timestamp": chrono::Utc::now()
    })))
}

/// 添加新消息到对话
pub async fn add_message_handler(
    State(state): State<ServerState>,
    Path(conversation_id): Path<i32>,
    Json(request): Json<Message>,
) -> Result<Json<Message>, ErrorResponse> {
    info!("添加消息到对话: conversation_id={}, role={}",
          conversation_id, request.role);

    // 将metadata转换为JSON字符串
    let metadata_str = request.metadata
        .as_ref()
        .and_then(|m| serde_json::to_string(m).ok());

    let sql = "INSERT INTO messages (conversation_id, role, content, model, prompt_tokens, completion_tokens, total_tokens, metadata, created_at) \
              VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";

    let result = sqlx::query(sql)
        .bind(conversation_id as i64)
        .bind(&request.role)
        .bind(&request.content)
        .bind(&request.model)
        .bind(request.usage.as_ref().map(|u| u.prompt_tokens))
        .bind(request.usage.as_ref().map(|u| u.completion_tokens))
        .bind(request.usage.as_ref().map(|u| u.total_tokens))
        .bind(metadata_str)
        .bind(chrono::Utc::now())
        .execute(state.database.pool())
        .await
        .map_err(|e| {
            error!("添加消息失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "添加消息失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Utc::now(),
            }
        })?;

    let message_id = result.last_insert_rowid();

    // 更新对话的消息计数和更新时间
    let update_sql = "UPDATE conversations SET message_count = message_count + 1, updated_at = ? WHERE id = ?";
    sqlx::query(update_sql)
        .bind(chrono::Utc::now())
        .bind(conversation_id as i64)
        .execute(state.database.pool())
        .await
        .map_err(|e| {
            error!("更新对话信息失败: {}", e);
            ErrorResponse {
                error: "database_error".to_string(),
                message: "更新对话信息失败".to_string(),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
                timestamp: chrono::Utc::now(),
            }
        })?;

    let message = Message {
        id: Some(message_id as i32),
        conversation_id,
        role: request.role,
        content: request.content,
        model: request.model,
        usage: request.usage,
        metadata: request.metadata,
        created_at: chrono::Utc::now(),
    };

    Ok(Json(message))
}