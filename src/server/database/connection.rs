use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing::info;

/// 数据库连接池
#[derive(Clone)]
pub struct DatabaseConnection {
    pool: PgPool,
}

impl DatabaseConnection {
    /// 获取数据库连接池
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // /// 检查数据库连接是否健康
    // pub async fn health_check(&self) -> Result<(), sqlx::Error> {
    //     let result = sqlx::query("SELECT 1").fetch_one(&self.pool).await;

    //     match result {
    //         Ok(_) => {
    //             info!("数据库连接健康检查通过");
    //             Ok(())
    //         }
    //         Err(e) => {
    //             error!("数据库连接健康检查失败: {}", e);
    //             Err(e)
    //         }
    //     }
    // }

    /// 获取对话历史消息
    pub async fn get_conversation_history(
        &self,
        conversation_id: &str,
        limit: i64,
    ) -> Result<Vec<crate::server::models::Message>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT message_id, conversation_id, role, content, model, created_at,
                   prompt_tokens, completion_tokens, total_tokens, metadata
            FROM messages
            WHERE conversation_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(conversation_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut messages: Vec<crate::server::models::Message> = rows
            .into_iter()
            .map(|row| {
                use sqlx::Row;
                let prompt_tokens: i32 = row
                    .try_get::<Option<i32>, _>("prompt_tokens")
                    .ok()
                    .flatten()
                    .unwrap_or(0);
                let completion_tokens: i32 = row
                    .try_get::<Option<i32>, _>("completion_tokens")
                    .ok()
                    .flatten()
                    .unwrap_or(0);
                let total_tokens: i32 = row
                    .try_get::<Option<i32>, _>("total_tokens")
                    .ok()
                    .flatten()
                    .unwrap_or(0);

                let metadata_str: Option<String> =
                    row.try_get::<Option<String>, _>("metadata").ok().flatten();
                let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

                crate::server::models::Message {
                    id: Some(row.try_get::<i64, _>("message_id").unwrap_or(0) as i32),
                    conversation_id: row
                        .try_get::<String, _>("conversation_id")
                        .unwrap_or_default(),
                    role: row.try_get::<String, _>("role").unwrap_or_default(),
                    content: row.try_get::<String, _>("content").unwrap_or_default(),
                    model: row.try_get::<Option<String>, _>("model").ok().flatten(),
                    usage: Some(crate::server::models::TokenUsage {
                        prompt_tokens: prompt_tokens as u32,
                        completion_tokens: completion_tokens as u32,
                        total_tokens: total_tokens as u32,
                    }),
                    metadata,
                    created_at: row
                        .try_get::<chrono::DateTime<chrono::Local>, _>("created_at")
                        .unwrap_or_else(|_| chrono::Local::now()),
                }
            })
            .collect();

        // 按时间正序排列回复给模型
        messages.reverse();
        Ok(messages)
    }
}

/// 初始化数据库连接
pub async fn init_database(database_url: &str) -> Result<DatabaseConnection, sqlx::Error> {
    info!("正在初始化数据库连接: {}", database_url);

    // 创建 PostgreSQL 连接池
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    info!("数据库连接池创建成功");

    // 运行数据库迁移
    run_migrations(&pool).await?;

    info!("数据库初始化完成");

    Ok(DatabaseConnection { pool })
}

/// 运行数据库迁移
async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    info!("正在运行数据库迁移...");

    // 创建用户表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id BIGSERIAL PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            subscription_level TEXT NOT NULL DEFAULT 'free',
            subscription_expires_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建API密钥表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS api_keys (
            id BIGSERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL REFERENCES users(id),
            api_key TEXT NOT NULL UNIQUE,
            key_name TEXT NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            expires_at TIMESTAMPTZ,
            last_used_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建对话表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS conversations (
            conversation_id TEXT PRIMARY KEY,
            user_id BIGINT NOT NULL REFERENCES users(id),
            title TEXT,
            model TEXT NOT NULL,
            message_count INTEGER DEFAULT 0,
            summary TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建消息表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            message_id BIGSERIAL PRIMARY KEY,
            conversation_id TEXT NOT NULL REFERENCES conversations(conversation_id),
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            model TEXT,
            prompt_tokens INTEGER,
            completion_tokens INTEGER,
            total_tokens INTEGER,
            metadata TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建使用统计表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS usage_stats (
            id BIGSERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL REFERENCES users(id),
            model TEXT NOT NULL,
            request_date TEXT NOT NULL,
            request_count INTEGER DEFAULT 0,
            token_count INTEGER DEFAULT 0,
            cost_usd REAL DEFAULT 0.0,
            avg_response_time_ms REAL DEFAULT 0.0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(user_id, model, request_date)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建MCP会话记录表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS mcp_sessions (
            id BIGSERIAL PRIMARY KEY,
            session_id TEXT UNIQUE NOT NULL,
            user_id BIGINT NOT NULL,
            transport_type TEXT NOT NULL,
            client_info TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_activity_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建MCP工具调用记录表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS mcp_tool_calls (
            id BIGSERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL,
            session_id TEXT,
            tool_name TEXT NOT NULL,
            request_arguments TEXT,
            response_result TEXT,
            execution_time_ms INTEGER,
            status TEXT NOT NULL,
            error_message TEXT,
            transport_type TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建索引
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_key ON api_keys (api_key)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_conversations_user_id ON conversations (user_id)")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages (conversation_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_stats_user_date ON usage_stats (user_id, request_date)")
        .execute(pool)
        .await?;

    // 创建MCP相关索引
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_mcp_sessions_session_id ON mcp_sessions (session_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_sessions_user_id ON mcp_sessions (user_id)")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_mcp_tool_calls_session_id ON mcp_tool_calls (session_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_mcp_tool_calls_user_id ON mcp_tool_calls (user_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_mcp_tool_calls_tool_name ON mcp_tool_calls (tool_name)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_mcp_tool_calls_created_at ON mcp_tool_calls (created_at)",
    )
    .execute(pool)
    .await?;

    info!("数据库迁移完成");
    Ok(())
}

/// 获取默认数据库URL
pub fn get_default_database_url() -> String {
    // 云端 PostgreSQL 数据库
    "postgresql://llm:dckj@zndx@139.159.198.14:5432/llm".to_string()
}
