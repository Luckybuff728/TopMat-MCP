//! 此模块定义了 [EmbeddingModel] trait，它表示可以为
//! 文档生成嵌入的嵌入模型。
//!
//! 此模块还定义了 [Embedding] 结构体，它表示单个文档嵌入。
//!
//! 最后，此模块定义了 [EmbeddingError] 枚举，它表示在
//! 嵌入生成或处理过程中可能发生的各种错误。

// 使用 futures 库的 BoxFuture 类型
use futures::future::BoxFuture;
// 使用 serde 库进行序列化和反序列化
use serde::{Deserialize, Serialize};

// 派生 Debug 和 thiserror::Error trait
#[derive(Debug, thiserror::Error)]
// 嵌入错误枚举，表示嵌入操作中可能发生的各种错误
pub enum EmbeddingError {
    /// HTTP 错误（例如：连接错误、超时等）
    // HTTP 错误（例如：连接错误、超时等）
    #[error("HttpError: {0}")]
    // HTTP 错误变体，包含 reqwest::Error
    HttpError(#[from] reqwest::Error),

    /// JSON 错误（例如：序列化、反序列化）
    // JSON 错误（例如：序列化、反序列化）
    #[error("JsonError: {0}")]
    // JSON 错误变体，包含 serde_json::Error
    JsonError(#[from] serde_json::Error),

    // URL 解析错误
    #[error("UrlError: {0}")]
    // URL 错误变体，包含 url::ParseError
    UrlError(#[from] url::ParseError),

    /// 处理文档嵌入时的错误
    // 处理文档嵌入时的错误
    #[error("DocumentError: {0}")]
    // 文档错误变体，包含动态错误对象
    DocumentError(Box<dyn std::error::Error + Send + Sync + 'static>),

    /// 解析完成响应时的错误
    // 解析完成响应时的错误
    #[error("ResponseError: {0}")]
    // 响应错误变体，包含错误消息字符串
    ResponseError(String),

    /// 嵌入模型提供商返回的错误
    // 嵌入模型提供商返回的错误
    #[error("ProviderError: {0}")]
    // 提供商错误变体，包含错误消息字符串
    ProviderError(String),
}

/// Trait for embedding models that can generate embeddings for documents.
// 为可以为文档生成嵌入的嵌入模型定义的 trait
pub trait EmbeddingModel: Clone + Sync + Send {
    /// The maximum number of documents that can be embedded in a single request.
    // 单次请求中可以嵌入的最大文档数量
    const MAX_DOCUMENTS: usize;

    /// The number of dimensions in the embedding vector.
    // 获取嵌入向量的维度数量
    fn ndims(&self) -> usize;

    /// Embed multiple text documents in a single request
    // 在单次请求中嵌入多个文本文档
    fn embed_texts(
        &self,
        // 文本迭代器，包含要嵌入的字符串
        texts: impl IntoIterator<Item = String> + Send,
    ) -> impl std::future::Future<Output = Result<Vec<Embedding>, EmbeddingError>> + Send;

    /// Embed a single text document.
    // 嵌入单个文本文档
    fn embed_text(
        &self,
        // 要嵌入的文本字符串
        text: &str,
    ) -> impl std::future::Future<Output = Result<Embedding, EmbeddingError>> + Send {
        // 异步块实现单文档嵌入
        async {
            // 调用多文档嵌入方法，传入包含单个文档的向量
            Ok(self
                .embed_texts(vec![text.to_string()])
                .await?
                // 从结果中弹出第一个（也是唯一的）嵌入
                .pop()
                // 期望至少有一个嵌入结果
                .expect("There should be at least one embedding"))
        }
    }
}

// 动态嵌入模型 trait，用于类型擦除
pub trait EmbeddingModelDyn: Sync + Send {
    // 获取最大文档数量
    fn max_documents(&self) -> usize;
    // 获取嵌入向量维度
    fn ndims(&self) -> usize;
    // 嵌入单个文本，返回装箱的 Future
    fn embed_text<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Embedding, EmbeddingError>>;
    // 嵌入多个文本，返回装箱的 Future
    fn embed_texts(
        &self,
        // 要嵌入的文本向量
        texts: Vec<String>,
    ) -> BoxFuture<'_, Result<Vec<Embedding>, EmbeddingError>>;
}

// 为实现了 EmbeddingModel 的类型实现 EmbeddingModelDyn
impl<T> EmbeddingModelDyn for T
where
    // T 必须实现 EmbeddingModel trait
    T: EmbeddingModel,
{
    // 获取最大文档数量
    fn max_documents(&self) -> usize {
        // 返回类型常量 MAX_DOCUMENTS
        T::MAX_DOCUMENTS
    }

    // 获取嵌入向量维度
    fn ndims(&self) -> usize {
        // 调用自身的 ndims 方法
        self.ndims()
    }

    // 嵌入单个文本，返回装箱的 Future
    fn embed_text<'a>(&'a self, text: &'a str) -> BoxFuture<'a, Result<Embedding, EmbeddingError>> {
        // 将 embed_text 方法的结果装箱
        Box::pin(self.embed_text(text))
    }

    // 嵌入多个文本，返回装箱的 Future
    fn embed_texts(
        &self,
        // 要嵌入的文本向量
        texts: Vec<String>,
    ) -> BoxFuture<'_, Result<Vec<Embedding>, EmbeddingError>> {
        // 将向量转换为迭代器，收集为新的向量，然后调用 embed_texts 并装箱
        Box::pin(self.embed_texts(texts.into_iter().collect::<Vec<_>>()))
    }
}

/// Trait for embedding models that can generate embeddings for images.
// 为可以为图像生成嵌入的嵌入模型定义的 trait
pub trait ImageEmbeddingModel: Clone + Sync + Send {
    /// The maximum number of images that can be embedded in a single request.
    // 单次请求中可以嵌入的最大图像数量
    const MAX_DOCUMENTS: usize;

    /// The number of dimensions in the embedding vector.
    // 获取嵌入向量的维度数量
    fn ndims(&self) -> usize;

    /// Embed multiple images in a single request from bytes.
    // 在单次请求中从字节数据嵌入多个图像
    fn embed_images(
        &self,
        // 图像字节数据迭代器
        images: impl IntoIterator<Item = Vec<u8>> + Send,
    ) -> impl std::future::Future<Output = Result<Vec<Embedding>, EmbeddingError>> + Send;

    /// Embed a single image from bytes.
    // 从字节数据嵌入单个图像
    fn embed_image<'a>(
        &'a self,
        // 图像的字节数据切片
        bytes: &'a [u8],
    ) -> impl std::future::Future<Output = Result<Embedding, EmbeddingError>> + Send {
        // 异步移动闭包实现单图像嵌入
        async move {
            // 调用多图像嵌入方法，传入包含单个图像字节的向量
            Ok(self
                .embed_images(vec![bytes.to_owned()])
                .await?
                // 从结果中弹出第一个（也是唯一的）嵌入
                .pop()
                // 期望至少有一个嵌入结果
                .expect("There should be at least one embedding"))
        }
    }
}

/// Struct that holds a single document and its embedding.
// 保存单个文档及其嵌入的结构体
// 派生 Clone, Default, Deserialize, Serialize, Debug trait
#[derive(Clone, Default, Deserialize, Serialize, Debug)]
pub struct Embedding {
    /// The document that was embedded. Used for debugging.
    // 被嵌入的文档，用于调试
    pub document: String,
    /// The embedding vector
    // 嵌入向量
    pub vec: Vec<f64>,
}

// 为 Embedding 实现 PartialEq trait
impl PartialEq for Embedding {
    // 比较两个嵌入是否相等
    fn eq(&self, other: &Self) -> bool {
        // 只比较文档内容，不比较向量
        self.document == other.document
    }
}

// 为 Embedding 实现 Eq trait
impl Eq for Embedding {}
