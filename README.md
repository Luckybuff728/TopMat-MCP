# TopMat-LLM 🦀

[![Rust](https://img.shields.io/badge/rust-2024-blue.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](https://www.docker.com)

基于 Rust Edition 2024 构建的高性能统一 LLM 聊天服务器，为多个 AI 模型提供商提供标准化的 REST API 接口。支持对话管理、数据持久化、实时监控以及 MCP（模型上下文协议）集成，配备专业的材料科学工具和领域特定功能。

## 🚀 核心功能

### 🤖 AI 模型集成
- **多提供商支持**: 统一 API 支持 Qwen、Ollama、OpenAI、Anthropic、Gemini 等 15+ AI 提供商
- **100+ Agent 示例**: 全面的示例集合，展示各种集成模式和最佳实践
- **流式与非流式**: 支持 Server-Sent Events (SSE) 实时响应和传统请求-响应模式

### 🛠️ MCP (Model Context Protocol)
- **专业工具集成**: 19+ 材料科学专用工具 (CalphaMesh、ONNX推理、相场仿真等)
- **自动工具注册**: 编译时宏自动注册，运行时动态发现
- **多传输协议**: 支持 StreamableHTTP、SSE 等多种 MCP 传输协议
- **会话隔离**: 每个连接独立的 CancellationToken，支持多客户端并发

### 💾 数据管理
- **对话持久化**: SQLite 存储，支持完整的对话历史和角色管理
- **使用统计**: 详细的 Token 使用、成本跟踪和性能分析
- **MCP 分析**: 工具调用统计、会话跟踪和成功率监控

### 🔧 开发者友好
- **OpenAPI 文档**: 自动生成的交互式 Swagger UI (`/swagger-ui`)
- **热重载**: `cargo watch` 支持开发时自动重启
- **类型安全**: Rust Edition 2024 + Serde 确保编译时类型检查
- **Docker 优化**: 多阶段构建，Debian Bookworm slim 基础镜像

### 🔒 企业特性
- **API Key 认证**: 外部认证服务集成，支持用户级权限管理
- **CORS 配置**: 开发环境宽松，生产环境可配置
- **错误处理**: 统一错误响应格式，详细的日志记录
- **时区本地化**: 系统统一以 UTC 存储，应用层（日志/响应）自动转换为北京时间 (UTC+8)
- **MCP权鉴**: 新增 `McpAuthMiddleware`，为工具发现与执行提供细粒度的权限控制

## 📋 目录

- [🏃‍♂️ 快速开始](#-快速开始)
- [🛠️ 安装说明](#-安装说明)
- [⚙️ 配置指南](#-配置指南)
- [📡 API 使用](#-api-使用)
- [🛠️ MCP 工具](#-mcp-工具)
- [📚 API 文档](#-api-文档)
- [🧪 开发指南](#-开发指南)
- [🐳 Docker 部署](#-docker-部署)
- [🏗️ 系统架构](#-系统架构)
- [📊 监控与分析](#-监控与分析)
- [🔒 安全性](#-安全性)
- [🤝 贡献指南](#-贡献指南)

## 🏃‍♂️ 快速开始

### 使用 Docker（推荐）

```bash
# 克隆仓库
git clone http://192.168.6.104:3000/fengmengqi/TopMat-LLM-Server.git
cd TopMat-LLM-Server

# 配置环境变量
cp .env.example .env
# 编辑 .env 文件，填入您的 API 密钥

# 使用 Docker Compose 启动
docker build -t 192.168.7.102:5000/topmat-llm:latest -t 192.168.7.102:5000/topmat-llm:v1.0 .

# 使用 Docker Compose 启动
docker-compose up -d

# 查看日志
docker-compose logs -f topmat-llm

# 推送到镜像仓库
docker push 192.168.7.102:5000/topmat-llm:latest

# 检查服务健康状态
curl http://localhost:10007/health
```

### 本地开发

```bash
# 安装 Rust（需要 1.88+ 版本）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆并构建
git clone http://192.168.6.104:3000/fengmengqi/TopMat-LLM-Server.git
cd TopMat-LLM-Server
cargo build --release

# 配置环境
cp .env.example .env
# 编辑 .env 文件进行配置

# 启动服务器
./target/release/TopMat-LLM
```

## 🛠️ 安装说明

### 系统要求

- **Rust**: 1.88+ 或更高版本（Edition 2024）
- **SQLite**: 数据库存储（自动创建）
- **API 密钥**: 至少一个 AI 提供商的 API 密钥
- **操作系统**: Windows、Linux、macOS

### 从源码构建

```bash
# 克隆仓库
git clone http://192.168.6.104:3000/fengmengqi/TopMat-LLM-Server.git
cd TopMat-LLM-Server

# 安装依赖
cargo build

# 运行测试
cargo test

# 启动开发服务器
cargo run
```

### 热重载开发模式

```bash
# 安装 cargo-watch
cargo install cargo-watch

# 文件变更时自动重启
cargo watch -x run
```

## ⚙️ 配置指南

### 环境变量

创建 `.env` 文件并配置以下变量：

```bash
# 服务器配置
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
RUST_LOG=info

# 数据库
DATABASE_URL=sqlite:data.db

# 认证服务
AUTH_API_URL=https://api.topmaterial-tech.com

# AI 提供商 API 密钥（至少需要一个）
DASHSCOPE_API_KEY=your_dashscope_key     # Qwen 模型（推荐）
OLLAMA_BASE_URL=http://localhost:11434   # Ollama 本地模型
OPENAI_API_KEY=your_openai_key          # OpenAI GPT 系列
ANTHROPIC_API_KEY=your_anthropic_key    # Claude 系列
GEMINI_API_KEY=your_gemini_key          # Google Gemini

# MCP 服务器配置（可选）
MCP_SERVER_URL=http://127.0.0.1:10001/mcp
MCP_API_KEY=your_mcp_api_key
```

### 最低配置要求

只需配置以下内容即可开始使用：

```bash
# 最低工作配置
DASHSCOPE_API_KEY=your_qwen_api_key
DATABASE_URL=sqlite:data.db
```

### 默认配置

- **默认模型**: `qwen-plus`
- **服务端口**: `3000` (Docker 容器映射到 `10007`)
- **数据库**: 自动创建 SQLite 文件
- **日志级别**: `info` (可通过 `RUST_LOG` 调整)
- **API 文档**: 启动后访问 `/swagger-ui`
- **时区配置**: 默认 `Asia/Shanghai` (北京时间)，支持日志与响应本地化

## 📡 API 使用

### 身份认证

首先验证您的 API 密钥：

```bash
curl -X POST http://localhost:3000/v1/auth \
  -H "Content-Type: application/json" \
  -d '{"api_key": "your_api_key"}'
```

### 聊天完成

```bash
# 非流式聊天
curl -X POST http://localhost:3000/v1/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key" \
  -d '{
    "model": "qwen-plus",
    "messages": [
      {"role": "user", "content": "你好，你怎么样？"}
    ],
    "stream": false
  }'

# 流式聊天
curl -X POST http://localhost:3000/v1/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key" \
  -d '{
    "model": "qwen-plus",
    "messages": [
      {"role": "user", "content": "介绍一下材料科学"}
    ],
    "stream": true
  }'
```

### 获取可用模型列表

```bash
curl -X GET http://localhost:3000/v1/models \
  -H "Authorization: Bearer your_api_key"
```

### 对话管理

```bash
# 创建新对话
curl -X POST http://localhost:3000/v1/conversations \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key" \
  -d '{"title": "材料科学讨论"}'

# 获取对话历史
curl -X GET http://localhost:3000/v1/conversations/{conversation_id}/messages \
  -H "Authorization: Bearer your_api_key"
```

## 🛠️ MCP 工具

TopMat-LLM 包含 **19+ 专业工具**，专为材料科学和计算工作流设计：

### 🔬 材料科学工具集

- **🧠 think**: 内部推理和思考能力
- **📐 CalphaMesh**: 点/线/Scheil 计算任务集成
  - `calphamesh_submit_point_task` - 提交点计算任务
  - `calphamesh_submit_line_task` - 提交线计算任务
  - `calphamesh_submit_scheil_task` - 提交Scheil任务
  - `calphamesh_get_task_status` - 获取任务状态
  - `calphamesh_list_tasks` - 任务列表查询
- **🔬 仿真工具**: 材料科学仿真系统
  - `TopPhiSimulator` - 涂层沉积模拟
  - `MLPerformancePredictor` - 机器学习性能预测
- **🤖 ONNX 服务**: 机器学习模型推理
  - `onnx_get_models_info` - 获取模型信息
  - `onnx_model_inference` - 模型推理
  - `onnx_get_model_config` - 模型配置查询
- **📚 Dify 集成**: 知识检索和生成
  - `steel_rag` - 钢铁RAG检索
  - `cemented_carbide_rag` - 硬质合金RAG
  - `Al_idme_workflow` - 铝IDME工作流
- **🌊 相场工具**: 物理仿真工具
  - `phase_field_submit_spinodal_decomposition_task` - 调幅分解仿真
  - `phase_field_submit_pvd_simulation_task` - PVD仿真
  - `phase_field_get_task_list` - 任务列表
  - `phase_field_get_task_status` - 任务状态

### 📡 MCP 协议特性

- **多传输协议**: StreamableHTTP、SSE 实时推送
- **自动工具注册**: 编译时宏注册，运行时动态发现
- **会话隔离**: 每个连接独立管理，支持多客户端并发
- **性能监控**: 工具调用统计、执行时间跟踪

### 🔧 使用 MCP 工具

#### REST API 方式
```bash
# 发现可用工具
curl -X GET http://localhost:3000/mcp/ \
  -H "Content-Type: application/json"

# 执行工具
curl -X POST http://localhost:3000/mcp/ \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_api_key" \
  -d '{
    "method": "tools/call",
    "params": {
      "name": "think",
      "arguments": {
        "input": "分析这个材料属性数据"
      }
    }
  }'
```

#### SSE 客户端连接
```bash
# MCP SSE 连接
curl -N http://localhost:3000/sse/ \
  -H "Accept: text/event-stream" \
  -H "Authorization: Bearer your_api_key"
```

## 🏗️ 系统架构

### 核心组件

- **Axum Web 服务器**: 高性能异步 Web 框架
- **SQLite 数据库**: 持久化存储，全面的架构设计
- **MCP 服务器**: 模型上下文协议实现
- **Agent 系统**: 多提供商 AI 模型集成
- **工具注册表**: 动态工具发现和注册

### 请求流程

1. **HTTP 请求** → CORS 层 → 认证中间件
2. **路由** → 处理器（REST API 或 MCP）
3. **处理** → AI Agent 或 MCP 工具执行
4. **响应** → 流式或非流式 → 客户端

### 数据库架构

- `users` - 用户管理，包含订阅等级
- `api_keys` - API 密钥管理，支持过期时间
- `conversations` - 对话元数据
- `messages` - 聊天消息存储
- `usage_statistics` - Token 使用和成本跟踪
- `mcp_sessions` - MCP 会话跟踪
- `mcp_tool_calls` - 工具执行记录

## 🧪 开发指南

### 运行测试

```bash
# 运行所有测试
cargo test

# 显示输出
cargo test -- --nocapture

# 运行特定测试
cargo test test_name

# 并行运行测试
cargo test --release
```

### 代码质量

```bash
# 格式化代码
cargo fmt

# 代码检查
cargo clippy

# 安全审计
cargo audit

# 生成文档
cargo doc --open

# 检查代码覆盖率
cargo tarpaulin --out Html
```

### Agent 示例

项目包含 96+ 个 Agent 示例，展示各种使用模式：

```bash
# 基本 Qwen 使用
cargo run --example qwen_basic

# 带工具的 Agent
cargo run --example agent_with_tools

# MCP 集成
cargo run --example rmcp

# 多提供商示例
cargo run --example openai_basic
cargo run --example anthropic_basic
cargo run --example gemini_basic
```

## 📚 API 文档

### 接口端点

#### 公开端点
- `GET /health` - 健康检查
- `GET /v1/models` - 获取可用模型列表
- `POST /v1/auth` - API 密钥认证

#### 认证端点
- `POST /v1/chat` - 聊天完成（支持流式）
- `GET|POST /v1/conversations` - 对话管理
- `GET /v1/conversations/:id/messages` - 消息历史
- `GET /usage/stats` - 使用统计
- `GET /usage/mcp/stats` - MCP 分析
- `GET /usage/comprehensive` - 综合统计

#### MCP 端点
- `GET /mcp/` - 工具发现（无需认证）
- `POST /mcp/` - 工具执行（需要认证）
- `/sse/` - 服务器发送事件传输

### 响应格式

#### 聊天响应（非流式）
```json
{
  "id": "chat_123",
  "object": "chat.completion",
  "created": 1701234567,
  "model": "qwen-plus",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "你好！今天我能为您做些什么？"
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 15,
    "total_tokens": 25
  }
}
```

#### 流式响应（SSE）
```
data: {"type":"content","text":"你好","finished":false}
data: {"type":"content","text":"！今天","finished":false}
data: {"type":"final","response":{...}}
```

## 🔒 安全性

- **API 密钥认证**: 外部服务集成
- **MCP 认证**: 工具发现与执行分离的认证模式
- **CORS 配置**: 生产环境可配置
- **输入验证**: 全面的数据清理
- **连接池**: 数据库连接管理
- **错误清理**: 生产环境安全响应

## 📊 监控

### 使用分析

```bash
# 获取使用统计
curl -X GET http://localhost:3000/usage/stats \
  -H "Authorization: Bearer your_api_key"

# MCP 专用分析
curl -X GET http://localhost:3000/usage/mcp/stats \
  -H "Authorization: Bearer your_api_key"

# 综合分析
curl -X GET http://localhost:3000/usage/comprehensive \
  -H "Authorization: Bearer your_api_key"
```

### 健康监控

服务器提供全面的健康监控：
- 数据库连接状态
- 外部 API 服务状态
- MCP 工具可用性
- 性能指标
- **本地化日志**: 所有系统日志均采用 ISO 8601 本地时间格式 (UTC+8)

## 🤝 贡献指南

1. Fork 仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 进行更改
4. 为新功能添加测试
5. 运行质量检查 (`cargo fmt && cargo clippy && cargo test`)
6. 提交更改 (`git commit -m 'Add amazing feature'`)
7. 推送到分支 (`git push origin feature/amazing-feature`)
8. 打开 Pull Request

### 开发准则

- 遵循 Rust 最佳实践和约定
- 为新功能添加全面的测试
- 更新 API 更改的文档
- 提交 PR 前确保所有测试通过
- 使用约定式提交消息


## 🆘 支持

- **文档**: 查看 `/docs` 目录获取详细指南
- **示例**: 查看 `src/server/agent/examples/` 了解实现模式
- **问题**: 通过 GitHub Issues 报告错误和请求功能
- **讨论**: 加入我们的 GitHub Discussions 获取社区支持

## 🎯 发展路线图

- [ ] **更多 AI 提供商**: 添加对更多 LLM 提供商的支持
- [ ] **高级分析**: 增强使用分析和仪表板
- [ ] **Web 界面**: 内置 Web 聊天界面
- [ ] **插件系统**: 自定义工具的动态插件加载
- [ ] **多租户**: 支持多个组织
- [ ] **速率限制**: 高级速率限制和配额管理
- [ ] **可观测性**: OpenTelemetry 集成
- [ ] **集群模式**: 多节点部署支持

---

**TopMat-LLM** - 用 Rust 构建材料科学 AI 的未来。🦀✨