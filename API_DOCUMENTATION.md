# TopMat LLM 统一聊天服务器 API 文档

## 概述

TopMat LLM 统一聊天服务器提供标准化的 REST API 接口，支持多种 AI 模型的聊天对话功能。本文档详细描述了 API 的使用方法、请求格式、响应格式以及错误处理。

**基础信息：**
- 基础URL: `http://localhost:3000`
- API版本: v1
- 协议: HTTP/HTTPS
- 数据格式: JSON
- 当前版本: 1.3.0

**主要功能：**
- 多模型统一接口（通义千问、Ollama 本地模型）
- 流式和非流式响应
- 完整的会话管理和历史记录存储
- 基于API Key的用户认证和授权
- 实时使用统计和监控
- 健康检查和服务状态监控
- AI回复内容自动提取和存储

## 认证机制

所有需要认证的API请求都需要通过API Key进行鉴权。支持两种传递方式：

1. **Authorization Header** (推荐)
   ```
   Authorization: Bearer your_api_key_here
   ```

2. **X-API-Key Header**
   ```
   X-API-Key: your_api_key_here
   ```

认证中间件会自动验证API Key的有效性，并将用户信息注入到请求上下文中。

## 错误响应格式

所有错误响应都遵循统一格式：

```json
{
  "error": "error_type",
  "message": "错误描述信息",
  "details": {
    "additional_info": "额外错误信息"
  },
  "timestamp": "2024-10-27T10:30:00Z"
}
```

## API接口列表

| 接口 | 方法 | 路径 | 描述 | 鉴权 |
|------|------|------|------|------|
| 用户鉴权 | POST | `/v1/auth` | 验证API Key有效性 |      |
| 模型列表 | GET | `/v1/models` | 获取可用模型列表 | 无需 |
| 健康检查 | GET | `/health` | 服务健康状态检查 | 无需 |
| AI聊天 | POST | `/v1/chat` | AI对话服务 | 必需 |
| 使用统计 | GET | `/usage/stats` | 获取用户使用统计 | 必需 |
| 对话列表 | GET | `/v1/conversations` | 获取用户对话列表 | 必需 |
| 创建对话 | POST | `/v1/conversations` | 创建新对话 | 必需 |
| 获取对话 | GET | `/v1/conversations/:id` | 获取特定对话详情 | 必需 |
| 更新对话标题 | PUT | `/v1/conversations/:id/title` | 更新对话标题 | 必需 |
| 删除对话 | DELETE | `/v1/conversations/:id` | 删除对话及相关消息 | 必需 |
| 对话消息列表 | GET | `/v1/conversations/:id/messages` | 获取对话中的消息列表 | 必需 |
| 添加消息 | POST | `/v1/conversations/:id/messages` | 向对话添加新消息 | 必需 |
| 获取消息 | GET | `/v1/conversations/:id/messages/:message_id` | 获取特定消息详情 | 必需 |
| 删除消息 | DELETE | `/v1/conversations/:id/messages/:message_id` | 删除特定消息 | 必需 |

## API接口详情

### 1. 用户鉴权

验证API Key的有效性并获取用户信息。

**请求：**
```http
POST /v1/auth
Content-Type: application/json
Authorization: Bearer your_api_key_here
```

**响应：**
```json
{
    "api_key": {
        "expires_at": "2025-12-31T23:59:59Z",
        "key_name": "Test Mock API Key"
    },
    "message": "鉴权成功",
    "timestamp": "2025-10-28T07:15:24.133269500Z",
    "user": {
        "email": "test@example.com",
        "subscription_level": "pro",
        "username": "test_user"
    },
    "valid": true
}
```

### 2. 模型列表

获取当前可用的AI模型列表。

**请求：**
```http
GET /v1/models
```

**响应：**
```json
{
    "models": [
        {
            "cost_per_1k_tokens": 0.0,
            "description": "Ollama本地Qwen3 4B参数版本",
            "max_tokens": 4096,
            "name": "ollama-qwen3-4b",
            "provider": "ollama",
            "supports_streaming": true
        },
        {
            "cost_per_1k_tokens": 0.002,
            "description": "通义千问Plus，适合一般对话，质量较高",
            "max_tokens": 4000,
            "name": "qwen-plus",
            "provider": "qwen",
            "supports_streaming": true
        },
        {
            "cost_per_1k_tokens": 0.0,
            "description": "Ollama本地Llama3模型",
            "max_tokens": 4096,
            "name": "ollama-llama3",
            "provider": "ollama",
            "supports_streaming": true
        },
        {
            "cost_per_1k_tokens": 0.003,
            "description": "通义千问增强版，逻辑推理能力强",
            "max_tokens": 4000,
            "name": "qwq-plus",
            "provider": "qwen",
            "supports_streaming": true
        },
        {
            "cost_per_1k_tokens": 0.0015,
            "description": "通义千问Turbo，响应速度快，适合实时对话",
            "max_tokens": 4000,
            "name": "qwen-turbo",
            "provider": "qwen",
            "supports_streaming": true
        },
        {
            "cost_per_1k_tokens": 0.0005,
            "description": "通义千问Flash，极速响应，适合简单问答",
            "max_tokens": 2000,
            "name": "qwen-flash",
            "provider": "qwen",
            "supports_streaming": true
        },
        {
            "cost_per_1k_tokens": 0.008,
            "description": "通义千问Max，最高质量，适合复杂任务",
            "max_tokens": 8000,
            "name": "qwen-max",
            "provider": "qwen",
            "supports_streaming": true
        }
    ],
    "timestamp": "2025-10-28T07:16:47.828212700Z",
    "total": 7
}
```

### 3. 健康检查

检查服务器和各组件的健康状态。

**请求：**
```http
GET /health
```

**响应：**
```json
{
    "status": "healthy",
    "timestamp": "2025-10-28T07:17:38.806250900Z",
    "version": "1.3.0",
    "services": {
        "database": "healthy",
        "cache": "healthy",
        "ai_models": {
            "qwen-max": "healthy",
            "qwen-flash": "healthy",
            "qwen-plus": "healthy",
            "qwq-plus": "healthy",
            "ollama-qwen3-4b": "healthy",
            "ollama-llama3": "healthy",
            "qwen-turbo": "healthy"
        }
    }
}
```

### 4. AI聊天

与AI模型进行对话，支持流式和非流式响应。

**请求：**
```http
POST /v1/chat HTTP/1.1
Host: 127.0.0.1:8081
Content-Type: application/json
Authorization: Bearer your_api_key_here

{
    "conversation_id": 1,
    "message": "你好",
    "stream": true,
    "model": "ollama-qwen3-4b"
}
```

**请求参数：**
- `message` (string, 必需): 用户输入的消息
- `model` (string, 必选): 使用的模型名称
- `stream` (boolean, 可选): 是否使用流式响应，默认为 false
- `conversation_id` (integer, 必选): 会话ID，用于多轮对话，可从POST /v1/conversations创建新对话获取conversation_id
- `temperature` (float, 可选): 温度参数，控制回复随机性
- `max_tokens` (integer, 可选): 最大token数限制
- `system_prompt` (string, 可选): 系统提示词
- `metadata` (object, 可选): 额外的元数据

**非流式响应：**
```json
{
    "content": "你好！有什么可以帮你的吗？😊",
    "model": "ollama-qwen3-4b",
    "usage": {
        "prompt_tokens": 9,
        "completion_tokens": 180,
        "total_tokens": 189
    },
    "conversation_id": 1,
    "timestamp": "2025-10-28T07:21:50.918134100Z"
}
```

**流式响应：**
流式响应通过Server-Sent Events (SSE)格式返回：

```http
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive

data: {"type":"content","text":"人工","finished":false}

data: {"type":"content","text":"智能","finished":false}

data: {"type":"content","text":"的发展","finished":false}

data: {"type":"reasoning","reasoning":"用户想了解AI历史，需要提供准确的时间线"}

data: {"type":"final","response":{"content":"人工智能的发展历史...","model":"qwen-plus","usage":{"prompt_tokens":25,"completion_tokens":380,"total_tokens":405},"conversation_id":123,"timestamp":"2024-10-27T10:30:00Z"}}
```

### 5. 使用统计

获取用户的使用统计信息。

**请求：**
```http
GET /usage/stats?period=day&from_date=2024-10-01T00:00:00Z&to_date=2024-10-27T23:59:59Z
Authorization: Bearer your_api_key_here
```

**查询参数：**
- `period` (string, 可选): 统计周期 (day/week/month)
- `from_date` (string, 可选): 开始日期 (RFC3339格式)
- `to_date` (string, 可选): 结束日期 (RFC3339格式)

**响应：**
```json
{
  "period": "day",
  "from_date": "2024-10-01T00:00:00Z",
  "to_date": "2024-10-27T23:59:59Z",
  "stats": {
    "total_requests": 156,
    "total_tokens": 45680,
    "total_cost": 45.68,
    "avg_response_time_ms": 1250.5,
    "model_usage": {
      "qwen-plus": {
        "model": "qwen-plus",
        "requests": 98,
        "tokens": 32450,
        "cost": 32.45
      },
      "ollama-qwen3-4b": {
        "model": "ollama-qwen3-4b",
        "requests": 58,
        "tokens": 13230,
        "cost": 13.23
      }
    }
  }
}
```

### 6. 对话管理

#### 6.1 获取对话列表

```http
GET /v1/conversations
Authorization: Bearer your_api_key_here
```

**响应：**
```json
{
    "conversations": [
        {
            "conversation_id": 1,
            "user_id": 1,
            "title": null,
            "model": "qwen-plus",
            "message_count": 9,
            "summary": null,
            "created_at": "2025-10-28T06:42:06Z",
            "updated_at": "2025-10-28T07:21:51Z"
        },
        {
            "conversation_id": 4,
            "user_id": 1,
            "title": "你好...",
            "model": "qwen-plus",
            "message_count": 0,
            "summary": null,
            "created_at": "2025-10-28T07:06:01Z",
            "updated_at": "2025-10-28T07:06:01Z"
        },
        {
            "conversation_id": 3,
            "user_id": 1,
            "title": "你好...",
            "model": "qwen-plus",
            "message_count": 0,
            "summary": null,
            "created_at": "2025-10-28T07:05:35Z",
            "updated_at": "2025-10-28T07:05:35Z"
        },
        {
            "conversation_id": 2,
            "user_id": 1,
            "title": "你好...",
            "model": "qwen-plus",
            "message_count": 0,
            "summary": null,
            "created_at": "2025-10-28T06:46:08Z",
            "updated_at": "2025-10-28T06:46:08Z"
        }
    ],
    "total": 4,
    "page": 1,
    "page_size": 20,
    "total_pages": 1
}
```

#### 6.2 创建新对话

```http
POST /v1/conversations
Content-Type: application/json
Authorization: Bearer your_api_key_here

{
  "title": "新的对话主题",
  "model": "qwen-plus"
}
```

**响应：**
```json
{
    "conversation": {
        "conversation_id": 5,
        "user_id": 1,
        "title": "新的对话主题",
        "model": "qwen-plus",
        "message_count": 0,
        "summary": null,
        "created_at": "2025-10-28T07:26:24Z",
        "updated_at": "2025-10-28T07:26:24Z"
    },
    "first_message": null
}
```

#### 6.3 获取对话详情

```http
GET /v1/conversations/2/messages
Authorization: Bearer your_api_key_here
```

**响应：**

```json
{
    "messages": [
        {
            "id": 0,
            "conversation_id": 1,
            "role": "user",
            "content": "你好",
            "model": "ollama-qwen3-4b",
            "usage": null,
            "metadata": null,
            "created_at": "2025-10-28T06:42:17Z"
        },
        {
            "id": 0,
            "conversation_id": 1,
            "role": "assistant",
            "content": "你好呀！有什么我可以帮忙的吗？😊",
            "model": "ollama-qwen3-4b",
            "usage": {
                "prompt_tokens": 9,
                "completion_tokens": 242,
                "total_tokens": 251
            },
            "metadata": null,
            "created_at": "2025-10-28T06:42:20Z"
        }
    ],
    "conversation_id": 1,
    "total": 2,
    "page": 1,
    "page_size": 50,
    "total_pages": 1,
    "has_more": false
}
```
#### 6.4 更新对话标题

```http
PUT /v1/conversations/1/title
Authorization: Bearer your_api_key_here

{
  "title": "更新后的对话标题"
}
```

**响应：**
```json
{
    "conversation_id": 1,
    "user_id": 1,
    "title": "更新后的对话标题",
    "model": "qwen-plus",
    "message_count": 9,
    "summary": null,
    "created_at": "2025-10-28T06:42:06Z",
    "updated_at": "2025-10-28T07:33:48Z"
}
```

#### 6.5 删除对话

```http
DELETE /v1/conversations/123
Authorization: Bearer your_api_key_here
```

**响应：**
```json
{
    "conversation_id": 2,
    "deleted_messages_count": 0,
    "message": "对话删除成功",
    "success": true,
    "timestamp": "2025-10-28T07:37:59.948272400Z"
}
```

### 7. 消息管理

#### 7.1 获取对话消息列表

```http
GET /v1/conversations/1/messages
Authorization: Bearer your_api_key_here
```

**响应：**
```json
```json
{
    "messages": [
        {
            "id": 0,
            "conversation_id": 1,
            "role": "user",
            "content": "你好",
            "model": "ollama-qwen3-4b",
            "usage": null,
            "metadata": null,
            "created_at": "2025-10-28T06:42:17Z"
        },
        {
            "id": 0,
            "conversation_id": 1,
            "role": "assistant",
            "content": "你好呀！有什么我可以帮忙的吗？😊",
            "model": "ollama-qwen3-4b",
            "usage": {
                "prompt_tokens": 9,
                "completion_tokens": 242,
                "total_tokens": 251
            },
            "metadata": null,
            "created_at": "2025-10-28T06:42:20Z"
        }
    ],
    "conversation_id": 1,
    "total": 2,
    "page": 1,
    "page_size": 50,
    "total_pages": 1,
    "has_more": false
}
```



#### 7.2 获取消息详情

```http
GET /v1/conversations/123/messages/456
Authorization: Bearer your_api_key_here
```

**响应：**
```json
{
    "id": 0,
    "conversation_id": 1,
    "role": "user",
    "content": "你好",
    "model": "ollama-qwen3-4b",
    "usage": null,
    "metadata": null,
    "created_at": "2025-10-28T06:42:17Z"
}
```

#### 7.3 删除消息

```http
DELETE /v1/conversations/123/messages/456
Authorization: Bearer your_api_key_here
```

**响应：**
```json
{
    "conversation_id": 1,
    "message": "消息删除成功",
    "message_id": 6,
    "success": true,
    "timestamp": "2025-10-28T07:42:55.913242800Z"
}
```

## 数据模型

### 对话 (Conversation)

```json
{
  "conversation_id": 123,
  "user_id": 456,
  "title": "对话标题",
  "model": "qwen-plus",
  "message_count": 10,
  "created_at": "2024-10-27T10:30:00Z",
  "updated_at": "2024-10-27T11:45:00Z"
}
```

### 消息 (Message)

```json
{
  "message_id": 789,
  "conversation_id": 123,
  "role": "user|assistant",
  "content": "消息内容",
  "model": "qwen-plus",
  "prompt_tokens": 25,
  "completion_tokens": 150,
  "total_tokens": 175,
  "created_at": "2024-10-27T10:30:00Z"
}
```

## 支持的AI模型

### 通义千问模型

| 模型ID | 名称 | 描述 | 适用场景 |
|--------|------|------|----------|
| `qwen-plus` | 通义千问 Plus | 平衡性能和成本 | 通用对话、文本生成 |
| `qwen-turbo` | 通义千问 Turbo | 快速响应 | 简单问答、实时交互 |
| `qwen-max` | 通义千问 Max | 最强性能 | 复杂推理、专业领域 |
| `qwen-flash` | 通义千问 Flash | 超快响应 | 轻量级任务 |
| `qwq-plus` | 通义千问 qwq Plus | 推理增强 | 数学、逻辑推理 |

### Ollama本地模型

| 模型ID | 名称 | 描述 | 系统要求 |
|--------|------|------|----------|
| `ollama-qwen3-4b` | Qwen3 4B | 轻量级本地模型 | 4GB+ RAM |
| `ollama-llama3` | Llama3 | Meta开源模型 | 8GB+ RAM |

## 错误代码

| 错误代码 | HTTP状态码 | 描述 |
|----------|------------|------|
| `auth_required` | 401 | 需要API Key认证 |
| `auth_invalid` | 401 | API Key无效或已过期 |
| `auth_failed` | 401 | 认证失败 |
| `model_not_supported` | 400 | 不支持的模型 |
| `qwen_not_configured` | 503 | 通义千问未配置 |
| `ollama_not_available` | 503 | Ollama服务不可用 |
| `conversation_not_found` | 404 | 对话不存在 |
| `message_not_found` | 404 | 消息不存在 |
| `access_denied` | 403 | 访问被拒绝 |
| `database_error` | 500 | 数据库错误 |
| `internal_error` | 500 | 内部服务器错误 |

## 配置说明

### 环境变量

| 变量名 | 描述 | 默认值 | 必需 |
|--------|------|--------|------|
| `SERVER_HOST` | 服务器监听地址 | 127.0.0.1 | 否 |
| `SERVER_PORT` | 服务器端口 | 3000 | 否 |
| `DATABASE_URL` | 数据库连接URL | sqlite:data.db | 否 |
| `DASHSCOPE_API_KEY` | 通义千问API密钥 | - | 是(使用通义模型) |
| `OLLAMA_BASE_URL` | Ollama服务地址 | http://localhost:11434 | 否 |
| `AUTH_API_URL` | 认证服务地址 | https://api.topmaterial-tech.com | 否 |
| `RUST_LOG` | 日志级别 | info | 否 |

### 数据库配置

系统使用SQLite作为默认数据库，数据库表会在启动时自动创建：

- `conversations`: 对话表
- `messages`: 消息表

## 开发指南

### 测试API

```bash
# 测试健康检查
curl http://localhost:3000/health

# 测试模型列表
curl http://localhost:3000/v1/models

# 测试认证
curl -X POST http://localhost:3000/v1/auth \
  -H "Authorization: Bearer your_api_key"

# 测试聊天
curl -X POST http://localhost:3000/v1/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key" \
  -d '{
    "message": "你好",
    "model": "qwen-plus",
    "stream": false
  }'
```

### 流式响应示例

JavaScript客户端处理流式响应：

```javascript
const response = await fetch('/v1/chat', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer your_api_key'
  },
  body: JSON.stringify({
    message: '你好',
    model: 'qwen-plus',
    stream: true
  })
});

const reader = response.body.getReader();
const decoder = new TextDecoder();

while (true) {
  const { done, value } = await reader.read();
  if (done) break;

  const chunk = decoder.decode(value);
  const lines = chunk.split('\n');

  for (const line of lines) {
    if (line.startsWith('data: ')) {
      const data = JSON.parse(line.slice(6));
      console.log('收到数据:', data);
    }
  }
}
```

## 版本更新日志

### v1.3.0 (2024-10-27)
- 新增：完整的对话管理功能
- 新增：消息历史记录存储
- 新增：使用统计API
- 新增：健康检查API
- 优化：AI回复内容自动提取和存储
- 修复：数据库字段命名优化

### v1.2.0
- 新增：通义千问模型支持
- 新增：Ollama本地模型支持
- 优化：流式响应处理

### v1.1.0
- 新增：用户认证系统
- 新增：API Key管理

### v1.0.0
- 初始版本发布
- 基础聊天功能

---

**文档更新时间：** 2024-10-27
**API版本：** v1.3.0