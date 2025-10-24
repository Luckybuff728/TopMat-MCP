# TopMat-LLM API 接口文档

## 概述

TopMat-LLM 提供统一的AI聊天服务API，支持多种AI模型，包含用户鉴权、对话管理、数据持久化等功能。

**基础信息**
- **Base URL**: `http://localhost:3000`
- **API版本**: v1
- **数据格式**: JSON
- **字符编码**: UTF-8
- **时区**: UTC

## 认证机制

所有API请求都需要通过API Key进行鉴权。支持三种传递方式：

1. **Authorization Header** (推荐)
   ```
   Authorization: Bearer your_api_key_here
   ```

2. **X-API-Key Header**
   ```
   X-API-Key: your_api_key_here
   ```

## 错误响应格式

所有错误响应都遵循统一格式：

```json
{
  "error": "error_type",
  "message": "错误描述信息",
  "details": {
    "additional_info": "额外错误信息"
  },
  "timestamp": "2024-10-23T10:30:00Z"
}
```

## API接口列表

| 接口 | 方法 | 路径 | 描述 | 鉴权 |
|------|------|------|------|------|
| 用户鉴权 | POST | `/v1/auth` | 验证API Key有效性 | 必需 |
| 模型列表 | GET | `/v1/models` | 获取可用模型列表 | 无需 |
| AI聊天 | POST | `/v1/chat` | AI对话服务 | 必需 |
| 对话列表 | GET | `/v1/conversations` | 获取用户对话列表 | 必需 |
| 创建对话 | POST | `/v1/conversations` | 创建新对话 | 必需 |
| 对话详情 | GET | `/v1/conversations/{id}` | 获取对话详情 | 必需 |
| 更新标题 | PUT | `/v1/conversations/{id}/title` | 更新对话标题 | 必需 |
| 删除对话 | DELETE | `/v1/conversations/{id}` | 删除对话 | 必需 |
| 消息列表 | GET | `/v1/conversations/{id}/messages` | 获取对话消息 | 必需 |
| 添加消息 | POST | `/v1/conversations/{id}/messages` | 添加新消息 | 必需 |
| 消息详情 | GET | `/v1/conversations/{id}/messages/{message_id}` | 获取消息详情 | 必需 |
| 删除消息 | DELETE | `/v1/conversations/{id}/messages/{message_id}` | 删除消息 | 必需 |
| 使用统计 | GET | `/usage/stats` | 获取用户使用统计 | 无需 |
| 健康检查 | GET | `/health` | 检查服务健康状态 | 无需 |

## 1. 用户鉴权接口

### POST /v1/auth

验证用户API Key的有效性并返回用户信息。

**请求说明**
- **功能**: 验证API Key，获取用户信息
- **鉴权**: 需要提供有效的API Key
- **幂等性**: 是

**请求头**
```
Content-Type: application/json
Authorization: Bearer your_api_key_here
```

**请求体**
```json
{
  "device_info": {          // 可选，设备信息
    "user_agent": "string",
    "ip_address": "string",
    "device_id": "string"
  }
}
```

**请求示例**

```bash
curl -X POST http://localhost:3000/auth \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer tk_abc123def456ghi789" \
  -d '{
    "device_info": {
      "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
      "ip_address": "192.168.1.100"
    }
  }'
```

**成功响应 (200 OK)**
```json
{
  "status": "success",
  "message": "鉴权成功",
  "data": {
    "user": {
      "id": 1,
      "username": "johndoe",
      "email": "john.doe@example.com",
      "subscription_level": "pro",
      "subscription_expires_at": "2025-12-31T23:59:59Z"
    },
    "api_key": {
      "id": 123,
      "key_name": "Production API Key",
      "expires_at": "2025-12-31T23:59:59Z",
      "last_used_at": "2024-10-23T10:30:00Z"
    }
  },
  "timestamp": "2024-10-23T10:30:00Z"
}
```

**错误响应**

| 状态码 | 错误类型 | 描述 | 示例 |
|--------|----------|------|------|
| 401 | `missing_api_key` | 请求中缺少API Key | API Key is required |
| 401 | `invalid_api_key` | API Key无效或不存在 | Invalid API Key |
| 401 | `expired_api_key` | API Key已过期 | API Key has expired |
| 403 | `inactive_api_key` | API Key未激活 | API Key is not active |
| 403 | `subscription_expired` | 用户订阅已过期 | User subscription has expired |
| 503 | `auth_service_error` | 鉴权服务不可用 | Authentication service temporarily unavailable |

**错误响应示例**
```json
{
  "error": "invalid_api_key",
  "message": "API Key无效或不存在",
  "details": {
    "auth_error": "Invalid API Key"
  },
  "timestamp": "2024-10-23T10:30:00Z"
}
```

## 2. 模型列表接口

### GET /v1/models

获取当前系统中所有可用的AI模型列表及其详细信息。

**请求说明**
- **功能**: 获取所有可用模型的详细信息
- **鉴权**: 无需鉴权（公开接口）
- **幂等性**: 是

**请求头**
```
Accept: application/json
```

**请求示例**

```bash
curl -X GET http://localhost:3000/v1/models \
  -H "Accept: application/json"
```

**成功响应 (200 OK)**
```json
{
  "models": [
    {
      "name": "qwen-plus",
      "provider": "qwen",
      "description": "通义千问Plus，适合一般对话，质量较高",
      "supports_streaming": true,
      "max_tokens": 4000,
      "cost_per_1k_tokens": 0.0020
    },
    {
      "name": "qwen-turbo",
      "provider": "qwen",
      "description": "通义千问Turbo，响应速度快，适合实时对话",
      "supports_streaming": true,
      "max_tokens": 4000,
      "cost_per_1k_tokens": 0.0015
    },
    {
      "name": "qwen-max",
      "provider": "qwen",
      "description": "通义千问Max，最高质量，适合复杂任务",
      "supports_streaming": true,
      "max_tokens": 8000,
      "cost_per_1k_tokens": 0.0080
    },
    {
      "name": "qwen-flash",
      "provider": "qwen",
      "description": "通义千问Flash，极速响应，适合简单问答",
      "supports_streaming": true,
      "max_tokens": 2000,
      "cost_per_1k_tokens": 0.0005
    },
    {
      "name": "qwq-plus",
      "provider": "qwen",
      "description": "通义千问增强版，逻辑推理能力强",
      "supports_streaming": true,
      "max_tokens": 4000,
      "cost_per_1k_tokens": 0.0030
    },
    {
      "name": "ollama-qwen3-4b",
      "provider": "ollama",
      "description": "Ollama本地Qwen3 4B参数版本",
      "supports_streaming": true,
      "max_tokens": 4096,
      "cost_per_1k_tokens": 0.0000
    },
    {
      "name": "ollama-llama3",
      "provider": "ollama",
      "description": "Ollama本地Llama3模型",
      "supports_streaming": true,
      "max_tokens": 4096,
      "cost_per_1k_tokens": 0.0000
    }
  ],
  "total": 7,
  "timestamp": "2024-10-23T10:30:00Z"
}
```

**响应字段说明**

| 字段 | 类型 | 说明 |
|------|------|------|
| `models` | Array | 模型列表数组 |
| `total` | Integer | 模型总数 |
| `timestamp` | String | 响应时间戳 (ISO 8601格式) |

**模型对象字段说明**

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | String | 模型名称，用于在chat接口中指定 |
| `provider` | String | 模型提供商 (qwen/ollama) |
| `description` | String | 模型描述 |
| `supports_streaming` | Boolean | 是否支持流式响应 |
| `max_tokens` | Integer | 最大token数量限制 |
| `cost_per_1k_tokens` | Number | 每1k token的成本 (美元) |

**使用场景**

1. **前端模型选择器**: 为用户提供模型选择界面
2. **成本估算**: 根据模型和token使用量估算成本
3. **功能检测**: 检查特定功能是否可用（如流式响应）
4. **配置验证**: 验证模型配置是否正确

## 3. AI聊天接口

### POST /v1/chat

与AI模型进行对话，支持流式和非流式响应。

**请求说明**
- **功能**: AI对话服务
- **鉴权**: 需要有效的API Key
- **幂等性**: 否（每次请求可能产生不同的回复）

**请求头**
```
Content-Type: application/json
Authorization: Bearer your_api_key_here
```

**请求体**
```json
{
  "session_id": "string",         // 可选，会话ID，用于多轮对话
  "message_id": "string",         // 可选，对话ID，用于继续特定对话
  "model": "string",              // 可选，AI模型名称，默认"qwen-plus"
  "messages": [{
    "role": "user",                // 角色, 默认"user",其他有"system""assistant"
    "content": "string"            // 内容
    }],
  "stream": boolean,              // 可选，是否流式响应，默认false
  "temperature": number,          // 可选，温度参数 (0.0-2.0)，默认0.7
  "max_tokens": number,           // 可选，最大生成token数
  "metadata": {                   // 可选，额外元数据
  }
}
```

**参数说明**

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
|session_id|string|否|-|会话ID|
|message_id|string|否|qwen-plus|消息ID|
|model|string|是|-|模型名称|
|role|string|是|user|角色名称|
|content|string|是|-|内容|
|stream|boolean|否|false|是否流式返回|
|temperature|float|否|0.8|温度，取值范围[0.0, 1.0]|
|max_tokens|int|否|-|最大返回字符数|
|metadata|object|否|-|元数据|


**支持的模型**

系统支持多种AI模型，分为云端模型和本地模型两类。详细的模型列表和配置信息可以通过 `/v1/models` 接口获取。

**模型分类**:
- **云端模型**: 通义千问系列 (qwen-plus, qwen-turbo, qwen-max, qwen-flash, qwq-plus)
- **本地模型**: Ollama本地部署模型 (ollama-qwen3-4b, ollama-llama3)

**获取最新模型信息**:
```bash
# 获取所有可用模型及详细配置
curl http://localhost:3000/v1/models
```

**注意**: 模型列表可能会动态变化，建议在实际使用时通过 `/v1/models` 接口获取最新的模型信息。


**非流式请求示例**

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer tk_abc123def456ghi789" \
  -d '{
    "model": "qwq-plus",
    messages: [{
      "role": "user",
      "content": "介绍一下rust语言"
    }],
    "stream": false
  }'
```

**非流式成功响应 (200 OK)**
```json
{: "qwen-plus",
  "content": "Rust是一种系统编程语言，由Mozilla研究开发。它专注于性能、并发和内存安全...",
  "model"
  "conversation_id": "conv_67890",
  "session_id": "session_12345",
  "message_id": "msg_abc123",
  "usage": {
    "prompt_tokens": 25,
    "completion_tokens": 156,
    "total_tokens": 181,
    "cost_usd": 0.000362
  },
  "created_at": "2024-10-23T10:30:00Z",
  "metadata": {
    "response_time_ms": 1250,
    "model_provider": "qwen"
  }
}
```

**流式请求示例**

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer tk_abc123def456ghi789" \
  -d '{
    "model": "qwen-plus"
    "messages": [{
      "role": "user",
      "content": "写一个Python的Hello World程序"
    }],
    "stream": true
  }'
```

**流式响应格式**

每个事件都采用Server-Sent Events (SSE)格式：

```http
HTTP/1.1 200 OK
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive

data: {"type":"content","text":"以下是","finished":false}

data: {"type":"content","text":"一个简单","finished":false}

data: {"type":"content","text":"的Python","finished":false}

data: {"type":"content","text":"Hello World程序：\n\n```python\nprint(\"Hello, World!\")\n```\n\n这个程序会输出","finished":false}

data: {"type":"final","response":{"content":"以下是简单的Python Hello World程序：\n\n```python\nprint(\"Hello, World!\")\n```\n\n这个程序会输出 \"Hello, World!\" 到控制台。","model":"qwen-plus","conversation_id":"conv_67890","session_id":"session_12345","message_id":"msg_def456","usage":{"prompt_tokens":20,"completion_tokens":45,"total_tokens":65,"cost_usd":0.00013},"created_at":"2024-10-23T10:30:00Z"}}
```

**流式事件类型**

| 类型 | 描述 |
|------|------|
| `content` | 文本内容块，部分回复内容 |
| `reasoning` | 推理过程（如果模型支持） |
| `tool_call` | 工具调用（如果使用工具） |
| `error` | 错误信息 |
| `final` | 最终完整响应 |

**错误响应**

| 状态码 | 错误类型 | 描述 | 示例 |
|--------|----------|------|------|
| 400 | `invalid_request` | 请求格式无效 | JSON parsing failed |
| 400 | `model_not_supported` | 不支持的模型 | Model xxx is not supported |
| 401 | `missing_api_key` | 请求中缺少API Key | API Key is required |
| 401 | `invalid_api_key` | API Key无效 | Invalid API Key |
| 402 | `quota_exceeded` | 超出配额限制 | Daily quota exceeded |
| 413 | `message_too_long` | 消息内容过长 | Message exceeds maximum length |
| 429 | `rate_limit_exceeded` | 请求频率过高 | Rate limit exceeded |
| 500 | `chat_failed` | 聊天处理失败 | Model processing failed |
| 503 | `service_unavailable` | 服务暂时不可用 | AI service temporarily unavailable |

**错误响应示例**
```json
{
  "error": "model_not_supported",
  "message": "不支持的模型: unknown-model",
  "details": {
    "available_models": ["qwen-plus", "qwen-turbo", "qwen-max", "ollama-qwen3-4b"]
  },
  "timestamp": "2024-10-23T10:30:00Z"
}
```


## 4. 对话历史管理

TopMat-LLM 提供完整的对话历史管理功能，支持对话的创建、查询、更新和删除，以及消息的管理。这些接口允许用户构建完整的对话历史记录和上下文管理。

### 4.1 对话管理

#### GET /v1/conversations

获取用户的对话列表，支持分页、筛选和搜索。

**请求参数 (Query)**:
- `limit` (integer, 可选): 分页大小，默认20，最大100
- `offset` (integer, 可选): 偏移量，默认0
- `session_id` (string, 可选): 按会话ID筛选
- `search` (string, 可选): 搜索关键词

**请求示例**:
```bash
curl -X GET "http://localhost:3000/v1/conversations?limit=10&offset=0&session_id=session_123" \
  -H "Authorization: Bearer your_api_key_here"
```

**成功响应 (200 OK)**:
```json
{
  "conversations": [
    {
      "id": 1,
      "user_id": 1,
      "session_id": "session_123",
      "title": "关于Rust编程的讨论",
      "model": "qwen-plus",
      "message_count": 5,
      "summary": "Rust是一个 systems programming language，...",
      "created_at": "2024-10-23T10:00:00Z",
      "updated_at": "2024-10-23T10:30:00Z"
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 10,
  "total_pages": 1
}
```

#### POST /v1/conversations

创建新的对话，可选择添加初始消息。

**请求体**:
```json
{
  "session_id": "session_new",         // 可选，会话ID
  "title": "新的对话",                // 可选，对话标题
  "system_prompt": "你是一个友好的AI助手", // 可选，系统提示词
  "initial_message": "你好，请介绍一下你的功能"  // 可选，初始消息
}
```

**请求示例**:
```bash
curl -X POST http://localhost:3000/v1/conversations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key_here" \
  -d '{
    "session_id": "session_new",
    "title": "新的对话",
    "initial_message": "你好，请介绍一下你的功能"
  }'
```

**成功响应 (201 Created)**:
```json
{
  "conversation": {
    "id": 3,
    "user_id": 1,
    "session_id": "session_new",
    "title": "新的对话",
    "model": "qwen-plus",
    "message_count": 1,
    "summary": "",
    "created_at": "2024-10-23T11:00:00Z",
    "updated_at": "2024-10-23T11:00:00Z"
  },
  "first_message": {
    "id": 15,
    "conversation_id": 3,
    "role": "user",
    "content": "你好，请介绍一下你的功能",
    "model": null,
    "usage": null,
    "metadata": null,
    "created_at": "2024-10-23T11:00:00Z"
  }
}
```

#### GET /v1/conversations/{id}

获取对话的完整信息。

**路径参数**:
- `id` (integer): 对话ID

**请求示例**:
```bash
curl -X GET http://localhost:3000/v1/conversations/3 \
  -H "Authorization: Bearer your_api_key_here"
```

#### PUT /v1/conversations/{id}/title

更新对话的标题。

**路径参数**:
- `id` (integer): 对话ID

**请求体**:
```json
{
  "title": "更新后的标题"
}
```

#### DELETE /v1/conversations/{id}

删除对话及其所有消息。

**路径参数**:
- `id` (integer): 对话ID

### 4.2 消息管理

#### GET /v1/conversations/{id}/messages

获取对话中的所有消息，支持分页。

**路径参数**:
- `id` (integer): 对话ID

**请求参数 (Query)**:
- `limit` (integer, 可选): 分页大小，默认50，最大100
- `offset` (integer, 可选): 偏移量，默认0
- `before` (integer, 可选): 获取指定消息ID之前的消息

**成功响应 (200 OK)**:
```json
{
  "messages": [
    {
      "id": 1,
      "conversation_id": 1,
      "role": "user",
      "content": "你好，请介绍一下Rust编程语言",
      "model": null,
      "usage": null,
      "metadata": null,
      "created_at": "2024-10-23T10:00:00Z"
    },
    {
      "id": 2,
      "conversation_id": 1,
      "role": "assistant",
      "content": "Rust是一种现代系统编程语言...",
      "model": "qwen-plus",
      "usage": {
        "prompt_tokens": 15,
        "completion_tokens": 80,
        "total_tokens": 95
      },
      "metadata": {
        "response_time_ms": 1200,
        "model_provider": "qwen"
      },
      "created_at": "2024-10-23T10:00:05Z"
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

#### POST /v1/conversations/{id}/messages

在指定对话中添加新消息。

**路径参数**:
- `id` (integer): 对话ID

**请求体**:
```json
{
  "role": "user",
  "content": "这是新的消息内容",
  "metadata": {}
}
```

#### GET /v1/conversations/{id}/messages/{message_id}

获取单个消息的完整信息。

**路径参数**:
- `id` (integer): 对话ID
- `message_id` (integer): 消息ID

#### DELETE /v1/conversations/{id}/messages/{message_id}

删除对话中的单个消息。

**路径参数**:
- `id` (integer): 对话ID
- `message_id` (integer): 消息ID

### 4.3 错误处理

#### 常见错误响应

| 状态码 | 错误类型 | 描述 |
|--------|----------|------|
| 400 | `bad_request` | 请求参数无效 |
| 401 | `unauthorized` | 未授权访问 |
| 404 | `not_found` | 资源不存在 |
| 500 | `internal_server_error` | 服务器内部错误 |

### 4.4 使用示例

#### JavaScript/TypeScript

```javascript
class ConversationManager {
  constructor(apiKey, baseURL = 'http://localhost:3000') {
    this.apiKey = apiKey;
    this.baseURL = baseURL;
  }

  async getConversations(params = {}) {
    const query = new URLSearchParams(params);
    const response = await fetch(`${this.baseURL}/v1/conversations?${query}`, {
      headers: {
        'Authorization': `Bearer ${this.apiKey}`
      }
    });

    if (!response.ok) {
      throw new Error(`获取对话列表失败: ${response.status}`);
    }

    return response.json();
  }

  async createConversation(data) {
    const response = await fetch(`${this.baseURL}/v1/conversations`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`
      },
      body: JSON.stringify(data)
    });

    if (!response.ok) {
      throw new Error(`创建对话失败: ${response.status}`);
    }

    return response.json();
  }

  async getConversationMessages(conversationId, params = {}) {
    const query = new URLSearchParams(params);
    const response = await fetch(`${this.baseURL}/v1/conversations/${conversationId}/messages?${query}`, {
      headers: {
        'Authorization': `Bearer ${this.apiKey}`
      }
    });

    if (!response.ok) {
      throw new Error(`获取消息失败: ${response.status}`);
    }

    return response.json();
  }
}

// 使用示例
const manager = new ConversationManager('your_api_key');

// 获取对话列表
const conversations = await manager.getConversations({ limit: 10, session_id: 'session_123' });

// 创建新对话
const newConversation = await manager.createConversation({
  title: '新对话',
  initial_message: '你好'
});

// 获取对话消息
const messages = await manager.getConversationMessages(1, { limit: 20 });
```

## 5. 使用统计接口

### GET /usage/stats

获取用户的使用统计信息，支持按时间范围和周期进行统计。

**请求说明**
- **功能**: 获取用户的使用统计数据
- **鉴权**: 无需鉴权（公开接口，可用于监控）
- **幂等性**: 是

**请求参数 (Query)**
- `from_date` (string, 可选): 开始日期，ISO 8601格式，默认30天前
- `to_date` (string, 可选): 结束日期，ISO 8601格式，默认当前时间
- `period` (string, 可选): 统计周期 (day/week/month)，默认"day"

**请求示例**
```bash
# 获取指定日期范围的使用统计
curl -X GET "http://localhost:3000/usage/stats?from_date=2024-10-01T00:00:00Z&to_date=2024-10-23T23:59:59Z&period=day" \
  -H "Accept: application/json"

# 使用默认参数
curl -X GET "http://localhost:3000/usage/stats" \
  -H "Accept: application/json"
```

**成功响应 (200 OK)**
```json
{
  "period": "day",
  "from_date": "2024-10-01T00:00:00Z",
  "to_date": "2024-10-23T23:59:59Z",
  "stats": {
    "total_requests": 266,
    "total_tokens": 76678,
    "total_cost": 0.203115,
    "avg_response_time_ms": 1250.0,
    "model_usage": {
      "qwen-plus": {
        "model": "qwen-plus",
        "requests": 120,
        "tokens": 34567,
        "cost": 0.098765
      },
      "qwen-turbo": {
        "model": "qwen-turbo",
        "requests": 85,
        "tokens": 22100,
        "cost": 0.033150
      },
      "qwen-max": {
        "model": "qwen-max",
        "requests": 25,
        "tokens": 8900,
        "cost": 0.071200
      },
      "ollama-qwen3-4b": {
        "model": "ollama-qwen3-4b",
        "requests": 36,
        "tokens": 11111,
        "cost": 0.000000
      }
    }
  }
}
```

**响应字段说明**

| 字段 | 类型 | 说明 |
|------|------|------|
| `period` | String | 统计周期 |
| `from_date` | String | 统计开始时间 |
| `to_date` | String | 统计结束时间 |
| `stats` | Object | 详细统计数据 |

**统计数据字段说明**

| 字段 | 类型 | 说明 |
|------|------|------|
| `total_requests` | Integer | 总请求数 |
| `total_tokens` | Integer | 总Token使用量 |
| `total_cost` | Number | 总成本（美元） |
| `avg_response_time_ms` | Number | 平均响应时间（毫秒） |
| `model_usage` | Object | 各模型使用情况 |

**模型使用统计字段说明**

| 字段 | 类型 | 说明 |
|------|------|------|
| `model` | String | 模型名称 |
| `requests` | Integer | 该模型的请求次数 |
| `tokens` | Integer | 该模型的Token使用量 |
| `cost` | Number | 该模型的成本（美元） |

## 6. 健康检查接口

### GET /health

检查服务及其依赖组件的健康状态。

**请求说明**
- **功能**: 检查服务健康状态
- **鉴权**: 无需鉴权（公开接口）
- **幂等性**: 是

**请求头**
```
Accept: application/json
```

**请求示例**
```bash
curl -X GET "http://localhost:3000/health" \
  -H "Accept: application/json"
```

**成功响应 (200 OK)**
```json
{
  "status": "healthy",
  "timestamp": "2024-10-24T02:27:37.509891900Z",
  "version": "1.2.0",
  "services": {
    "database": "healthy",
    "cache": "healthy",
    "ai_models": {
      "qwen-plus": "healthy",
      "qwen-turbo": "healthy",
      "qwen-max": "healthy",
      "qwen-flash": "healthy",
      "qwq-plus": "healthy",
      "ollama-qwen3-4b": "healthy",
      "ollama-llama3": "healthy"
    }
  }
}
```

**响应字段说明**

| 字段 | 类型 | 说明 |
|------|------|------|
| `status` | String | 整体服务状态 (healthy/unhealthy/unknown) |
| `timestamp` | String | 检查时间戳 (ISO 8601格式) |
| `version` | String | 服务版本号 |
| `services` | Object | 各组件状态详情 |

**服务组件状态说明**

| 字段 | 类型 | 说明 |
|------|------|------|
| `database` | String | 数据库状态 |
| `cache` | String | 缓存状态 |
| `ai_models` | Object | AI模型状态映射 |

**AI模型状态说明**

| 状态 | 含义 |
|------|------|
| `healthy` | 模型正常可用 |
| `unhealthy` | 模型不可用 |
| `unknown` | 状态未知 |

**使用场景**

1. **服务监控**: 定期检查服务健康状态
2. **负载均衡**: 根据健康状态决定流量分发
3. **运维告警**: 监控服务异常并触发告警
4. **服务发现**: 配合服务注册中心使用

## 速率限制

| 端点 | 限制 | 时间窗口 |
|------|------|----------|
| `/v1/auth` | 10次/IP/分钟 | 1分钟 |
| `/v1/chat` | 60次/用户/分钟 | 1分钟 |
| `/v1/chat` (流式) | 30次/用户/分钟 | 1分钟 |

## SDK和代码示例

### JavaScript/TypeScript

```javascript
class TopMatLLMClient {
  constructor(apiKey, baseURL = 'http://localhost:3000') {
    this.apiKey = apiKey;
    this.baseURL = baseURL;
  }

  async authenticate() {
    const response = await fetch(`${this.baseURL}/auth`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`
      }
    });

    if (!response.ok) {
      throw new Error(`Auth failed: ${response.status}`);
    }

    return response.json();
  }

  async chat(message, options = {}) {
    const response = await fetch(`${this.baseURL}/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`
      },
      body: JSON.stringify({
        message,
        stream: false,
        model: 'qwen-plus',
        ...options
      })
    });

    if (!response.ok) {
      throw new Error(`Chat failed: ${response.status}`);
    }

    return response.json();
  }

  async *chatStream(message, options = {}) {
    const response = await fetch(`${this.baseURL}/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`
      },
      body: JSON.stringify({
        message,
        stream: true,
        model: 'qwen-plus',
        ...options
      })
    });

    if (!response.ok) {
      throw new Error(`Chat stream failed: ${response.status}`);
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        const chunk = decoder.decode(value);
        const lines = chunk.split('\n');

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            if (data.trim()) {
              try {
                const parsed = JSON.parse(data);
                yield parsed;
                if (parsed.type === 'final') break;
              } catch (e) {
                console.error('Failed to parse SSE data:', data);
              }
            }
          }
        }
      }
    } finally {
      reader.releaseLock();
    }
  }
}

// 使用示例
const client = new TopMatLLMClient('your_api_key');

// 非流式聊天
const response = await client.chat('你好，请介绍一下自己');
console.log(response.content);

// 流式聊天
for await (const chunk of client.chatStream('写一首关于编程的诗')) {
  if (chunk.type === 'content') {
    process.stdout.write(chunk.text);
  }
}
```

### Python

```python
import requests
import json
from typing import AsyncGenerator, Optional

class TopMatLLMClient:
    def __init__(self, api_key: str, base_url: str = "http://localhost:3000"):
        self.api_key = api_key
        self.base_url = base_url
        self.session = requests.Session()
        self.session.headers.update({
            'Authorization': f'Bearer {api_key}',
            'Content-Type': 'application/json'
        })

    def authenticate(self) -> dict:
        """鉴权验证"""
        response = self.session.post(f"{self.base_url}/auth")
        response.raise_for_status()
        return response.json()

    def chat(self, message: str, **options) -> dict:
        """非流式聊天"""
        data = {
            "message": message,
            "stream": False,
            "model": "qwen-plus",
            **options
        }

        response = self.session.post(f"{self.base_url}/chat", json=data)
        response.raise_for_status()
        return response.json()

    def chat_stream(self, message: str, **options) -> AsyncGenerator[dict, None]:
        """流式聊天"""
        import asyncio
        import aiohttp

        async def stream_generator():
            data = {
                "message": message,
                "stream": True,
                "model": "qwen-plus",
                **options
            }

            async with aiohttp.ClientSession() as session:
                session.headers.update({
                    'Authorization': f'Bearer {self.api_key}',
                    'Content-Type': 'application/json'
                })

                async with session.post(f"{self.base_url}/chat", json=data) as response:
                    if response.status != 200:
                        raise Exception(f"Chat stream failed: {response.status}")

                    async for line in response.content:
                        line = line.decode('utf-8').strip()
                        if line.startswith('data: '):
                            data_str = line[6:]
                            if data_str:
                                try:
                                    chunk = json.loads(data_str)
                                    yield chunk
                                    if chunk.get('type') == 'final':
                                        break
                                except json.JSONDecodeError:
                                    continue

        return stream_generator()

# 使用示例
client = TopMatLLMClient('your_api_key')

# 鉴权
auth_result = client.authenticate()
print(f"用户: {auth_result['data']['user']['username']}")

# 非流式聊天
response = client.chat('你好，请介绍一下自己')
print(f"回复: {response['content']}")

# 流式聊天
async def stream_example():
    async for chunk in client.chat_stream('写一首关于编程的诗'):
        if chunk.get('type') == 'content':
            print(chunk['text'], end='', flush=True)
    print()

# 运行流式示例
asyncio.run(stream_example())
```

## 常见问题

### Q: 如何处理长对话？
A: 使用 `conversation_id` 参数继续特定对话，系统会自动加载历史上下文。

### Q: 如何控制AI回复的长度？
A: 使用 `max_tokens` 参数限制最大生成长度，使用 `temperature` 控制创造性。

### Q: 流式响应中断了怎么办？
A: 流式连接中断后，可以使用 `conversation_id` 继续对话，系统会保存已生成的内容。

### Q: 如何查看使用统计？
A: 使用 `/usage/stats` 接口查看详细的token使用和成本统计。

### Q: 支持哪些文件格式？
A: 目前主要支持文本输入，计划支持图片、文档等多模态输入。

## 更新日志

### v1.3.0 (2024-10-24)
- 新增使用统计接口 `/usage/stats`
- 新增健康检查接口 `/health`
- 支持按时间范围和周期统计API使用情况
- 提供详细的服务健康状态监控
- 完善API文档，添加完整的接口说明和示例

### v1.2.0 (2024-10-23)
- 新增完整的对话历史管理功能
- 实现10个对话和消息管理接口
- 支持对话创建、查询、更新、删除
- 支持消息管理和分页加载
- 提供详细的SDK使用示例

### v1.1.0 (2024-10-23)
- 新增 `/v1/models` 接口，用于获取可用模型列表
- 重构模型路由机制，提高代码可维护性
- 优化模型注册和分发逻辑
- 更新API文档，添加模型详细信息

### v1.0.0 (2024-10-23)
- 初始版本发布
- 支持 `/auth` 和 `/chat` 接口
- 支持流式和非流式响应
- 集成多种AI模型
- 实现用户鉴权和会话管理

---

**联系方式**
- 技术支持: support@topmaterial-tech.com
- 文档更新: 请关注GitHub仓库的最新版本