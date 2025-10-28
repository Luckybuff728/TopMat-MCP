# TopMat LLM 统一聊天服务器

[![Rust](https://img.shields.io/badge/rust-2024%20Edition-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/version-1.3.0-blue.svg)](https://github.com/your-org/TopMat-LLM)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**TopMat LLM** 是一个基于 Rust 构建的统一大语言模型聊天服务器，提供标准化的 REST API 接口，支持多种 AI 模型提供商，具备完整的会话管理、数据持久化和实时监控功能。

## ✨ 核心特性

### 🚀 核心功能
- **统一接口** - 单一 `/v1/chat` 端点处理所有模型交互
- **流式响应** - 支持实时流式输出 (Server-Sent Events)
- **多模型支持** - 通义千问、Ollama 本地模型等
- **会话管理** - 完整的对话历史记录和上下文保持
- **数据持久化** - SQLite 数据库存储对话和消息

### 🔐 认证与安全
- **API Key 认证** - 基于Bearer Token的安全认证
- **用户权限管理** - 细粒度的功能权限控制
- **请求中间件** - 自动身份验证和用户注入

### 📊 监控与统计
- **使用统计** - 详细的 Token 使用情况和成本跟踪
- **健康检查** - 服务状态和组件健康度监控
- **性能指标** - 响应时间和吞吐量统计

### ⚡ 性能特性
- **高性能** - 基于 Tokio 异步运行时
- **类型安全** - Rust 类型系统保证可靠性
- **智能路由** - 根据模型自动选择处理器
- **异步存储** - 非阻塞的数据持久化

## 🚀 快速开始

### 1. 环境准备

确保您的系统已安装：
- **Rust 2024 Edition** 或更高版本
- **SQLite 3** (通常系统自带)
- **Git**

### 2. 克隆项目

```bash
git clone https://github.com/your-org/TopMat-LLM.git
cd TopMat-LLM
```

### 3. 配置环境变量

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env 文件，填入您的配置
```

编辑 `.env` 文件：
```bash
# ===== 服务器配置（可选） =====
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
RUST_LOG=info

# ===== 数据库配置（可选） =====
DATABASE_URL=sqlite:data.db

# ===== 认证服务配置（可选） =====
AUTH_API_URL=https://api.topmaterial-tech.com

# ===== 通义千问 API（必需，如果使用通义模型） =====
DASHSCOPE_API_KEY=your_dashscope_api_key_here

# ===== Ollama 配置（可选） =====
OLLAMA_BASE_URL=http://localhost:11434

# ===== 其他提供商 API 密钥（可选） =====
# OPENAI_API_KEY=your_openai_api_key_here
# ANTHROPIC_API_KEY=your_anthropic_api_key_here
```

### 4. 启动服务器

```bash
# 开发模式运行
cargo run

# 生产模式运行
cargo run --release

# 或者直接运行二进制文件
cargo run --bin unified_server
```

服务器将在 `http://localhost:3000` 启动。

### 5. 验证安装

```bash
# 健康检查
curl http://localhost:3000/health

# 获取模型列表
curl http://localhost:3000/v1/models
```

## 📖 API 使用指南

### 认证

所有需要认证的接口都需要在请求头中包含 API Key：

```bash
# 使用 Authorization Header（推荐）
Authorization: Bearer your_api_key_here

# 或使用 X-API-Key Header
X-API-Key: your_api_key_here
```

### 核心 API 端点

| 端点 | 方法 | 描述 | 认证 |
|------|------|------|------|
| `/v1/auth` | POST | 验证API Key | 无需 |
| `/v1/models` | GET | 获取可用模型列表 | 无需 |
| `/health` | GET | 健康检查 | 无需 |
| `/v1/chat` | POST | AI对话服务 | 必需 |
| `/usage/stats` | GET | 使用统计 | 必需 |
| `/v1/conversations` | GET/POST | 对话管理 | 必需 |
| `/v1/conversations/:id/messages` | GET/POST | 消息管理 | 必需 |

### 聊天请求示例

#### 非流式请求

```bash
curl -X POST http://localhost:3000/v1/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key" \
  -d '{
    "message": "你好，请介绍一下Rust语言",
    "stream": false,
    "model": "qwen-plus",
    "conversation_id": 123,
    "temperature": 0.7,
    "max_tokens": 2000,
    "system_prompt": "你是一个专业的编程助手"
  }'
```

**响应格式**：
```json
{
  "content": "Rust是一门系统编程语言，注重内存安全和并发性...",
  "model": "qwen-plus",
  "usage": {
    "prompt_tokens": 25,
    "completion_tokens": 180,
    "total_tokens": 205
  },
  "conversation_id": 123,
  "timestamp": "2024-10-27T12:00:00Z",
  "metadata": {}
}
```

#### 流式请求

```bash
curl -X POST http://localhost:3000/v1/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key" \
  -d '{
    "message": "请写一个Rust Hello World程序",
    "stream": true,
    "model": "qwen-plus"
  }'
```

**流式响应** (Server-Sent Events)：
```
data: {"type":"content","text":"Rust","finished":false}

data: {"type":"content","text":" 中的","finished":false}

data: {"type":"content","text":" Hello World","finished":false}

data: {"type":"final","response":{"content":"Rust 中的 Hello World 程序如下：\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```","model":"qwen-plus","usage":{"prompt_tokens":15,"completion_tokens":45,"total_tokens":60},"conversation_id":123,"timestamp":"2024-10-27T12:00:00Z"}}
```

### 对话管理示例

#### 创建新对话

```bash
curl -X POST http://localhost:3000/v1/conversations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key" \
  -d '{
    "title": "Rust学习讨论",
    "model": "qwen-plus"
  }'
```

#### 获取对话列表

```bash
curl -X GET "http://localhost:3000/v1/conversations?page=1&limit=20" \
  -H "Authorization: Bearer your_api_key"
```

#### 获取对话消息

```bash
curl -X GET "http://localhost:3000/v1/conversations/123/messages?page=1&limit=50" \
  -H "Authorization: Bearer your_api_key"
```

### 使用统计示例

```bash
curl -X GET "http://localhost:3000/usage/stats?period=day&from_date=2024-10-01T00:00:00Z" \
  -H "Authorization: Bearer your_api_key"
```

## 🤖 支持的模型

### 通义千问模型

| 模型ID | 名称 | 描述 | 适用场景 |
|--------|------|------|----------|
| `qwen-plus` | 通义千问 Plus | 平衡性能和成本 | 通用对话、文本生成 |
| `qwen-turbo` | 通义千问 Turbo | 快速响应 | 简单问答、实时交互 |
| `qwen-max` | 通义千问 Max | 最强性能 | 复杂推理、专业领域 |
| `qwen-flash` | 通义千问 Flash | 超快响应 | 轻量级任务 |
| `qwq-plus` | 通义千问 qwq Plus | 推理增强 | 数学、逻辑推理 |

**要求**：配置 `DASHSCOPE_API_KEY`

### Ollama 本地模型

| 模型ID | 名称 | 描述 | 系统要求 |
|--------|------|------|----------|
| `ollama-qwen3-4b` | Qwen3 4B | 轻量级本地模型 | 4GB+ RAM |
| `ollama-llama3` | Llama3 | Meta开源模型 | 8GB+ RAM |

**Ollama 配置**：
```bash
# 安装 Ollama
# 访问 https://ollama.com/ 下载安装

# 安装模型
ollama pull qwen3:4b
ollama pull llama3:latest

# 启动 Ollama 服务
ollama serve
```

## 💻 客户端示例

### JavaScript/TypeScript 客户端

```typescript
interface ChatRequest {
  message: string;
  stream?: boolean;
  model?: string;
  conversation_id?: number;
  temperature?: number;
  max_tokens?: number;
  system_prompt?: string;
}

interface ChatResponse {
  content: string;
  model: string;
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
  conversation_id: number;
  timestamp: string;
  metadata: Record<string, any>;
}

class TopMatLLMClient {
  constructor(
    private baseURL: string = 'http://localhost:3000',
    private apiKey: string
  ) {}

  // 非流式聊天
  async chat(request: ChatRequest): Promise<ChatResponse> {
    const response = await fetch(`${this.baseURL}/v1/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`
      },
      body: JSON.stringify(request)
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    return await response.json();
  }

  // 流式聊天
  async chatStream(
    request: ChatRequest,
    onChunk: (chunk: any) => void,
    onComplete?: (response: ChatResponse) => void
  ): Promise<void> {
    const response = await fetch(`${this.baseURL}/v1/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`
      },
      body: JSON.stringify({ ...request, stream: true })
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const reader = response.body?.getReader();
    const decoder = new TextDecoder();

    if (!reader) {
      throw new Error('Response body is not available');
    }

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
              onChunk(parsed);

              if (parsed.type === 'final' && onComplete) {
                onComplete(parsed.response);
              }
            } catch (e) {
              console.log('Raw data:', data);
            }
          }
        }
      }
    }
  }

  // 对话管理
  async createConversation(title: string, model: string) {
    const response = await fetch(`${this.baseURL}/v1/conversations`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`
      },
      body: JSON.stringify({ title, model })
    });
    return response.json();
  }

  async getConversations(page = 1, limit = 20) {
    const response = await fetch(
      `${this.baseURL}/v1/conversations?page=${page}&limit=${limit}`,
      {
        headers: {
          'Authorization': `Bearer ${this.apiKey}`
        }
      }
    );
    return response.json();
  }

  async getConversationMessages(conversationId: number, page = 1, limit = 50) {
    const response = await fetch(
      `${this.baseURL}/v1/conversations/${conversationId}/messages?page=${page}&limit=${limit}`,
      {
        headers: {
          'Authorization': `Bearer ${this.apiKey}`
        }
      }
    );
    return response.json();
  }

  // 健康检查
  async healthCheck() {
    const response = await fetch(`${this.baseURL}/health`);
    return response.json();
  }

  // 获取模型列表
  async getModels() {
    const response = await fetch(`${this.baseURL}/v1/models`);
    return response.json();
  }
}

// 使用示例
const client = new TopMatLLMClient('http://localhost:3000', 'your_api_key');

// 非流式对话
client.chat({
  message: '你好，请介绍一下自己',
  model: 'qwen-plus',
  conversation_id: 123
}).then(response => {
  console.log('AI回复:', response.content);
  console.log('Token使用:', response.usage);
}).catch(console.error);

// 流式对话
client.chatStream({
  message: '请写一首关于编程的诗',
  model: 'qwen-plus',
  stream: true
},
(chunk) => {
  if (chunk.type === 'content') {
    process.stdout.write(chunk.text);
  } else if (chunk.type === 'final') {
    console.log('\n对话完成!');
    console.log('完整回复:', chunk.response.content);
  }
});

// 创建新对话
client.createConversation('学习Rust', 'qwen-plus').then(conversation => {
  console.log('新对话ID:', conversation.conversation_id);

  // 在新对话中发送消息
  return client.chat({
    message: '请开始教我Rust语言',
    conversation_id: conversation.conversation_id,
    model: 'qwen-plus'
  });
}).then(console.log);
```

### Python 客户端

```python
import requests
import json
from typing import Optional, Dict, Any, Iterator
import sseclient

class TopMatLLMClient:
    def __init__(self, base_url: str = "http://localhost:3000", api_key: str = ""):
        self.base_url = base_url
        self.api_key = api_key
        self.headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}"
        }

    def chat(self, message: str, model: str = "qwen-plus",
             conversation_id: Optional[int] = None,
             stream: bool = False, **kwargs) -> Dict[str, Any]:
        """发送聊天请求"""
        data = {
            "message": message,
            "model": model,
            "stream": stream,
            **kwargs
        }
        if conversation_id:
            data["conversation_id"] = conversation_id

        response = requests.post(
            f"{self.base_url}/v1/chat",
            headers=self.headers,
            json=data
        )
        response.raise_for_status()
        return response.json()

    def chat_stream(self, message: str, model: str = "qwen-plus",
                   conversation_id: Optional[int] = None, **kwargs) -> Iterator[Dict[str, Any]]:
        """流式聊天"""
        data = {
            "message": message,
            "model": model,
            "stream": True,
            **kwargs
        }
        if conversation_id:
            data["conversation_id"] = conversation_id

        response = requests.post(
            f"{self.base_url}/v1/chat",
            headers=self.headers,
            json=data,
            stream=True
        )
        response.raise_for_status()

        client = sseclient.SSEClient(response)
        for event in client.events():
            if event.data:
                try:
                    yield json.loads(event.data)
                except json.JSONDecodeError:
                    continue

    def create_conversation(self, title: str, model: str = "qwen-plus") -> Dict[str, Any]:
        """创建新对话"""
        response = requests.post(
            f"{self.base_url}/v1/conversations",
            headers=self.headers,
            json={"title": title, "model": model}
        )
        response.raise_for_status()
        return response.json()

    def get_conversations(self, page: int = 1, limit: int = 20) -> Dict[str, Any]:
        """获取对话列表"""
        response = requests.get(
            f"{self.base_url}/v1/conversations",
            headers=self.headers,
            params={"page": page, "limit": limit}
        )
        response.raise_for_status()
        return response.json()

    def health_check(self) -> Dict[str, Any]:
        """健康检查"""
        response = requests.get(f"{self.base_url}/health")
        response.raise_for_status()
        return response.json()

    def get_models(self) -> Dict[str, Any]:
        """获取模型列表"""
        response = requests.get(f"{self.base_url}/v1/models")
        response.raise_for_status()
        return response.json()

# 使用示例
if __name__ == "__main__":
    client = TopMatLLMClient(api_key="your_api_key")

    # 健康检查
    health = client.health_check()
    print("服务状态:", health["status"])

    # 非流式对话
    response = client.chat(
        message="你好，请介绍一下Python语言",
        model="qwen-plus"
    )
    print("AI回复:", response["content"])
    print("Token使用:", response["usage"])

    # 流式对话
    print("\n流式回复:")
    for chunk in client.chat_stream(
        message="请用Python写一个Hello World程序",
        model="qwen-plus"
    ):
        if chunk.get("type") == "content":
            print(chunk["text"], end="", flush=True)
        elif chunk.get("type") == "final":
            print("\n\n对话完成!")
            print("完整回复:", chunk["response"]["content"])
            print("Token使用:", chunk["response"]["usage"])

    # 对话管理
    conversation = client.create_conversation("Python学习", "qwen-plus")
    print(f"\n新对话ID: {conversation['conversation_id']}")

    # 在对话中发送消息
    response = client.chat(
        message="请开始教我Python基础语法",
        conversation_id=conversation["conversation_id"],
        model="qwen-plus"
    )
    print("对话回复:", response["content"])
```

## 🏗️ 项目架构

### 目录结构

```
TopMat-LLM/
├── src/
│   ├── main.rs                           # 应用程序入口点
│   └── server/
│       ├── mod.rs                        # 服务器模块声明
│       ├── server.rs                     # 服务器创建和配置
│       ├── routing/
│       │   └── mod.rs                    # 路由配置
│       ├── model_router.rs               # 模型路由器
│       ├── handlers/                     # 请求处理器
│       │   ├── mod.rs                    # 处理器模块声明
│       │   ├── auth.rs                   # 认证处理器
│       │   ├── chat.rs                   # 聊天处理器
│       │   ├── conversations.rs          # 对话管理处理器
│       │   ├── messages.rs               # 消息管理处理器
│       │   ├── models.rs                 # 模型列表处理器
│       │   ├── usage.rs                  # 使用统计处理器
│       │   └── models/                   # 数据模型定义
│       ├── middleware/                   # 中间件
│       │   └── auth.rs                   # 认证中间件
│       ├── database/                     # 数据库模块
│       │   ├── mod.rs                    # 数据库模块声明
│       │   └── connection.rs             # 数据库连接和初始化
│       ├── auth/                         # 认证模块
│       │   ├── mod.rs                    # 认证模块声明
│       │   ├── client.rs                 # 认证客户端
│       │   └── utils.rs                  # 认证工具函数
│       ├── agent/                        # AI提供商实现
│       │   ├── mod.rs                    # 代理模块声明
│       │   ├── qwen.rs                   # 通义千问提供商
│       │   └── ollama.rs                 # Ollama提供商
│       ├── request.rs                    # 请求处理工具
│       ├── responses.rs                  # 响应工具
│       └── models.rs                     # 核心数据结构
├── rig/
│   └── rig-core/                         # 本地 rig-core 依赖
├── tests/                                # 集成测试
├── docs/                                 # 文档目录
│   ├── API_DOCUMENTATION.md              # API文档
│   ├── CLAUDE.md                         # 开发指南
│   └── examples/                         # 示例代码
├── scripts/                              # 脚本文件
│   ├── test_unified_chat.sh              # API测试脚本
│   └── test_chat_api.sh                  # 聊天API测试
├── Cargo.toml                            # 项目依赖和元数据
├── Cargo.lock                            # 依赖锁定文件
├── .env.example                          # 环境变量模板
├── .env                                  # 环境变量（包含实际密钥）
├── .gitignore                            # Git忽略规则
└── README.md                             # 项目说明文档
```

### 技术栈

- **Web框架**：Axum v0.7 - 高性能异步Web框架
- **异步运行时**：Tokio v1 - 异步I/O处理
- **LLM框架**：rig-core v0.21.0 - AI代理框架
- **序列化**：Serde v1.0 - JSON序列化/反序列化
- **数据库**：SQLx v0.8 - 异步数据库访问
- **日志**：Tracing v0.1 - 结构化日志
- **HTTP客户端**：reqwest v0.12 - API调用
- **流处理**：futures-util v0.3 - 异步流处理
- **时间处理**：chrono v0.4 - 日期时间处理
- **环境变量**：dotenvy v0.15 - 环境变量加载

### 架构设计

#### 分层架构
1. **API层** - HTTP请求处理和路由
2. **认证层** - API Key验证和用户管理
3. **业务层** - 聊天逻辑和会话管理
4. **数据层** - 数据库持久化
5. **AI层** - 模型提供商抽象

#### 核心组件

- **ServerState** - 服务器状态管理，包含数据库连接池
- **ModelRouter** - 模型路由器，根据模型名称选择对应处理器
- **AuthMiddleware** - 认证中间件，自动验证API Key
- **ConversationManager** - 对话管理器，处理对话生命周期
- **MessageStorage** - 消息存储，异步持久化聊天记录
- **AI Agents** - AI代理抽象，统一不同提供商接口

#### 数据流

1. **请求流**：HTTP请求 → 认证中间件 → 路由 → 处理器 → AI代理 → 响应
2. **存储流**：聊天请求 → 异步存储 → 数据库 → 统计分析
3. **流式响应**：AI流 → SSE格式 → HTTP流 → 客户端

## 🔧 开发指南

### 开发环境设置

```bash
# 1. 克隆项目
git clone https://github.com/your-org/TopMat-LLM.git
cd TopMat-LLM

# 2. 安装 Rust 工具链
rustup update stable
rustup component add rustfmt clippy

# 3. 安装开发依赖
cargo install cargo-watch cargo-audit

# 4. 设置环境变量
cp .env.example .env
# 编辑 .env 文件

# 5. 初始化数据库
cargo run  # 首次运行会自动创建数据库表
```

### 开发命令

```bash
# 开发模式运行（自动重启）
cargo watch -x run

# 构建项目
cargo build

# 生产构建
cargo build --release

# 运行测试
cargo test

# 运行特定测试
cargo test test_name

# 格式化代码
cargo fmt

# 代码检查
cargo clippy

# 安全审计
cargo audit

# 生成文档
cargo doc --open

# 运行基准测试
cargo bench
```

### 代码质量

确保代码符合项目标准：

```bash
# 完整的质量检查
cargo fmt && cargo clippy && cargo test

# 检查代码覆盖率
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### 添加新的AI提供商

1. **创建提供商模块**：
   ```bash
   # 在 src/server/agent/ 目录下创建新文件
   touch src/server/agent/new_provider.rs
   ```

2. **实现提供商接口**：
   ```rust
   // src/server/agent/new_provider.rs
   use crate::server::{ChatRequest, ChatResponse, ErrorResponse};
   use axum::response::Response;

   pub async fn new_provider_model_with_response(
       request: ChatRequest,
   ) -> Result<(Response, ChatResponse), ErrorResponse> {
       // 实现具体的AI调用逻辑
       todo!()
   }
   ```

3. **注册提供商**：
   ```rust
   // src/server/agent/mod.rs
   pub mod new_provider;

   // src/server/model_router.rs
   router.register("new-provider-model", agent::new_provider::new_provider_model_with_response);
   ```

4. **添加测试**：
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_new_provider() {
           // 添加测试用例
       }
   }
   ```

### 数据库迁移

当需要修改数据库结构时：

1. **更新迁移脚本**：
   ```rust
   // src/server/database/connection.rs
   async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
       // 添加新的迁移SQL
   }
   ```

2. **备份数据**：
   ```bash
   cp data.db data.db.backup
   ```

3. **重新初始化**（开发环境）：
   ```bash
   rm data.db
   cargo run
   ```

### 性能优化

1. **数据库优化**：
   - 使用连接池
   - 添加适当的索引
   - 批量操作优化

2. **异步优化**：
   - 避免阻塞操作
   - 使用 tokio::spawn 处理耗时任务
   - 合理设置超时

3. **内存优化**：
   - 避免大对象克隆
   - 使用引用和借用
   - 及时释放资源

### 测试指南

1. **单元测试**：
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_chat_handler() {
           // 测试逻辑
       }
   }
   ```

2. **集成测试**：
   ```rust
   // tests/integration_test.rs
   use axum_test::TestServer;

   #[tokio::test]
   async fn test_full_chat_flow() {
       // 端到端测试
   }
   ```

3. **API测试**：
   ```bash
   # 使用提供的测试脚本
   chmod +x scripts/test_unified_chat.sh
   ./scripts/test_unified_chat.sh
   ```

## 🚨 错误处理

### 错误响应格式

```json
{
  "error": "error_type",
  "message": "详细错误描述",
  "details": {
    "additional_info": "额外错误信息"
  },
  "timestamp": "2024-10-27T12:00:00Z"
}
```

### 常见错误类型

| 错误代码 | HTTP状态码 | 描述 | 解决方案 |
|----------|------------|------|----------|
| `auth_required` | 401 | 需要API Key认证 | 添加 Authorization header |
| `auth_invalid` | 401 | API Key无效或已过期 | 检查API Key是否正确 |
| `model_not_supported` | 400 | 不支持的模型 | 使用支持的模型名称 |
| `qwen_not_configured` | 503 | 通义千问未配置 | 设置 DASHSCOPE_API_KEY |
| `ollama_not_available` | 503 | Ollama服务不可用 | 启动Ollama服务 |
| `conversation_not_found` | 404 | 对话不存在 | 检查conversation_id |
| `database_error` | 500 | 数据库错误 | 检查数据库连接 |
| `internal_error` | 500 | 内部服务器错误 | 查看服务器日志 |

### 调试技巧

1. **启用详细日志**：
   ```bash
   RUST_LOG=debug cargo run
   ```

2. **查看数据库状态**：
   ```bash
   sqlite3 data.db ".schema"
   sqlite3 data.db "SELECT * FROM conversations LIMIT 5;"
   ```

3. **监控API调用**：
   ```bash
   # 使用 curl 监控响应时间
   curl -w "@curl-format.txt" -X POST http://localhost:3000/v1/chat ...
   ```

## 📊 性能特性

### 性能指标

- **并发连接**：支持 10,000+ 并发连接
- **响应延迟**：P50 < 100ms，P99 < 1s
- **吞吐量**：1,000+ 请求/秒
- **内存使用**：< 100MB（空闲状态）
- **存储效率**：SQLite，支持TB级数据

### 性能优化建议

1. **客户端优化**：
   - 使用连接池
   - 启用HTTP/2
   - 合理设置超时

2. **服务器优化**：
   - 调整工作线程数
   - 优化数据库查询
   - 使用缓存

3. **监控和告警**：
   ```bash
   # 使用健康检查端点
   watch -n 5 curl http://localhost:3000/health

   # 监控资源使用
   top -p $(pgrep TopMat-LLM)
   ```

## 🤝 贡献指南

我们欢迎社区贡献！请遵循以下步骤：

### 贡献流程

1. **Fork 项目**
   ```bash
   # 在 GitHub 上 Fork 项目
   # 然后克隆你的 Fork
   git clone https://github.com/your-username/TopMat-LLM.git
   cd TopMat-LLM
   ```

2. **创建功能分支**
   ```bash
   git checkout -b feature/amazing-feature
   ```

3. **开发和测试**
   ```bash
   # 开发你的功能
   # 确保所有测试通过
   cargo test

   # 检查代码质量
   cargo fmt && cargo clippy
   ```

4. **提交更改**
   ```bash
   git add .
   git commit -m "feat: add amazing feature"
   ```

5. **推送并创建 PR**
   ```bash
   git push origin feature/amazing-feature
   # 在 GitHub 上创建 Pull Request
   ```

### 开发规范

1. **代码风格**：
   - 遵循 Rust 官方代码风格
   - 使用 `cargo fmt` 格式化代码
   - 通过 `cargo clippy` 检查

2. **提交信息**：
   - 使用语义化提交信息
   - 格式：`type(scope): description`
   - 类型：feat, fix, docs, style, refactor, test, chore

3. **测试要求**：
   - 为新功能编写单元测试
   - 添加集成测试（如需要）
   - 确保测试覆盖率 > 80%

4. **文档更新**：
   - 更新相关的 API 文档
   - 添加使用示例
   - 更新 README（如需要）

### 问题报告

报告问题时请提供：

1. **环境信息**：
   - OS 版本
   - Rust 版本
   - 项目版本

2. **重现步骤**：
   - 详细的操作步骤
   - 相关的配置信息
   - 错误信息和日志

3. **期望行为**：
   - 描述你期望发生的情况
   - 提供可能的解决方案

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Rust](https://www.rust-lang.org/) - 系统编程语言
- [Axum](https://github.com/tokio-rs/axum) - Web框架
- [Tokio](https://tokio.rs/) - 异步运行时
- [SQLx](https://github.com/launchbadge/sqlx) - 数据库工具包
- [Serde](https://serde.rs/) - 序列化框架
- [通义千问](https://qwen.aliyun.com/) - AI模型服务
- [Ollama](https://ollama.ai/) - 本地AI模型运行时

## 📞 联系我们

- **项目主页**：https://github.com/your-org/TopMat-LLM
- **问题反馈**：https://github.com/your-org/TopMat-LLM/issues
- **讨论区**：https://github.com/your-org/TopMat-LLM/discussions
- **邮箱**：your-email@example.com

---

**文档更新时间**：2024-10-27
**项目版本**：1.3.0
**API版本**：v1