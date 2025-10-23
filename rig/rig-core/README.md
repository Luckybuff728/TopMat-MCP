# Rig
Rig is a Rust library for building LLM-powered applications that focuses on ergonomics and modularity.

More information about this crate can be found in the [crate documentation](https://docs/rig-core/latest/rig/).
## Table of contents

- [Rig](#rig)
  - [Table of contents](#table-of-contents)
  - [High-level features](#high-level-features)
  - [Installation](#installation)
  - [Simple example:](#simple-example)
  - [Integrations](#integrations)
  - [Who is using Rig in production?](#who-is-using-rig-in-production)

## Features
- Agentic workflows that can handle multi-turn streaming and prompting
- Full [GenAI Semantic Convention](https://opentelemetry.io/docs/specs/semconv/gen-ai/) compatibility
- 20+ model providers, all under one singular unified interface
- 10+ vector store integrations, all under one singular unified interface
- Full support for LLM completion and embedding workflows
- Support for transcription, audio generation and image generation model capabilities
- Integrate LLMs in your app with minimal boilerplate
- Full WASM compatibility (core library only)

## Installation
```bash
cargo add rig-core
```

## Simple example:
```rust
use rig::{completion::Prompt, providers::openai};

#[tokio::main]
async fn main() {
    // Create OpenAI client and model
    // This requires the `OPENAI_API_KEY` environment variable to be set.
    let openai_client = openai::Client::from_env();

    let gpt4 = openai_client.model("gpt-4").build();

    // Prompt the model and print its response
    let response = gpt4
        .prompt("Who are you?")
        .await
        .expect("Failed to prompt GPT-4");

    println!("GPT-4: {response}");
}
```
Note using `#[tokio::main]` requires you enable tokio's `macros` and `rt-multi-thread` features
or just `full` to enable all features (`cargo add tokio --features macros,rt-multi-thread`).

## Integrations
Rig supports the following LLM providers out of the box:

- Anthropic
- Azure
- Cohere
- Deepseek
- Galadriel
- Gemini
- Groq
- Huggingface
- Hyperbolic
- Mira
- Mistral
- Moonshot
- Ollama
- Openai
- OpenRouter
- Perplexity
- Together
- Voyage AI
- xAI

Vector stores are available as separate companion-crates:

- MongoDB: [`rig-mongodb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-mongodb)
- LanceDB: [`rig-lancedb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-lancedb)
- Neo4j: [`rig-neo4j`](https://github.com/0xPlaygrounds/rig/tree/main/rig-neo4j)
- Qdrant: [`rig-qdrant`](https://github.com/0xPlaygrounds/rig/tree/main/rig-qdrant)
- SQLite: [`rig-sqlite`](https://github.com/0xPlaygrounds/rig/tree/main/rig-sqlite)
- SurrealDB: [`rig-surrealdb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-surrealdb)
- Milvus: [`rig-milvus`](https://github.com/0xPlaygrounds/rig/tree/main/rig-milvus)
- ScyllaDB: [`rig-scylladb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-scylladb)
- AWS S3Vectors: [`rig-s3vectors`](https://github.com/0xPlaygrounds/rig/tree/main/rig-s3vectors)

The following providers are available as separate companion-crates:

- Fastembed: [`rig-fastembed`](https://github.com/0xPlaygrounds/rig/tree/main/rig-fastembed)
- Eternal AI: [`rig-eternalai`](https://github.com/0xPlaygrounds/rig/tree/main/rig-eternalai)

## Who is using Rig in production?
Below is a non-exhaustive list of companies and people who are using Rig in production:
- [Dria Compute Node](https://github.com/firstbatchxyz/dkn-compute-node) - a node that serves computation results within the Dria Knowledge Network
- [The MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk ) - the official Model Context Protocol Rust SDK. Has an example for usage with Rig.
- [Probe](https://github.com/buger/probe) - an AI-friendly, fully local semantic code search tool.
- [NINE](https://github.com/NethermindEth/nine) - Neural Interconnected Nodes Engine, by [Nethermind.](https://www.nethermind.io/)
- [rig-onchain-kit](https://github.com/0xPlaygrounds/rig-onchain-kit) - the Rig Onchain Kit. Intended to make interactions between Solana/EVM and Rig much easier to implement.
- [Linera Protocol](https://github.com/linera-io/linera-protocol) - Decentralized blockchain infrastructure designed for highly scalable, secure, low-latency Web3 applications.
- [Listen](https://github.com/piotrostr/listen) - A framework aiming to become the go-to framework for AI portfolio management agents. Powers [the Listen app.](https://app.listen-rs.com/)

Are you also using Rig in production? [Open an issue](https://www.github.com/0xPlaygrounds/rig/issues) to have your name added!

#### 主要功能模块

**Agent 系统 (`agent/`)**
- `completion.rs`: Agent 的核心实现，支持完成请求处理
- `builder.rs`: Agent 构建器模式实现
- `tool.rs`: 工具集成支持
- `prompt_request/`: 提示请求处理，包括流式处理

**Completion 系统 (`completion/`)**
- `request.rs`: 完成请求的构建和处理
- `message.rs`: 消息类型定义
- 支持多种完成模式：简单提示、聊天、流式处理

**Provider 系统 (`providers/`)**
支持 20+ 个 LLM 提供商：
- OpenAI (GPT-4, GPT-3.5, etc.)
- Anthropic (Claude)
- Google (Gemini)
- Cohere
- Groq
- Mistral
- Together AI
- xAI (Grok)
- DeepSeek
- Moonshot
- Perplexity
- Ollama (本地模型)
- Azure OpenAI
- 等等...

**Embeddings 系统 (`embeddings/`)**
- `embedding.rs`: 嵌入模型接口
- `builder.rs`: 嵌入构建器
- `distance.rs`: 距离计算
- 支持文本嵌入和向量相似度搜索

**Vector Store 系统 (`vector_store/`)**
- `mod.rs`: 向量存储核心接口
- `in_memory_store.rs`: 内存向量存储实现
- `request.rs`: 向量搜索请求定义
- 支持多种向量存储后端

**Tool 系统 (`tool.rs`)**
- 工具定义和执行接口
- 支持静态工具和动态工具（RAG）
- 工具集管理
- MCP (Model Context Protocol) 支持

**Client 系统 (`client/`)**
- 统一的客户端接口
- 多态支持，可动态切换提供商
- 环境变量配置支持

#### 关键特性

1. **Agent 架构**
   - 支持系统提示 (preamble)
   - 静态和动态上下文文档
   - 工具集成
   - 流式处理
   - 多轮对话

2. **RAG (检索增强生成)**
   - 动态上下文检索
   - 向量相似度搜索
   - 文档嵌入和管理

3. **流式处理**
   - 实时响应流
   - 多轮流式对话
   - 工具调用的流式处理

4. **工具系统**
   - 类型安全的工具定义
   - 动态工具发现
   - 工具链支持