# TopMat-LLM

[![Rust](https://img.shields.io/badge/rust-2024%20Edition-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

**TopMat-LLM** 是一个基于 Rust 构建的统一大语言模型聊天服务器，提供标准化的 REST API 接口，支持多种 AI 模型提供商，具备流式和非流式响应能力。

## ✨ 核心特性

- 🚀 **统一接口** - 单一 `/chat` 端点处理所有模型交互
- 🌊 **流式响应** - 支持实时流式输出 (SSE)
- 🤖 **多模型支持** - 通义千问、Ollama 本地模型等
- ⚡ **高性能** - 基于 Tokio 异步运行时
- 🛡️ **类型安全** - Rust 类型系统保证可靠性
- 📊 **使用统计** - 详细的 Token 使用情况跟踪
- 🔄 **会话管理** - 支持会话上下文保持
- 🎯 **智能路由** - 根据参数自动选择流式/非流式响应

## 🚀 快速开始

### 1. 环境准备

确保您的系统已安装：
- **Rust 2024 Edition** 或更高版本
- **Git**

### 2. 克隆项目

```bash
git clone <repository-url>
cd TopMat-LLM
```

### 3. 配置环境变量

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env 文件，填入您的 API 密钥
```

编辑 `.env` 文件：
```bash
# 服务器配置（可选）
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
RUST_LOG=info

# 通义千问 API 密钥（必需，如果使用通义千问模型）
DASHSCOPE_API_KEY=your_dashscope_api_key_here

# 其他提供商 API 密钥（可选）
# OPENAI_API_KEY=your_openai_api_key_here
# ANTHROPIC_API_KEY=your_anthropic_api_key_here
# COHERE_API_KEY=your_cohere_api_key_here
```


### 4. 启动服务器

```bash
# 构建并运行服务器
cargo run
```

服务器将在 `http://localhost:3000` 启动。

## 📖 API 使用指南

### 基础请求格式

```json
{
  "message": "string",           // 必需 - 用户消息
  "stream": boolean,             // 可选 - 是否流式响应，默认 false
  "model": "string",             // 可选 - 模型名称，默认 "qwen3:4b"
  "system_prompt": "string",     // 可选 - 系统提示词
  "temperature": number,         // 可选 - 温度参数 (0.0-1.0)
  "max_tokens": number,          // 可选 - 最大 Token 数
  "session_id": "string",        // 可选 - 会话 ID
  "metadata": {}                 // 可选 - 额外元数据
}
```

### 非流式请求示例

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "你好，请介绍一下Rust语言",
    "stream": false,
    "model": "qwen3:4b"
  }'
```

**响应格式**：
```json
{
  "content": "Rust是一门系统编程语言...",
  "model": "qwen3:4b",
  "usage": {
    "prompt_tokens": 25,
    "completion_tokens": 150,
    "total_tokens": 175
  },
  "session_id": "session_12345",
  "timestamp": "2024-01-01T12:00:00Z",
  "metadata": {}
}
```

### 流式请求示例

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "写一个Rust Hello World程序",
    "stream": true,
    "model": "qwen3:4b"
  }'
```

**流式响应** (Server-Sent Events)：
```
data: {"type":"content","text":"Rust","finished":false}

data: {"type":"content","text":" 中的","finished":false}

data: {"type":"content","text":" Hello World","finished":false}

data: {"type":"final","response":{"content":"Rust 中的 Hello World 程序...","model":"qwen3:4b","usage":{...},"timestamp":"..."}}
```

## 🤖 支持的模型

### 通义千问模型
- `qwen` / `qwen-plus` - 通义千问 Plus 版本
- `qwen` / `qwen-max` - 通义千问 Max 版本
- `qwen` / `qwen-turbo` - 通义千问 Turbo 模型
- `qwen` / `qwen-flash` - 通义千问 flash 模型
- `qwen` / `qwq-plus` - 通义千问深度推理模型
- **要求**：配置 `DASHSCOPE_API_KEY`

### Ollama 本地模型
- `qwen3:4b` - 本地通义千问3代4B模型
- `llama3:latest` - Llama 3 最新版本
- **要求**：安装并运行 Ollama 服务

#### Ollama 配置（可选）

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

### JavaScript 客户端

```javascript
// 非流式请求
async function chat(message) {
  const response = await fetch('http://localhost:3000/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      message: message,
      stream: false,
      model: 'qwen3:4b'
    })
  });

  const data = await response.json();
  console.log('Response:', data.content);
  return data;
}

// 流式请求
async function chatStream(message, onChunk) {
  const response = await fetch('http://localhost:3000/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      message: message,
      stream: true,
      model: 'qwen3:4b'
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
        const data = line.slice(6);
        if (data.trim()) {
          try {
            const parsed = JSON.parse(data);
            onChunk(parsed);
          } catch (e) {
            console.log('Raw data:', data);
          }
        }
      }
    }
  }
}

// 使用示例
chat('你好，请介绍一下自己').then(console.log);

chatStream('请写一首关于编程的诗', (chunk) => {
  if (chunk.type === 'content') {
    process.stdout.write(chunk.text);
  } else if (chunk.type === 'final') {
    console.log('\nComplete:', chunk.response);
  }
});
```


## 🏗️ 项目架构

### 目录结构

```
TopMat-LLM/
├── src/
│   ├── main.rs                 # 应用程序入口点
│   └── server/
│       ├── mod.rs              # 模块声明
│       ├── chat.rs             # 聊天 API 处理器和路由
│       ├── models.rs           # 数据结构定义
│       ├── request.rs          # 请求处理工具
│       └── agent/              # AI 提供商实现
│           ├── mod.rs          # 代理模块声明
│           ├── qwen.rs         # 通义千问提供商
│           └── ollama.rs       # Ollama 本地模型提供商
├── rig/
│   └── rig-core/              # 本地 rig-core 依赖
├── Cargo.toml                  # 项目依赖和元数据
├── Cargo.lock                  # 依赖锁定文件
├── .env                        # 环境变量（包含实际 API 密钥）
├── .env.example               # 环境变量模板
├── .gitignore                  # Git 忽略规则
└── docs/                      # 文档目录
    ├── README_UNIFIED_CHAT.md  # 统一聊天服务器文档
    ├── UNIFIED_CHAT_API.md     # API 文档
    ├── agent.md                # 代理文档
    └── design/                 # 设计文档
        ├── rig设计.md
        └── 后端设计方案_v0.3.md
```

### 技术栈

- **核心框架**：Axum v0.7 - 高性能 Web 框架
- **异步运行时**：Tokio v1 - 异步 I/O 处理
- **LLM 框架**：rig-core v0.21.0 - AI 代理框架
- **序列化**：Serde v1.0 - JSON 序列化/反序列化
- **日志**：Tracing v0.1 - 结构化日志
- **HTTP 客户端**：reqwest v0.14 - API 调用
- **流处理**：async-stream v0.3 - SSE 流支持

### 核心组件

- **ServerState** - 服务器状态管理
- **ChatRequest/ChatResponse** - 请求响应数据结构
- **StreamChunk** - 流式响应数据块
- **chat_handler** - 主要请求处理器
- **Agent 实现** - 不同 AI 提供商的统一接口

## 🔧 开发指南

### 构建项目

```bash
# 开发构建
cargo build

# 生产构建
cargo build --release

# 运行测试
cargo test
```

### 代码质量

```bash
# 格式化代码
cargo fmt

# 检查代码
cargo clippy

# 生成文档
cargo doc --open
```

### 添加新的 AI 提供商

1. 在 `src/server/agent/` 目录下创建新的提供商文件
2. 实现相应的处理函数
3. 在 `src/server/agent/mod.rs` 中添加模块声明
4. 在 `src/server/chat.rs` 中添加路由逻辑

## 🚨 错误处理

### 错误响应格式

```json
{
  "error": "error_type",
  "message": "详细错误描述",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### 常见错误类型

- `qwen_not_configured` - 通义千问 API 密钥未配置
- `model_not_found` - 请求的模型不存在
- `chat_failed` - 聊天处理失败
- `streaming_chat_failed` - 流式聊天失败
- `invalid_request` - 请求格式无效

## 📊 性能特性

- **高并发**：基于 Tokio 异步运行时，支持数千并发连接
- **低延迟**：优化的请求处理管道，毫秒级响应
- **内存效率**：Rust 零成本抽象和内存安全保证
- **流式优化**：实时数据流，无需等待完整响应

## 🤝 贡献指南

我们欢迎社区贡献！请遵循以下步骤：

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### 开发规范

- 遵循 Rust 官方代码风格
- 添加适当的文档注释
- 编写单元测试
- 确保所有检查通过 (`cargo fmt && cargo clippy && cargo test`)

