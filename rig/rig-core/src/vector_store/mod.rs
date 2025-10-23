// 导入 futures 的 BoxFuture 类型
use futures::future::BoxFuture;
// 重新导出 VectorSearchRequest
pub use request::VectorSearchRequest;
// 导入 reqwest 的状态码类型
use reqwest::StatusCode;
// 导入 serde 的序列化和反序列化 trait
use serde::{Deserialize, Serialize};
// 导入 serde_json 的 Value 和 json! 宏
use serde_json::{Value, json};

// 导入 crate 根模块的类型
use crate::{
    // 嵌入 trait 和一个或多个类型的包装器
    Embed, OneOrMany,
    // 完成模块的工具定义
    completion::ToolDefinition,
    // 嵌入模块的 Embedding 和 EmbeddingError
    embeddings::{Embedding, EmbeddingError},
    // 工具 trait
    tool::Tool,
};

// 导出内存存储模块
pub mod in_memory_store;
// 导出请求模块
pub mod request;

// 派生 Debug 和 thiserror::Error trait
#[derive(Debug, thiserror::Error)]
// 向量存储错误枚举
pub enum VectorStoreError {
    // 嵌入错误
    #[error("Embedding error: {0}")]
    EmbeddingError(#[from] EmbeddingError),

    /// JSON 错误（例如：序列化、反序列化等）
    // JSON 错误（例如：序列化、反序列化等）
    #[error("Json error: {0}")]
    JsonError(#[from] serde_json::Error),

    // 数据存储错误
    #[error("Datastore error: {0}")]
    DatastoreError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),

    // 缺少 ID 错误
    #[error("Missing Id: {0}")]
    MissingIdError(String),

    // HTTP 请求错误
    #[error("HTTP request error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    // 外部 API 错误
    #[error("External call to API returned an error. Error code: {0} Message: {1}")]
    ExternalAPIError(StatusCode, String),

    // 构建器错误
    #[error("Error while building VectorSearchRequest: {0}")]
    BuilderError(String),
}

/// Trait for inserting documents into a vector store.
// 用于将文档插入向量存储的 trait
pub trait InsertDocuments: Send + Sync {
    /// Insert documents into the vector store.
    ///
    // 将文档插入向量存储
    fn insert_documents<Doc: Serialize + Embed + Send>(
        &self,
        // 要插入的文档和嵌入向量
        documents: Vec<(Doc, OneOrMany<Embedding>)>,
    ) -> impl std::future::Future<Output = Result<(), VectorStoreError>> + Send;
}

/// Trait for vector store indexes
// 向量存储索引的 trait
pub trait VectorStoreIndex: Send + Sync {
    /// Get the top n documents based on the distance to the given query.
    /// The result is a list of tuples of the form (score, id, document)
    // 根据与给定查询的距离获取前 n 个文档
    // 结果是形式为 (分数, id, 文档) 的元组列表
    fn top_n<T: for<'a> Deserialize<'a> + Send>(
        &self,
        // 向量搜索请求
        req: VectorSearchRequest,
    ) -> impl std::future::Future<Output = Result<Vec<(f64, String, T)>, VectorStoreError>> + Send;

    /// Same as `top_n` but returns the document ids only.
    // 与 `top_n` 相同，但只返回文档 ID
    fn top_n_ids(
        &self,
        // 向量搜索请求
        req: VectorSearchRequest,
    ) -> impl std::future::Future<Output = Result<Vec<(f64, String)>, VectorStoreError>> + Send;
}

// 前 N 个结果的类型别名
pub type TopNResults = Result<Vec<(f64, String, Value)>, VectorStoreError>;

// 动态向量存储索引 trait
pub trait VectorStoreIndexDyn: Send + Sync {
    // 获取前 n 个文档，返回装箱的 Future
    fn top_n<'a>(&'a self, req: VectorSearchRequest) -> BoxFuture<'a, TopNResults>;

    // 获取前 n 个文档 ID，返回装箱的 Future
    fn top_n_ids<'a>(
        &'a self,
        // 向量搜索请求
        req: VectorSearchRequest,
    ) -> BoxFuture<'a, Result<Vec<(f64, String)>, VectorStoreError>>;
}

// 为实现了 VectorStoreIndex 的类型实现 VectorStoreIndexDyn
impl<I: VectorStoreIndex> VectorStoreIndexDyn for I {
    // 获取前 n 个文档，返回装箱的 Future
    fn top_n<'a>(
        &'a self,
        req: VectorSearchRequest,
    ) -> BoxFuture<'a, Result<Vec<(f64, String, Value)>, VectorStoreError>> {
        // 装箱异步块
        Box::pin(async move {
            // 调用 top_n 方法并处理结果
            Ok(self
                .top_n::<serde_json::Value>(req)
                .await?
                .into_iter()
                .map(|(score, id, doc)| (score, id, prune_document(doc).unwrap_or_default()))
                .collect::<Vec<_>>())
        })
    }

    // 获取前 n 个文档 ID，返回装箱的 Future
    fn top_n_ids<'a>(
        &'a self,
        req: VectorSearchRequest,
    ) -> BoxFuture<'a, Result<Vec<(f64, String)>, VectorStoreError>> {
        // 装箱 top_n_ids 方法的结果
        Box::pin(self.top_n_ids(req))
    }
}

// 修剪文档，移除过大的数组
fn prune_document(document: serde_json::Value) -> Option<serde_json::Value> {
    // 匹配文档类型
    match document {
        // 对象类型：递归处理每个值
        Value::Object(mut map) => {
            // 创建新的映射，过滤掉修剪后的 None 值
            let new_map = map
                .iter_mut()
                .filter_map(|(key, value)| {
                    prune_document(value.take()).map(|value| (key.clone(), value))
                })
                .collect::<serde_json::Map<_, _>>();

            // 返回修剪后的对象
            Some(Value::Object(new_map))
        }
        // 数组类型：如果长度超过 400，返回 None（移除）
        Value::Array(vec) if vec.len() > 400 => None,
        // 数组类型：递归处理每个元素
        Value::Array(vec) => Some(Value::Array(
            vec.into_iter().filter_map(prune_document).collect(),
        )),
        // 数字类型：直接返回
        Value::Number(num) => Some(Value::Number(num)),
        // 字符串类型：直接返回
        Value::String(s) => Some(Value::String(s)),
        // 布尔类型：直接返回
        Value::Bool(b) => Some(Value::Bool(b)),
        // 空值类型：直接返回
        Value::Null => Some(Value::Null),
    }
}

// 派生 Serialize, Deserialize, Debug trait
#[derive(Serialize, Deserialize, Debug)]
// 向量存储输出结构体
pub struct VectorStoreOutput {
    // 相似性分数
    pub score: f64,
    // 文档 ID
    pub id: String,
    // 文档内容（JSON 值）
    pub document: Value,
}

// 为实现了 VectorStoreIndex 的类型实现 Tool trait
impl<T> Tool for T
where
    // T 必须实现 VectorStoreIndex
    T: VectorStoreIndex,
{
    // 工具名称常量
    const NAME: &'static str = "search_vector_store";

    // 错误类型
    type Error = VectorStoreError;
    // 参数类型
    type Args = VectorSearchRequest;
    // 输出类型
    type Output = Vec<VectorStoreOutput>;

    // 获取工具定义
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        // 创建工具定义
        ToolDefinition {
            // 工具名称
            name: Self::NAME.to_string(),
            // 工具描述
            description:
                "Retrieves the most relevant documents from a vector store based on a query."
                    .to_string(),
            // 工具参数定义（JSON Schema 格式）
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The query string to search for relevant documents in the vector store."
                    },
                    "samples": {
                        "type": "integer",
                        "description": "The maxinum number of samples / documents to retrieve.",
                        "default": 5,
                        "minimum": 1
                    },
                    "threshold": {
                        "type": "number",
                        "description": "Similarity search threshold. If present, any result with a distance less than this may be omitted from the final result."
                    }
                },
                "required": ["query", "samples"]
            }),
        }
    }

    // 执行工具调用
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 获取搜索结果
        let results = self.top_n(args).await?;
        // 转换为 VectorStoreOutput 格式
        Ok(results
            .into_iter()
            .map(|(score, id, document)| VectorStoreOutput {
                score,
                id,
                document,
            })
            .collect())
    }
}
