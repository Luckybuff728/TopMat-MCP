# 对话历史管理 API 文档

## 概述

TopMat-LLM 提供完整的对话历史管理功能，支持对话的创建、查询、更新和删除，以及消息的管理。这些接口允许用户构建完整的对话历史记录和上下文管理。

## 接口列表

| 接口 | 方法 | 路径 | 描述 | 鉴权 |
|------|------|------|------|------|
| 对话列表 | GET | `/v1/conversations` | 获取用户对话列表 | 必需 |
| 创建对话 | POST | `/v1/conversations` | 创建新对话 | 必需 |
| 对话详情 | GET | `/v1/conversations/{id}` | 获取对话详情 | 必需 |
| 更新标题 | PUT | `/v1/conversations/{id}/title` | 更新对话标题 | 必需 |
| 删除对话 | DELETE | `/v1/conversations/{id}` | 删除对话 | 必需 |
| 消息列表 | GET | `/v1/conversations/{id}/messages` | 获取对话消息 | 必需 |
| 添加消息 | POST | `/v1/conversations/{id}/messages` | 添加新消息 | 必需 |
| 消息详情 | GET | `/v1/conversations/{id}/messages/{message_id}` | 获取消息详情 | 必需 |
| 删除消息 | DELETE | `/v1/conversations/{id}/messages/{message_id}` | 删除消息 | 必需 |

## 接口详细说明

### 1. 对话管理

#### 1.1 获取对话列表

**接口**: `GET /v1/conversations`

**功能**: 获取用户的对话列表，支持分页、筛选和搜索

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

#### 1.2 创建新对话

**接口**: `POST /v1/conversations`

**功能**: 创建新的对话，可选择添加初始消息

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

#### 1.3 获取对话详情

**接口**: `GET /v1/conversations/{id}`

**功能**: 获取对话的完整信息

**路径参数**:
- `id` (integer): 对话ID

**请求示例**:
```bash
curl -X GET http://localhost:3000/v1/conversations/3 \
  -H "Authorization: Bearer your_api_key_here"
```

**成功响应 (200 OK)**:
```json
{
  "id": 3,
  "user_id": 1,
  "session_id": "session_new",
  "title": "新的对话",
  "model": "qwen-plus",
  "message_count": 1,
  "created_at": "2024-10-23T11:00:00Z",
  "updated_at": "2024-10-23T11:00:00Z"
}
```

#### 1.4 更新对话标题

**接口**: `PUT /v1/conversations/{id}/title`

**功能**: 更新对话的标题

**路径参数**:
- `id` (integer): 对话ID

**请求体**:
```json
{
  "title": "更新后的标题"
}
```

**请求示例**:
```bash
curl -X PUT http://localhost:3000/v1/conversations/3/title \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key_here" \
  -d '{"title": "更新后的标题"}'
```

**成功响应 (200 OK)**:
```json
{
  "id": 3,
  "user_id": 1,
  "session_id": "session_new",
  "title": "更新后的标题",
  "model": "qwen-plus",
  "message_count": 1,
  "created_at": "2024-10-23T11:00:00Z",
  "updated_at": "2024-10-23T11:15:00Z"
}
```

#### 1.5 删除对话

**接口**: `DELETE /v1/conversations/{id}`

**功能**: 删除对话及其所有消息

**路径参数**:
- `id` (integer): 对话ID

**请求示例**:
```bash
curl -X DELETE http://localhost:3000/v1/conversations/3 \
  -H "Authorization: Bearer your_api_key_here"
```

**成功响应 (200 OK)**:
```json
{
  "success": true,
  "message": "对话删除成功",
  "conversation_id": 3,
  "timestamp": "2024-10-23T11:30:00Z"
}
```

### 2. 消息管理

#### 2.1 获取对话消息列表

**接口**: `GET /v1/conversations/{id}/messages`

**功能**: 获取对话中的所有消息，支持分页

**路径参数**:
- `id` (integer): 对话ID

**请求参数 (Query)**:
- `limit` (integer, 可选): 分页大小，默认50，最大100
- `offset` (integer, 可选): 偏移量，默认0
- `before` (integer, 可选): 获取指定消息ID之前的消息

**请求示例**:
```bash
curl -X GET "http://localhost:3000/v1/conversations/1/messages?limit=20" \
  -H "Authorization: Bearer your_api_key_here"
```

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
  "page_size": 20,
  "total_pages": 1,
  "has_more": false
}
```

#### 2.2 添加新消息

**接口**: `POST /v1/conversations/{id}/messages`

**功能**: 在指定对话中添加新消息

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

**请求示例**:
```bash
curl -X POST http://localhost:3000/v1/conversations/1/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key_here" \
  -d '{
    "role": "user",
    "content": "请继续刚才的话题"
  }'
```

**成功响应 (201 Created)**:
```json
{
  "id": 25,
  "conversation_id": 1,
  "role": "user",
  "content": "请继续刚才的话题",
  "model": null,
  "usage": null,
  "metadata": null,
  "created_at": "2024-10-23T11:45:00Z"
}
```

#### 2.3 获取消息详情

**接口**: `GET /v1/conversations/{id}/messages/{message_id}`

**功能**: 获取单个消息的完整信息

**路径参数**:
- `id` (integer): 对话ID
- `message_id` (integer): 消息ID

**请求示例**:
```bash
curl -X GET http://localhost:3000/v1/conversations/1/messages/2 \
  -H "Authorization: Bearer your_api_key_here"
```

**成功响应 (200 OK)**:
```json
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
```

#### 2.4 删除消息

**接口**: `DELETE /v1/conversations/{id}/messages/{message_id}`

**功能**: 删除对话中的单个消息

**路径参数**:
- `id` (integer): 对话ID
- `message_id` (integer): 消息ID

**请求示例**:
```bash
curl -X DELETE http://localhost:3000/v1/conversations/1/messages/3 \
  -H "Authorization: Bearer your_api_key_here"
```

**成功响应 (200 OK)**:
```json
{
  "success": true,
  "message": "消息删除成功",
  "message_id": 3,
  "conversation_id": 1,
  "timestamp": "2024-10-23T12:00:00Z"
}
```

## 错误处理

### 常见错误响应

#### 401 Unauthorized
```json
{
  "error": "unauthorized",
  "message": "未授权访问，请提供有效的API Key",
  "timestamp": "2024-10-23T10:30:00Z"
}
```

#### 404 Not Found
```json
{
  "error": "not_found",
  "message": "对话不存在",
  "details": {
    "conversation_id": 999
  },
  "timestamp": "2024-10-23T10:30:00Z"
}
```

#### 400 Bad Request
```json
{
  "error": "bad_request",
  "message": "请求参数无效",
  "details": {
    "field": "limit",
    "error": "limit不能超过100"
  },
  "timestamp": "2024-10-23T10:30:00Z"
}
```

#### 500 Internal Server Error
```json
{
  "error": "internal_server_error",
  "message": "服务器内部错误",
  "timestamp": "2024-10-23T10:30:00Z"
}
```

## 使用示例

### JavaScript SDK

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

  async addMessage(conversationId, message) {
    const response = await fetch(`${this.baseURL}/v1/conversations/${conversationId}/messages`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`
      },
      body: JSON.stringify(message)
    });

    if (!response.ok) {
      throw new Error(`添加消息失败: ${response.status}`);
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

// 添加新消息
const newMessage = await manager.addMessage(1, {
  role: 'user',
  content: '继续讨论'
});
```

### Python SDK

```python
import requests
import json

class ConversationManager:
    def __init__(self, api_key, base_url="http://localhost:3000"):
        self.api_key = api_key
        self.base_url = base_url
        self.session = requests.Session()
        self.session.headers.update({
            'Authorization': f'Bearer {api_key}',
            'Content-Type': 'application/json'
        })

    def get_conversations(self, **params):
        """获取对话列表"""
        response = self.session.get(
            f"{self.base_url}/v1/conversations",
            params=params
        )
        response.raise_for_status()
        return response.json()

    def create_conversation(self, data):
        """创建新对话"""
        response = self.session.post(
            f"{self.base_url}/v1/conversations",
            json=data
        )
        response.raise_for_status()
        return response.json()

    def get_conversation_messages(self, conversation_id, **params):
        """获取对话消息"""
        response = self.session.get(
            f"{self.base_url}/v1/conversations/{conversation_id}/messages",
            params=params
        )
        response.raise_for_status()
        return response.json()

    def add_message(self, conversation_id, message):
        """添加新消息"""
        response = self.session.post(
            f"{self.base_url}/v1/conversations/{conversation_id}/messages",
            json=message
        )
        response.raise_for_status()
        return response.json()

# 使用示例
manager = ConversationManager('your_api_key')

# 获取对话列表
conversations = manager.get_conversations(limit=10, session_id='session_123')

# 创建新对话
new_conversation = manager.create_conversation({
    'title': '新对话',
    'initial_message': '你好'
})

# 获取对话消息
messages = manager.get_conversation_messages(1, limit=20)

# 添加新消息
new_message = manager.add_message(1, {
    'role': 'user',
    'content': '继续讨论'
})
```

## 最佳实践

### 1. 分页处理
- 使用合理的 `limit` 参数（建议20-50）
- 在前端实现无限滚动或分页导航
- 根据消息数量动态调整加载策略

### 2. 会话管理
- 为不同的使用场景创建不同的 `session_id`
- 有意义的对话标题有助于用户识别和管理
- 定期清理不需要的对话

### 3. 性能优化
- 使用 `before` 参数实现消息的增量加载
- 合理设置缓存策略
- 避免重复获取相同的数据

### 4. 错误处理
- 实现重试机制处理网络错误
- 提供用户友好的错误提示
- 记录和监控API使用情况

## 注意事项

1. **数据持久化**: 当前实现使用模拟数据，实际应用中需要集成数据库
2. **权限控制**: 需要确保用户只能访问自己的对话和消息
3. **数据安全**: 敏感数据需要加密存储和传输
4. **性能考虑**: 大量历史消息的加载需要优化策略
5. **兼容性**: 保持API接口的向后兼容性