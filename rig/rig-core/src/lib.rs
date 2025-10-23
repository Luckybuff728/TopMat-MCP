#![cfg_attr(docsrs, feature(doc_cfg))]
//! Rig 是一个专注于人体工程学和模块化的 Rust 库，用于构建基于 LLM 的应用程序。
//!
//! # 目录
//! - [高级特性](#高级特性)
//! - [简单示例](#简单示例)
//! - [核心概念](#核心概念)
//! - [集成](#集成)
//!
//! # 高级特性
//! - 完全支持 LLM 完成和嵌入工作流
//! - 对 LLM 提供商（如 OpenAI、Cohere）和向量存储（如 MongoDB、内存）提供简单但强大的通用抽象
//! - 以最少的样板代码在您的应用中集成 LLM
//!
//! # 简单示例：
//! ```
//! use rig::{completion::Prompt, providers::openai};
//!
//! #[tokio::main]
//! async fn main() {
//!     // 创建 OpenAI 客户端和代理。
//!     // 这需要设置 `OPENAI_API_KEY` 环境变量。
//!     let openai_client = openai::Client::from_env();
//!
//!     let gpt4 = openai_client.agent("gpt-4").build();
//!
//!     // 提示模型并打印其响应
//!     let response = gpt4
//!         .prompt("Who are you?")
//!         .await
//!         .expect("Failed to prompt GPT-4");
//!
//!     println!("GPT-4: {response}");
//! }
//! ```
//! 注意：使用 `#[tokio::main]` 需要您启用 tokio 的 `macros` 和 `rt-multi-thread` 特性
//! 或者直接使用 `full` 来启用所有特性（`cargo add tokio --features macros,rt-multi-thread`）。
//!
//! # 核心概念
//! ## 完成和嵌入模型
//! Rig 为使用 LLM 和嵌入提供了统一的 API。具体来说，
//! 每个提供商（如 OpenAI、Cohere）都有一个 `Client` 结构体，可用于初始化完成
//! 和嵌入模型。这些模型分别实现了 [CompletionModel](crate::completion::CompletionModel)
//! 和 [EmbeddingModel](crate::embeddings::EmbeddingModel) trait，它们提供了一个通用的、
//! 低级别的接口来创建完成和嵌入请求并执行它们。
//!
//! ## 代理
//! Rig 还以 [Agent](crate::agent::Agent) 类型的形式提供了 LLM 的高级抽象。
//!
//! [Agent](crate::agent::Agent) 类型可用于创建从使用普通模型的简单代理到完整的
//! RAG 系统的任何内容，该系统可用于使用知识库回答问题。
//!
//! ## 向量存储和索引
//! Rig 为使用向量存储和索引提供了通用接口。具体来说，该库
//! 提供了 [VectorStoreIndex](crate::vector_store::VectorStoreIndex)
//! trait，可以实现它来分别定义向量存储和索引。
//! 然后可以将这些用作启用 RAG 的 [Agent](crate::agent::Agent) 的知识库，或
//! 作为使用多个 LLM 或代理的自定义架构中的上下文文档源。
//!
//! # 集成
//! ## 模型提供商
//! Rig 原生支持以下完成和嵌入模型提供商集成：
//! - Anthropic
//! - Azure
//! - Cohere
//! - Deepseek
//! - Galadriel
//! - Gemini
//! - Groq
//! - Huggingface
//! - Hyperbolic
//! - Mira
//! - Mistral
//! - Moonshot
//! - Ollama
//! - Openai
//! - OpenRouter
//! - Perplexity
//! - Together
//! - Voyage AI
//! - xAI
//!
//! 您还可以通过定义实现 [CompletionModel](crate::completion::CompletionModel) 和 [EmbeddingModel](crate::embeddings::EmbeddingModel) trait 的类型来实现自己的模型提供商集成。
//!
//! 向量存储作为独立的配套 crate 提供：
//!
//! - MongoDB: [`rig-mongodb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-mongodb)
//! - LanceDB: [`rig-lancedb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-lancedb)
//! - Neo4j: [`rig-neo4j`](https://github.com/0xPlaygrounds/rig/tree/main/rig-neo4j)
//! - Qdrant: [`rig-qdrant`](https://github.com/0xPlaygrounds/rig/tree/main/rig-qdrant)
//! - SQLite: [`rig-sqlite`](https://github.com/0xPlaygrounds/rig/tree/main/rig-sqlite)
//! - SurrealDB: [`rig-surrealdb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-surrealdb)
//! - Milvus: [`rig-milvus`](https://github.com/0xPlaygrounds/rig/tree/main/rig-milvus)
//! - ScyllaDB: [`rig-scylladb`](https://github.com/0xPlaygrounds/rig/tree/main/rig-scylladb)
//! - AWS S3Vectors: [`rig-s3vectors`](https://github.com/0xPlaygrounds/rig/tree/main/rig-s3vectors)
//!
//! 您还可以通过定义实现 [VectorStoreIndex](crate::vector_store::VectorStoreIndex) trait 的类型来实现自己的向量存储集成。
//!
//! 以下提供商作为独立的配套 crate 提供：
//!
//! - Fastembed: [`rig-fastembed`](https://github.com/0xPlaygrounds/rig/tree/main/rig-fastembed)
//! - Eternal AI: [`rig-eternalai`](https://github.com/0xPlaygrounds/rig/tree/main/rig-eternalai)
//!

extern crate self as rig;

pub mod agent;
#[cfg(feature = "audio")]
#[cfg_attr(docsrs, doc(cfg(feature = "audio")))]
pub mod audio_generation;
pub mod cli_chatbot;
pub mod client;
pub mod completion;
pub mod embeddings;
pub mod extractor;
#[cfg(feature = "image")]
#[cfg_attr(docsrs, doc(cfg(feature = "image")))]
pub mod image_generation;
pub(crate) mod json_utils;
pub mod loaders;
pub mod one_or_many;
pub mod pipeline;
pub mod prelude;
pub mod providers;
pub mod streaming;
pub mod tool;
pub mod tools;
pub mod transcription;
pub mod vector_store;

// 重新导出常用类型和 trait
pub use completion::message;
pub use embeddings::Embed;
pub use one_or_many::{EmptyListError, OneOrMany};

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use rig_derive::Embed;

pub mod telemetry;
