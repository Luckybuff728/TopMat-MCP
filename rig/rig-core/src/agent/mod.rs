//! 此模块包含 [Agent] 结构体及其构建器的实现。
//!
//! [Agent] 结构体表示一个 LLM 代理，它将 LLM 模型与前言（系统提示）、
//! 一组上下文文档和一组工具相结合。注意：上下文文档和工具都可以是
//! 静态的（即：它们总是被提供）或动态的（即：它们在提示时进行 RAG 检索）。
//!
//! [Agent] 结构体高度可配置，允许用户定义从
//! 具有特定系统提示的简单机器人到具有一组动态
//! 上下文文档和工具的复杂 RAG 系统的任何内容。
//!
//! [Agent] 结构体实现了 [crate::completion::Completion] 和 [crate::completion::Prompt] trait，
//! 允许它用于生成完成响应和提示。[Agent] 结构体还
//! 实现了 [crate::completion::Chat] trait，允许它用于生成聊天完成。
//!
//! [AgentBuilder] 实现了用于创建 [Agent] 实例的构建器模式。
//! 它允许在构建代理之前配置模型、前言、上下文文档、工具、温度和附加参数。
//!
//! # 示例
//! ```rust
//! use rig::{
//!     completion::{Chat, Completion, Prompt},
//!     providers::openai,
//! };
//!
//! let openai = openai::Client::from_env();
//!
//! // 配置代理
//! let agent = openai.agent("gpt-4o")
//!     .preamble("System prompt")
//!     .context("Context document 1")
//!     .context("Context document 2")
//!     .tool(tool1)
//!     .tool(tool2)
//!     .temperature(0.8)
//!     .additional_params(json!({"foo": "bar"}))
//!     .build();
//!
//! // 使用代理进行完成和提示
//! // 从提示和聊天历史生成聊天完成响应
//! let chat_response = agent.chat("Prompt", chat_history)
//!     .await
//!     .expect("Failed to chat with Agent");
//!
//! // 从简单提示生成提示完成响应
//! let chat_response = agent.prompt("Prompt")
//!     .await
//!     .expect("Failed to prompt the Agent");
//!
//! // 从提示和聊天历史生成完成请求构建器。构建器
//! // 将包含代理的配置（即：前言、上下文文档、工具、
//! // 模型参数等），但这些可以被覆盖。
//! let completion_req_builder = agent.completion("Prompt", chat_history)
//!     .await
//!     .expect("Failed to create completion request builder");
//!
//! let response = completion_req_builder
//!     .temperature(0.9) // 覆盖代理的温度
//!     .send()
//!     .await
//!     .expect("Failed to send completion request");
//! ```
//!
//! RAG 代理示例
//! ```rust
//! use rig::{
//!     completion::Prompt,
//!     embeddings::EmbeddingsBuilder,
//!     providers::openai,
//!     vector_store::{in_memory_store::InMemoryVectorStore, VectorStore},
//! };
//!
//! // 初始化 OpenAI 客户端
//! let openai = openai::Client::from_env();
//!
//! // 初始化 OpenAI 嵌入模型
//! let embedding_model = openai.embedding_model(openai::TEXT_EMBEDDING_ADA_002);
//!
//! // 创建向量存储，计算嵌入并将其加载到存储中
//! let mut vector_store = InMemoryVectorStore::default();
//!
//! let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
//!     .simple_document("doc0", "Definition of a *flurbo*: A flurbo is a green alien that lives on cold planets")
//!     .simple_document("doc1", "Definition of a *glarb-glarb*: A glarb-glarb is a ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.")
//!     .simple_document("doc2", "Definition of a *linglingdong*: A term used by inhabitants of the far side of the moon to describe humans.")
//!     .build()
//!     .await
//!     .expect("Failed to build embeddings");
//!
//! vector_store.add_documents(embeddings)
//!     .await
//!     .expect("Failed to add documents");
//!
//! // 创建向量存储索引
//! let index = vector_store.index(embedding_model);
//!
//! let agent = openai.agent(openai::GPT_4O)
//!     .preamble("
//!         You are a dictionary assistant here to assist the user in understanding the meaning of words.
//!         You will find additional non-standard word definitions that could be useful below.
//!     ")
//!     .dynamic_context(1, index)
//!     .build();
//!
//! // 提示代理并打印响应
//! let response = agent.prompt("What does \"glarb-glarb\" mean?").await
//!     .expect("Failed to prompt the agent");
//! ```
// 定义代理构建器模块
mod builder;
// 定义代理完成模块
mod completion;
// 定义提示请求模块（仅内部可见）
pub(crate) mod prompt_request;
// 定义工具模块
mod tool;

// 重新导出消息文本类型
pub use crate::message::Text;
// 重新导出代理构建器
pub use builder::AgentBuilder;
// 重新导出代理结构体
pub use completion::Agent;
// 重新导出提示钩子
pub use prompt_request::PromptHook;
// 重新导出流式处理相关类型
pub use prompt_request::streaming::{
    FinalResponse, MultiTurnStreamItem, StreamingPromptRequest, stream_to_stdout,
};
// 重新导出提示请求和响应类型
pub use prompt_request::{PromptRequest, PromptResponse};
