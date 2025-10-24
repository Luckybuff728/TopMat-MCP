# TopMat-LLM 数据库设计文档

## 概述

本文档描述了 TopMat-LLM 系统的数据库设计，用于保存用户信息、聊天记录、API使用统计等数据。

## 数据库选择

- **主数据库**: PostgreSQL 14+
- **缓存**: Redis 6+
- **连接池**: SQLx
- **ORM**: SQLx (类型安全的异步SQL)

## 表结构设计

### 1. 用户相关表

#### users - 用户基本信息表
```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    external_id INTEGER NOT NULL UNIQUE,           -- 外部鉴权系统的用户ID
    username VARCHAR(100) NOT NULL,
    email VARCHAR(255) NOT NULL,
    subscription_level VARCHAR(50) NOT NULL,       -- basic, pro, enterprise
    subscription_expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    INDEX idx_users_external_id (external_id),
    INDEX idx_users_subscription_level (subscription_level),
    INDEX idx_users_subscription_expires (subscription_expires_at)
);
```

#### user_sessions - 用户会话表
```sql
CREATE TABLE user_sessions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id VARCHAR(255) NOT NULL UNIQUE,      -- 前端传入的session_id
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_activity_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true,

    INDEX idx_sessions_user_id (user_id),
    INDEX idx_sessions_session_id (session_id),
    INDEX idx_sessions_last_activity (last_activity_at)
);
```

### 2. 聊天相关表

#### conversations - 对话表
```sql
CREATE TABLE conversations (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id VARCHAR(255),
    title VARCHAR(500),                           -- 可选的对话标题
    model VARCHAR(100) NOT NULL,                  -- 使用的AI模型
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    INDEX idx_conversations_user_id (user_id),
    INDEX idx_conversations_session_id (session_id),
    INDEX idx_conversations_model (model),
    INDEX idx_conversations_created_at (created_at)
);
```

#### messages - 消息表
```sql
CREATE TABLE messages (
    id SERIAL PRIMARY KEY,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL,                    -- 'user' | 'assistant' | 'system'
    content TEXT NOT NULL,
    model VARCHAR(100),                           -- 仅当role='assistant'时有值
    metadata JSONB,                               -- 额外的元数据
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    INDEX idx_messages_conversation_id (conversation_id),
    INDEX idx_messages_role (role),
    INDEX idx_messages_created_at (created_at)
);
```

### 3. API使用统计表

#### api_usage - API使用记录表
```sql
CREATE TABLE api_usage (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id INTEGER,                            -- 外部API Key的ID
    endpoint VARCHAR(100) NOT NULL,                -- '/chat', '/auth'
    method VARCHAR(10) NOT NULL,                   -- 'POST', 'GET'
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER,                      -- 响应时间（毫秒）
    request_size INTEGER,                          -- 请求大小（字节）
    response_size INTEGER,                         -- 响应大小（字节）
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    INDEX idx_api_usage_user_id (user_id),
    INDEX idx_api_usage_endpoint (endpoint),
    INDEX idx_api_usage_status_code (status_code),
    INDEX idx_api_usage_created_at (created_at)
);
```

#### token_usage - Token使用统计表
```sql
CREATE TABLE token_usage (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id INTEGER REFERENCES conversations(id) ON DELETE CASCADE,
    message_id INTEGER REFERENCES messages(id) ON DELETE CASCADE,
    model VARCHAR(100) NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    total_tokens INTEGER NOT NULL,
    cost_usd DECIMAL(10, 6),                       -- 成本（美元）
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    INDEX idx_token_usage_user_id (user_id),
    INDEX idx_token_usage_conversation_id (conversation_id),
    INDEX idx_token_usage_model (model),
    INDEX idx_token_usage_created_at (created_at)
);
```

### 4. 系统管理表

#### api_keys - API Key管理表
```sql
CREATE TABLE api_keys (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    external_api_key_id INTEGER UNIQUE,            -- 外部系统的API Key ID
    key_name VARCHAR(200) NOT NULL,
    key_hash VARCHAR(255) NOT NULL,                -- API Key的哈希值（不存储明文）
    is_active BOOLEAN DEFAULT true,
    expires_at TIMESTAMP WITH TIME ZONE,
    last_used_at TIMESTAMP WITH TIME ZONE,
    usage_count INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    INDEX idx_api_keys_user_id (user_id),
    INDEX idx_api_keys_external_id (external_api_key_id),
    INDEX idx_api_keys_hash (key_hash),
    INDEX idx_api_keys_last_used (last_used_at)
);
```

#### system_logs - 系统日志表
```sql
CREATE TABLE system_logs (
    id SERIAL PRIMARY KEY,
    level VARCHAR(20) NOT NULL,                    -- DEBUG, INFO, WARN, ERROR
    message TEXT NOT NULL,
    context JSONB,                                -- 上下文信息
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    session_id VARCHAR(255),
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    INDEX idx_system_logs_level (level),
    INDEX idx_system_logs_user_id (user_id),
    INDEX idx_system_logs_created_at (created_at)
);
```

## /chat 接口的数据库集成

### 请求处理流程

1. **鉴权阶段**
   ```sql
   -- 1. 查找或创建用户记录
   INSERT INTO users (external_id, username, email, subscription_level, subscription_expires_at)
   VALUES ($1, $2, $3, $4, $5)
   ON CONFLICT (external_id)
   DO UPDATE SET
       username = EXCLUDED.username,
       email = EXCLUDED.email,
       subscription_level = EXCLUDED.subscription_level,
       subscription_expires_at = EXCLUDED.subscription_expires_at,
       updated_at = NOW()
   RETURNING id;

   -- 2. 记录API使用
   INSERT INTO api_usage (user_id, endpoint, method, status_code, response_time_ms)
   VALUES ($1, '/chat', 'POST', 200, $2);
   ```

2. **会话管理**
   ```sql
   -- 1. 查找或创建会话
   SELECT id FROM user_sessions
   WHERE user_id = $1 AND session_id = $2 AND is_active = true;

   -- 2. 如果会话不存在，创建新会话
   INSERT INTO user_sessions (user_id, session_id)
   VALUES ($1, $2)
   ON CONFLICT (session_id)
   DO UPDATE SET last_activity_at = NOW();
   ```

3. **对话保存**
   ```sql
   -- 1. 创建或更新对话
   INSERT INTO conversations (user_id, session_id, model)
   VALUES ($1, $2, $3)
   ON CONFLICT (user_id, session_id)
   DO UPDATE SET updated_at = NOW()
   RETURNING id;

   -- 2. 保存用户消息
   INSERT INTO messages (conversation_id, role, content)
   VALUES ($1, 'user', $2);

   -- 3. 保存AI回复
   INSERT INTO messages (conversation_id, role, content, model, metadata)
   VALUES ($1, 'assistant', $2, $3, $4);

   -- 4. 记录Token使用
   INSERT INTO token_usage (user_id, conversation_id, message_id, model, prompt_tokens, completion_tokens, total_tokens)
   VALUES ($1, $2, $3, $4, $5, $6, $7);
   ```

### 更新后的 /chat 接口请求格式

```json
{
  "message": "string",              // 用户消息
  "stream": boolean,              // 是否流式响应
  "model": "string",              // AI模型
  "system_prompt": "string",      // 系统提示词
  "temperature": number,          // 温度参数
  "max_tokens": number,           // 最大token数
  "session_id": "string",         // 会话ID
  "conversation_id": "string",    // 对话ID（可选）
  "save_history": boolean,        // 是否保存聊天历史
  "metadata": {}                  // 额外元数据
}
```

### 更新后的 /chat 接口响应格式

```json
{
  "content": "string",             // AI回复内容
  "model": "string",               // 使用的模型
  "conversation_id": "string",      // 对话ID
  "session_id": "string",          // 会话ID
  "message_id": "string",          // 消息ID
  "usage": {                       // Token使用情况
    "prompt_tokens": number,
    "completion_tokens": number,
    "total_tokens": number,
    "cost_usd": number
  },
  "created_at": "2024-01-01T00:00:00Z",
  "metadata": {}                   // 额外元数据
}
```

## 数据访问层 (DAL) 设计

### Rust 结构体设计

```rust
// 用户相关
pub struct User {
    pub id: i32,
    pub external_id: i32,
    pub username: String,
    pub email: String,
    pub subscription_level: String,
    pub subscription_expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

// 对话相关
pub struct Conversation {
    pub id: i32,
    pub user_id: i32,
    pub session_id: Option<String>,
    pub title: Option<String>,
    pub model: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct Message {
    pub id: i32,
    pub conversation_id: i32,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// 统计相关
pub struct TokenUsage {
    pub id: i32,
    pub user_id: i32,
    pub conversation_id: Option<i32>,
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost_usd: Option<rust_decimal::Decimal>,
}
```

### 数据库操作接口

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_or_create_user(&self, external_user: &ExternalUserInfo) -> Result<User, DbError>;
    async fn get_user_by_id(&self, id: i32) -> Result<Option<User>, DbError>;
    async fn update_user_subscription(&self, user_id: i32, level: &str, expires_at: chrono::DateTime<chrono::Utc>) -> Result<(), DbError>;
}

#[async_trait]
pub trait ConversationRepository: Send + Sync {
    async fn create_conversation(&self, user_id: i32, session_id: Option<&str>, model: &str) -> Result<Conversation, DbError>;
    async fn get_conversation_by_id(&self, id: i32) -> Result<Option<Conversation>, DbError>;
    async fn get_conversations_by_user(&self, user_id: i32, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Conversation>, DbError>;
    async fn add_message(&self, conversation_id: i32, role: &str, content: &str, model: Option<&str>, metadata: Option<serde_json::Value>) -> Result<Message, DbError>;
    async fn get_conversation_messages(&self, conversation_id: i32, limit: Option<i64>) -> Result<Vec<Message>, DbError>;
}

#[async_trait]
pub trait UsageRepository: Send + Sync {
    async fn record_api_usage(&self, user_id: i32, endpoint: &str, method: &str, status_code: i32, response_time_ms: i64) -> Result<(), DbError>;
    async fn record_token_usage(&self, usage: &TokenUsage) -> Result<(), DbError>;
    async fn get_user_usage_stats(&self, user_id: i32, from_date: chrono::DateTime<chrono::Utc>, to_date: chrono::DateTime<chrono::Utc>) -> Result<UsageStats, DbError>;
}
```

## 环境配置

### 数据库连接配置

```bash
# PostgreSQL 配置
DATABASE_URL=postgresql://username:password@localhost:5432/topmat_llm
DATABASE_POOL_SIZE=10
DATABASE_MIN_CONNECTIONS=2

# Redis 配置
REDIS_URL=redis://localhost:6379/0

# Token 成本配置（按模型）
COST_QWEN_PLUS_PER_1K_TOKENS=0.0020
COST_QWEN_TURBO_PER_1K_TOKENS=0.0015
COST_OLLAMA_PER_1K_TOKENS=0.0000
```

## 数据迁移策略

1. **初始迁移**: 创建所有表结构
2. **版本控制**: 使用sqlx migrate进行数据库版本管理
3. **数据备份**: 定期备份用户数据
4. **数据清理**: 定期清理过期的日志和临时数据

## 性能优化

1. **索引策略**: 为常用查询字段添加索引
2. **分区表**: 对大表（如messages）按时间分区
3. **缓存策略**: 使用Redis缓存热点数据
4. **连接池**: 合理配置数据库连接池大小
5. **异步处理**: 使用异步操作提高并发性能