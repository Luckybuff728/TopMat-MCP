//! 此模块提供用于处理嵌入的功能。
//! 嵌入是文档或其他对象的数值表示，通常用于
//! 自然语言处理（NLP）任务，如文本分类、信息检索和文档相似性。

pub mod builder;
pub mod embed;
pub mod embedding;
pub mod tool;

pub mod distance;
pub use builder::EmbeddingsBuilder;
pub use embed::{Embed, EmbedError, TextEmbedder, to_texts};
pub use embedding::{Embedding, EmbeddingError, EmbeddingModel};
pub use tool::ToolSchema;
