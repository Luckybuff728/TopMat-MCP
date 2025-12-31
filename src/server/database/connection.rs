use sqlx::{Pool, Sqlite, SqlitePool, sqlite::SqliteConnectOptions};
use std::str::FromStr;
use std::path::Path;
use tracing::{info, error};

/// 数据库连接池
#[derive(Clone)]
pub struct DatabaseConnection {
    pool: SqlitePool,
}

impl DatabaseConnection {
    /// 获取数据库连接池
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// 检查数据库连接是否健康
    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        let result = sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await;

        match result {
            Ok(_) => {
                info!("数据库连接健康检查通过");
                Ok(())
            }
            Err(e) => {
                error!("数据库连接健康检查失败: {}", e);
                Err(e)
            }
        }
    }
}

/// 初始化数据库连接
pub async fn init_database(database_url: &str) -> Result<DatabaseConnection, sqlx::Error> {
    info!("正在初始化数据库连接: {}", database_url);

    // 确保数据库目录存在
    if let Some(parent) = Path::new(database_url.strip_prefix("sqlite:").unwrap_or(database_url)).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| sqlx::Error::Io(e))?;
            info!("创建数据库目录: {:?}", parent);
        }
    }

    // 创建连接池，使用连接选项
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
    ).await?;

    info!("数据库连接池创建成功");

    // 运行数据库迁移
    run_migrations(&pool).await?;

    info!("数据库初始化完成");

    Ok(DatabaseConnection { pool })
}

/// 运行数据库迁移
async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    info!("正在运行数据库迁移...");

    // 创建用户表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            subscription_level TEXT NOT NULL DEFAULT 'free',
            subscription_expires_at TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建API密钥表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS api_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            api_key TEXT NOT NULL UNIQUE,
            key_name TEXT NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT 1,
            expires_at TEXT,
            last_used_at TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users (id)
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
            user_id INTEGER NOT NULL,
            title TEXT,
            model TEXT NOT NULL,
            message_count INTEGER DEFAULT 0,
            summary TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users (id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建消息表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            message_id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            model TEXT,
            prompt_tokens INTEGER,
            completion_tokens INTEGER,
            total_tokens INTEGER,
            metadata TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建使用统计表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS usage_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            model TEXT NOT NULL,
            request_date TEXT NOT NULL,
            request_count INTEGER DEFAULT 0,
            token_count INTEGER DEFAULT 0,
            cost_usd REAL DEFAULT 0.0,
            avg_response_time_ms REAL DEFAULT 0.0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users (id),
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
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT UNIQUE NOT NULL,
            user_id INTEGER NOT NULL,
            transport_type TEXT NOT NULL,  -- 'http' 或 'sse'
            client_info TEXT,  -- 客户端信息JSON
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_activity_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建MCP工具调用记录表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS mcp_tool_calls (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            session_id TEXT,
            tool_name TEXT NOT NULL,
            request_arguments TEXT,  -- JSON格式
            response_result TEXT,    -- JSON格式
            execution_time_ms INTEGER,
            status TEXT NOT NULL,  -- 'success', 'error', 'timeout'
            error_message TEXT,
            transport_type TEXT NOT NULL,  -- 'http' 或 'sse'
            endpoint TEXT NOT NULL,  -- '/mcp' 或 '/sse/mcp'
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
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

    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages (conversation_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_stats_user_date ON usage_stats (user_id, request_date)")
        .execute(pool)
        .await?;

    // 创建MCP相关索引
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_sessions_session_id ON mcp_sessions (session_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_sessions_user_id ON mcp_sessions (user_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tool_calls_session_id ON mcp_tool_calls (session_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tool_calls_user_id ON mcp_tool_calls (user_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tool_calls_tool_name ON mcp_tool_calls (tool_name)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tool_calls_created_at ON mcp_tool_calls (created_at)")
        .execute(pool)
        .await?;

    info!("数据库迁移完成");
    Ok(())
}

/// 获取默认数据库URL
pub fn get_default_database_url() -> String {
    // 获取当前工作目录的绝对路径
    let current_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let db_path = current_dir.join("data").join("topmat_llm.db");

    // 转换为绝对路径字符串
    let db_path_str = db_path.to_string_lossy();
    format!("sqlite:{}", db_path_str)
}