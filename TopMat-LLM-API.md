# TopMat-LLM 完整 API 文档

> 基于 Rust Edition 2024 构建的高性能统一 LLM 聊天服务器

---

## 概述

**TopMat-LLM** 提供标准化的 REST API 接口，支持多种 AI 模型的聊天对话、MCP 工具集成和完整的对话管理功能。

| 项目 | 信息 |
|------|------|
| **基础 URL** | `http://localhost:3000` (开发) / `http://localhost:10007` (Docker) |
| **API 版本** | v1 |
| **协议** | HTTP/HTTPS |
| **数据格式** | JSON |
| **文档版本** | 1.3.0 |
| **Swagger UI** | `/swagger-ui` |
| **OpenAPI 规范** | `/api-docs/openapi.json` |

---

## 目录

1. [认证机制](#认证机制)
2. [API 端点总览](#api-端点总览)
3. [公开端点](#公开端点)
   - [健康检查](#健康检查)
   - [模型列表](#模型列表)
   - [用户鉴权](#用户鉴权)
4. [聊天端点](#聊天端点)
   - [AI 聊天](#ai-聊天)
5. [对话管理](#对话管理)
   - [对话列表](#获取对话列表)
   - [创建对话](#创建新对话)
   - [获取对话](#获取对话详情)
   - [更新对话标题](#更新对话标题)
   - [删除对话](#删除对话)
6. [消息管理](#消息管理)
   - [消息列表](#获取消息列表)
   - [消息详情](#获取消息详情)
   - [删除消息](#删除消息)
7. [使用统计](#使用统计)
   - [聊天使用统计](#获取使用统计)
   - [MCP 使用统计](#mcp-使用统计)
   - [MCP 会话列表](#mcp-会话列表)
   - [MCP 工具调用记录](#mcp-工具调用记录)
   - [综合统计](#综合统计)
8. [MCP 工具](#mcp-工具)
   - [MCP 服务器信息](#mcp-服务器信息)
   - [SSE 连接](#sse-连接)
   - [工具列表](#mcp-工具列表)
9. [支持的 AI 模型](#支持的-ai-模型)
10. [数据模型](#数据模型)
11. [错误处理](#错误处理)
12. [配置说明](#配置说明)

---

## 认证机制

所有需要认证的 API 请求都需要通过 API Key 进行鉴权：

```http
Authorization: Bearer your_api_key_here
```

> **提示**: 可通过 `/v1/auth` 端点验证 API Key 的有效性。

---

## API 端点总览

### 公开端点 (无需认证)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/health` | 健康检查 |
| GET | `/v1/models` | 获取可用模型列表 |
| POST | `/v1/auth` | API Key 认证 |
| GET | `/swagger-ui` | Swagger 文档界面 |

### 认证端点

| 方法 | 路径 | 描述 |
|------|------|------|
| POST | `/v1/chat` | AI 聊天对话 |
| GET | `/v1/conversations` | 获取对话列表 |
| POST | `/v1/conversations` | 创建新对话 |
| GET | `/v1/conversations/{id}` | 获取对话详情 |
| PUT | `/v1/conversations/{id}/title` | 更新对话标题 |
| DELETE | `/v1/conversations/{id}` | 删除对话 |
| GET | `/v1/conversations/{id}/messages` | 获取消息列表 |
| GET | `/v1/conversations/{id}/messages/{message_id}` | 获取消息详情 |
| DELETE | `/v1/conversations/{id}/messages/{message_id}` | 删除消息 |
| GET | `/usage/stats` | 获取使用统计 |
| GET | `/usage/mcp/stats` | MCP 使用统计 |
| GET | `/usage/mcp/sessions` | MCP 会话列表 |
| GET | `/usage/mcp/tool-calls` | MCP 工具调用记录 |
| GET | `/usage/comprehensive` | 综合统计 |

### MCP 端点

| 方法 | 路径 | 描述 | 认证 |
|------|------|------|------|
| GET | `/mcp/` | 工具发现 | 无需 |
| POST | `/mcp/` | 工具执行 | 必需 |
| GET | `/sse/` | SSE 连接 | 可选 |
| POST | `/sse/message` | SSE 消息发送 | 必需 |

---

## 公开端点

### 健康检查

检查服务器和各组件的健康状态。

```http
GET /health
```

**响应示例:**
```json
{
  "status": "healthy",
  "timestamp": "2026-01-15T11:30:00+08:00",
  "version": "1.3.0",
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

---

### 模型列表

获取当前可用的 AI 模型列表。

```http
GET /v1/models
```

**响应示例:**
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
      "name": "calphamesh",
      "provider": "qwen",
      "description": "Calphamesh智能体，可以调用Calphamesh工具",
      "supports_streaming": true,
      "max_tokens": 4096,
      "cost_per_1k_tokens": 0.0010
    }
  ],
  "total": 11,
  "timestamp": "2026-01-15T11:30:00+08:00"
}
```

---

### 用户鉴权

验证 API Key 的有效性并获取用户信息。

```http
POST /v1/auth
Authorization: Bearer your_api_key_here
```

**响应示例:**
```json
{
  "valid": true,
  "message": "鉴权成功",
  "timestamp": "2026-01-15T11:30:00+08:00",
  "user": {
    "username": "example_user",
    "subscription_level": "pro",
    "email": "user@example.com"
  },
  "api_key": {
    "key_name": "My API Key",
    "expires_at": "2026-12-31T23:59:59Z"
  }
}
```

---

## 聊天端点

### AI 聊天

与 AI 模型进行对话，支持流式和非流式响应。

```http
POST /v1/chat
Authorization: Bearer your_api_key_here
Content-Type: application/json
```

**请求参数:**

| 参数 | 类型 | 必需 | 描述 |
|------|------|------|------|
| `message` | string | 是 | 用户输入的消息 |
| `model` | string | 否 | 模型名称，默认 `qwen-flash` |
| `stream` | boolean | 否 | 是否流式响应，默认 `false` |
| `conversation_id` | string | 否 | 对话 ID (UUID)，不提供则自动创建 |
| `enable_reasoning` | boolean | 否 | 是否开启思考模式（推理模式），默认 `false` |
| `temperature` | float | 否 | 温度参数 (0.0-2.0) |
| `max_tokens` | integer | 否 | 最大 token 数 |
| `system_prompt` | string | 否 | 系统提示词 |
| `metadata` | object | 否 | 自定义元数据 |

**请求示例:**
```json
{
  "message": "你好，介绍一下你自己",
  "model": "qwen-plus",
  "stream": false,
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000"
}
```

**非流式响应:**
```json
{
  "content": "你好！我是 TopMat-LLM，一个专注于材料科学的 AI 助手。",
  "model": "qwen-plus",
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 35,
    "total_tokens": 55
  },
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "timestamp": "2026-01-15T11:30:00+08:00",
  "metadata": {
    "response_time_ms": 1500
  }
}
```

**流式响应 (SSE):**
```
data: {"type":"content","text":"你好","finished":false}
data: {"type":"content","text":"！我是","finished":false}
data: {"type":"reasoning","reasoning":"正在思考..."}
data: {"type":"tool_call","id":"call_abc123","name":"calphamesh_list_tasks","arguments":{...}}
data: {"type":"tool_result","id":"call_abc123","result":"..."}
data: {"type":"error","message":"发生了一些问题"}
data: {"type":"final","response":{...}}
```

---

## 对话管理

### 获取对话列表

```http
GET /v1/conversations
Authorization: Bearer your_api_key_here
```

**查询参数:**
| 参数 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `limit` | integer | 20 | 每页数量 |
| `offset` | integer | 0 | 偏移量 |
| `model` | string | - | 按模型筛选 |
| `search` | string | - | 搜索关键词 (标题或摘要) |

**响应示例:**
```json
{
  "conversations": [
    {
      "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
      "user_id": 1,
      "title": "材料科学讨论",
      "model": "qwen-plus",
      "message_count": 10,
      "summary": "关于金属材料强度特性的讨论",
      "created_at": "2026-01-15T10:00:00+08:00",
      "updated_at": "2026-01-15T11:30:00+08:00"
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 20,
  "total_pages": 1
}
```

---

### 创建新对话

```http
POST /v1/conversations
Authorization: Bearer your_api_key_here
Content-Type: application/json
```

**请求体:**
```json
{
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "title": "新的对话主题",
  "model": "qwen-plus",
  "system_prompt": "你是一个专业的材料科学助手",
  "initial_message": "你好，开始我们的对话"
}
```

**响应示例:**
```json
{
  "conversation": {
    "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
    "user_id": 1,
    "title": "新的对话主题",
    "model": "qwen-plus",
    "message_count": 1,
    "summary": null,
    "created_at": "2026-01-15T12:00:00+08:00",
    "updated_at": "2026-01-15T12:00:00+08:00"
  },
  "first_message": {
    "id": 1,
    "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
    "role": "user",
    "content": "你好，开始我们的对话",
    "created_at": "2026-01-15T12:00:00+08:00"
  }
}
```

---

### 获取对话详情

```http
GET /v1/conversations/{id}
Authorization: Bearer your_api_key_here
```

**响应示例:**
```json
{
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "user_id": 1,
  "title": "材料科学讨论",
  "model": "qwen-plus",
  "message_count": 10,
  "summary": "关于金属材料强度特性的讨论",
  "created_at": "2026-01-15T10:00:00+08:00",
  "updated_at": "2026-01-15T11:30:00+08:00"
}
```

---

### 更新对话标题

```http
PUT /v1/conversations/{id}/title
Authorization: Bearer your_api_key_here
Content-Type: application/json
```

**请求体:**
```json
{
  "title": "更新后的标题"
}
```

**响应示例:**
```json
{
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "user_id": 1,
  "title": "更新后的标题",
  "model": "qwen-plus",
  "message_count": 10,
  "summary": "...",
  "created_at": "2026-01-15T10:00:00+08:00",
  "updated_at": "2026-01-15T12:30:00+08:00"
}
```

---

### 删除对话

```http
DELETE /v1/conversations/{id}
Authorization: Bearer your_api_key_here
```

**响应示例:**
```json
{
  "success": true,
  "message": "对话删除成功",
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "deleted_messages_count": 10,
  "timestamp": "2026-01-15T13:00:00+08:00"
}
```

---

## 消息管理

### 获取消息列表

```http
GET /v1/conversations/{id}/messages
Authorization: Bearer your_api_key_here
```

**查询参数:**
| 参数 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `limit` | integer | 50 | 每页数量 (最大 100) |
| `offset` | integer | 0 | 偏移量 |
| `before` | integer | - | 获取指定消息 ID 之前的消息 |

**响应示例:**
```json
{
  "messages": [
    {
      "id": 1,
      "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
      "role": "user/assistant/tool",
      "content": "你好",
      "reasoning_content": "",
      "tool_calls":[],
      "tool_call_id": "",
      "model": "qwen-plus",
      "usage": {
        "prompt_tokens": 5,
        "completion_tokens": 10,
        "total_tokens": 15
      },
      "medata": [],
      "created_at": "2026-01-15T12:00:00+08:00"
    }
  ],
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "total": 1,
  "page": 1,
  "page_size": 50,
  "total_pages": 1,
  "has_more": false
}
```

---

### 获取消息详情

```http
GET /v1/conversations/{id}/messages/{message_id}
Authorization: Bearer your_api_key_here
```

**响应示例:**
```json
{
  "id": 1,
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "role": "assistant",
  "content": "你好！有什么我可以帮您的吗？",
  "reasoning_content": "用户说了你好，我应该礼貌回应。",
  "tool_calls":[],
  "tool_call_id": "",
  "model": "qwen-plus",
  "usage": {
    "prompt_tokens": 5,
    "completion_tokens": 10,
    "total_tokens": 15
  },
  "medata": [],
  "created_at": "2026-01-15T12:00:05+08:00"
}
```

---

### 删除消息

```http
DELETE /v1/conversations/{id}/messages/{message_id}
Authorization: Bearer your_api_key_here
```

**响应示例:**
```json
{
  "success": true,
  "message": "消息删除成功",
  "message_id": 2,
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "timestamp": "2026-01-15T13:30:00+08:00"
}
```

---

## 使用统计

### 获取使用统计

```http
GET /usage/stats
Authorization: Bearer your_api_key_here
```

**查询参数:**
| 参数 | 类型 | 描述 |
|------|------|------|
| `period` | string | 统计周期 (day/week/month) |
| `from_date` | string | 开始日期 (ISO 8601) |
| `to_date` | string | 结束日期 (ISO 8601) |

**响应示例:**
```json
{
  "period": "day",
  "from_date": "2026-01-27T17:42:29+08:00",
  "to_date": "2026-01-28T17:42:29+08:00",
  "stats": {
    "total_requests": 150,
    "total_tokens": 12500,
    "total_cost": 0.25,
    "avg_response_time_ms": 1150.0,
    "model_usage": {
      "qwen-plus": {
        "model": "qwen-plus",
        "requests": 100,
        "tokens": 8000,
        "cost": 0.16
      },
      "qwen-max": {
        "model": "qwen-max",
        "requests": 50,
        "tokens": 4500,
        "cost": 0.09
      }
    }
  }
}
```

---

### MCP 使用统计

```http
GET /usage/mcp/stats
Authorization: Bearer your_api_key_here
```

**响应示例:**
```json
{
  "total_sessions": 25,
  "total_tool_calls": 150,
  "unique_tools_used": 8,
  "success_rate": 0.95,
  "transport_type_counts": {
    "http": 80,
    "sse": 70
  }
}
```

---

### MCP 会话列表

```http
GET /usage/mcp/sessions
Authorization: Bearer your_api_key_here
```

**响应示例:**
```json
{
  "data": [
    {
      "session_id": "sess_123456",
      "transport_type": "http",
      "tool_calls_count": 5,
      "created_at": "2024-01-01T12:00:00+08:00",
      "last_activity_at": "2024-01-01T12:30:00+08:00"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 25,
    "total_pages": 2
  }
}
```

---

### MCP 工具调用记录

```http
GET /usage/mcp/tool-calls
Authorization: Bearer your_api_key_here
```

**响应示例:**
```json
{
  "data": [
    {
      "session_id": "sess_123456",
      "tool_name": "calpha_mesh_simulation",
      "request_arguments": "{\"temp\": 1000}",
      "response_result": "{\"status\": \"success\"}",
      "status": "success",
      "error_message": null,
      "transport_type": "http",
      "endpoint": "/mcp",
      "execution_time_ms": 1250,
      "created_at": "2024-01-01T12:15:00+08:00"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 150,
    "total_pages": 8
  }
}
```

---

### 综合统计

```http
GET /usage/comprehensive
Authorization: Bearer your_api_key_here
```

**响应示例:**
```json
{
  "mcp": {
    "total_sessions": 25,
    "total_tool_calls": 150,
    "unique_tools_used": 8,
    "success_rate": 0.95,
    "transport_type_counts": {
      "http": 80,
      "sse": 70
    }
  },
  "chat": {
    "total_conversations": 300,
    "total_messages": 15000
  },
  "summary": {
    "total_requests": 450,
    "active_sessions": 325,
    "data_points": 15150
  }
}
```

---

## MCP 工具

TopMat-LLM 提供 **19+ 专业工具**，专为材料科学和计算工作流设计。

### MCP 工具列表

| 工具名称 | 类别 | 描述 |
|----------|------|------|
| **think** | 通用 | 内部推理和思考能力 |
| **calphamesh_submit_point_task** | CalphaMesh | 提交点计算任务 |
| **calphamesh_submit_line_task** | CalphaMesh | 提交线计算任务 |
| **calphamesh_submit_scheil_task** | CalphaMesh | 提交 Scheil 任务 |
| **calphamesh_get_task_status** | CalphaMesh | 获取任务状态 |
| **calphamesh_list_tasks** | CalphaMesh | 任务列表查询 |
| **onnx_get_models_info** | ONNX | 获取模型信息 |
| **onnx_model_inference** | ONNX | 模型推理 |
| **onnx_get_model_config** | ONNX | 模型配置查询 |
| **steel_rag** | Dify | 钢铁 RAG 检索 |
| **cemented_carbide_rag** | Dify | 硬质合金 RAG |
| **Al_idme_workflow** | Dify | 铝 IDME 工作流 |
| **phase_field_submit_spinodal_decomposition_task** | 相场 | 调幅分解仿真 |
| **phase_field_submit_pvd_simulation_task** | 相场 | PVD 仿真 |
| **phase_field_get_task_list** | 相场 | 任务列表 |
| **phase_field_get_task_status** | 相场 | 任务状态 |
| **topPhi_simulator** | 仿真 | 涂层沉积模拟 |
| **ml_performance_predictor** | 仿真 | ML 性能预测 |
| **historical_data_query** | 仿真 | 历史数据查询 |
| **experimental_data_reader** | 仿真 | 实验数据读取 |

### MCP 服务器信息

```http
GET /mcp/
```

返回 MCP 服务器能力声明和可用工具列表。

### SSE 连接

```http
GET /sse/
Accept: text/event-stream
Authorization: Bearer your_api_key_here
```

通过 Server-Sent Events 建立 MCP 连接。

**JavaScript 示例:**
```javascript
const eventSource = new EventSource('/sse/');
eventSource.onmessage = (event) => {
  console.log('收到消息:', JSON.parse(event.data));
};
```

### 工具调用

```http
POST /mcp/
Authorization: Bearer your_api_key_here
Content-Type: application/json
```

**请求示例 (JSON-RPC 2.0):**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "calphamesh_submit_point_task",
    "arguments": {
      "database": "tcfe",
      "components": ["Fe", "C", "Mn"],
      "composition": {"Fe": 0.95, "C": 0.02, "Mn": 0.03},
      "temperature": 1273.15,
      "pressure": 101325
    }
  }
}
```

---

## 支持的 AI 模型

### 通义千问模型

| 模型 ID | 描述 | 最大 Token | 成本/1K Token |
|---------|------|------------|---------------|
| `qwen-plus` | 通义千问 Plus，质量较高 | 4000 | ¥0.0020 |
| `qwen-turbo` | 通义千问 Turbo，响应快 | 4000 | ¥0.0015 |
| `qwen-max` | 通义千问 Max，最高质量 | 8000 | ¥0.0080 |
| `qwen-flash` | 通义千问 Flash，极速响应 | 2000 | ¥0.0005 |
| `qwq-plus` | 通义千问增强，逻辑推理强 | 4000 | ¥0.0030 |

### Ollama 本地模型

| 模型 ID | 描述 | 最大 Token |
|---------|------|------------|
| `ollama-qwen3-4b` | Qwen3 4B 本地版 | 4096 |
| `ollama-llama3` | Llama3 本地模型 | 4096 |

### Agent 模型

| 模型 ID | 描述 | 集成工具 |
|---------|------|----------|
| `calphamesh` | CalphaMesh 智能体 | CalphaMesh 工具集 |
| `phase-field` | Phase-field 智能体 | 相场仿真工具 |
| `ml-server` | ML-Server 智能体 | ONNX-Server 工具 |
| `coating` | 涂层优化智能体 | 涂层优化工具 (测试) |

---

## 数据模型

### 对话 (Conversation)

```json
{
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "user_id": 1,
  "title": "对话标题",
  "model": "qwen-plus",
  "message_count": 10,
  "summary": null,
  "created_at": "2026-01-15T10:00:00+08:00",
  "updated_at": "2026-01-15T11:30:00+08:00"
}
```

### 消息 (Message)

```json
{
  "id": 1,
  "conversation_id": "123e4567-e89b-12d3-a456-426614174000",
  "role": "assistant",
  "content": "消息内容",
  "model": "qwen-plus",
  "usage": {
    "prompt_tokens": 25,
    "completion_tokens": 150,
    "total_tokens": 175
  },
  "metadata": {"response_time_ms": 1500},
  "created_at": "2026-01-15T10:30:00+08:00"
}
```

### Token 使用 (TokenUsage)

```json
{
  "prompt_tokens": 25,
  "completion_tokens": 150,
  "total_tokens": 175
}
```

---

## 错误处理

### 错误响应格式

```json
{
  "error": "error_type",
  "message": "错误描述信息",
  "details": {
    "additional_info": "额外错误信息"
  },
  "timestamp": "2026-01-15T11:30:00+08:00"
}
```

### 错误代码

| 错误代码 | HTTP 状态码 | 描述 |
|----------|-------------|------|
| `auth_required` | 401 | 需要 API Key 认证 |
| `auth_invalid` | 401 | API Key 无效或已过期 |
| `auth_failed` | 401 | 认证失败 |
| `model_not_supported` | 400 | 不支持的模型 |
| `qwen_not_configured` | 503 | 通义千问未配置 |
| `ollama_not_available` | 503 | Ollama 服务不可用 |
| `conversation_not_found` | 404 | 对话不存在 |
| `message_not_found` | 404 | 消息不存在 |
| `access_denied` | 403 | 访问被拒绝 |
| `database_error` | 500 | 数据库错误 |
| `internal_error` | 500 | 内部服务器错误 |

---

## 配置说明

### 环境变量

| 变量名 | 描述 | 默认值 | 必需 |
|--------|------|--------|------|
| `SERVER_HOST` | 服务器监听地址 | `127.0.0.1` | 否 |
| `SERVER_PORT` | 服务器端口 | `3000` | 否 |
| `DATABASE_URL` | PostgreSQL 连接 URL | - | 是 |
| `DASHSCOPE_API_KEY` | 通义千问 API 密钥 | - | 是 |
| `OLLAMA_BASE_URL` | Ollama 服务地址 | `http://localhost:11434` | 否 |
| `AUTH_API_URL` | 认证服务地址 | `https://api.topmaterial-tech.com` | 否 |
| `MCP_SERVER_URL` | MCP 服务器 URL | `http://127.0.0.1:10001/mcp` | 否 |
| `RUST_LOG` | 日志级别 | `info` | 否 |

### 数据库表

- `users` - 用户管理
- `api_keys` - API 密钥管理
- `conversations` - 对话元数据
- `messages` - 聊天消息存储
- `usage_statistics` - Token 使用和成本跟踪
- `mcp_sessions` - MCP 会话跟踪
- `mcp_tool_calls` - 工具执行记录

---

## 快速开始

### 测试健康检查

```bash
curl http://localhost:3000/health
```

### 测试认证

```bash
curl -X POST http://localhost:3000/v1/auth \
  -H "Authorization: Bearer your_api_key"
```

### 测试聊天

```bash
curl -X POST http://localhost:3000/v1/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key" \
  -d '{
    "message": "你好",
    "model": "qwen-plus",
    "stream": false
  }'
```

### JavaScript 流式响应

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

---

**TopMat-LLM** - 用 Rust 构建材料科学 AI 的未来 🦀✨

*文档生成时间: 2026-01-15*
