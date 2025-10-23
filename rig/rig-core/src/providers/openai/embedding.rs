// 导入父模块的 API 响应、客户端和使用情况类型
use super::{ApiErrorResponse, ApiResponse, Client, completion::Usage};
// 导入嵌入模块
use crate::embeddings;
// 导入嵌入错误类型
use crate::embeddings::EmbeddingError;
// 导入反序列化宏
use serde::Deserialize;
// 导入 JSON 宏
use serde_json::json;

// ================================================================
// OpenAI 嵌入 API
// ================================================================
/// `text-embedding-3-large` 嵌入模型
// text-embedding-3-large 嵌入模型常量
pub const TEXT_EMBEDDING_3_LARGE: &str = "text-embedding-3-large";
/// `text-embedding-3-small` 嵌入模型
// text-embedding-3-small 嵌入模型常量
pub const TEXT_EMBEDDING_3_SMALL: &str = "text-embedding-3-small";
/// `text-embedding-ada-002` 嵌入模型
// text-embedding-ada-002 嵌入模型常量
pub const TEXT_EMBEDDING_ADA_002: &str = "text-embedding-ada-002";

// 派生 Debug 和 Deserialize trait
#[derive(Debug, Deserialize)]
// 嵌入响应结构体
pub struct EmbeddingResponse {
    // 对象类型
    pub object: String,
    // 嵌入数据向量
    pub data: Vec<EmbeddingData>,
    // 模型名称
    pub model: String,
    // 使用情况
    pub usage: Usage,
}

// 为 ApiErrorResponse 实现到 EmbeddingError 的转换
impl From<ApiErrorResponse> for EmbeddingError {
    // 转换方法
    fn from(err: ApiErrorResponse) -> Self {
        // 返回提供商错误
        EmbeddingError::ProviderError(err.message)
    }
}

// 为 ApiResponse<EmbeddingResponse> 实现到 Result<EmbeddingResponse, EmbeddingError> 的转换
impl From<ApiResponse<EmbeddingResponse>> for Result<EmbeddingResponse, EmbeddingError> {
    // 转换方法
    fn from(value: ApiResponse<EmbeddingResponse>) -> Self {
        // 匹配 API 响应
        match value {
            // 成功响应
            ApiResponse::Ok(response) => Ok(response),
            // 错误响应
            ApiResponse::Err(err) => Err(EmbeddingError::ProviderError(err.message)),
        }
    }
}

// 派生 Debug 和 Deserialize trait
#[derive(Debug, Deserialize)]
// 嵌入数据结构体
pub struct EmbeddingData {
    // 对象类型
    pub object: String,
    // 嵌入向量
    pub embedding: Vec<f64>,
    // 索引
    pub index: usize,
}

// 派生 Clone trait
#[derive(Clone)]
// 嵌入模型结构体
pub struct EmbeddingModel {
    // 客户端
    client: Client,
    // 模型名称
    pub model: String,
    // 维度数
    ndims: usize,
}

// 为 EmbeddingModel 实现 embeddings::EmbeddingModel trait
impl embeddings::EmbeddingModel for EmbeddingModel {
    // 最大文档数常量
    const MAX_DOCUMENTS: usize = 1024;

    // 获取维度数
    fn ndims(&self) -> usize {
        // 返回维度数
        self.ndims
    }

    // 嵌入文本方法（支持 worker 特性）
    #[cfg_attr(feature = "worker", worker::send)]
    async fn embed_texts(
        &self,
        documents: impl IntoIterator<Item = String>,
    ) -> Result<Vec<embeddings::Embedding>, EmbeddingError> {
        // 将文档迭代器转换为向量
        let documents = documents.into_iter().collect::<Vec<_>>();

        // 发送 POST 请求到嵌入端点
        let response = self
            .client
            .post("/embeddings")
            .json(&json!({
                "model": self.model,
                "input": documents,
            }))
            .send()
            .await?;

        // 检查响应状态
        if response.status().is_success() {
            // 解析响应为 JSON
            match response.json::<ApiResponse<EmbeddingResponse>>().await? {
                // 成功响应
                ApiResponse::Ok(response) => {
                    // 记录令牌使用情况
                    tracing::info!(target: "rig",
                        "OpenAI embedding token usage: {:?}",
                        response.usage
                    );

                    // 检查响应数据长度是否匹配输入长度
                    if response.data.len() != documents.len() {
                        return Err(EmbeddingError::ResponseError(
                            "Response data length does not match input length".into(),
                        ));
                    }

                    // 转换响应数据为嵌入向量
                    Ok(response
                        .data
                        .into_iter()
                        .zip(documents.into_iter())
                        .map(|(embedding, document)| embeddings::Embedding {
                            // 文档内容
                            document,
                            // 嵌入向量
                            vec: embedding.embedding,
                        })
                        .collect())
                }
                // 错误响应
                ApiResponse::Err(err) => Err(EmbeddingError::ProviderError(err.message)),
            }
        } else {
            // 返回提供商错误
            Err(EmbeddingError::ProviderError(response.text().await?))
        }
    }
}

// EmbeddingModel 的实现
impl EmbeddingModel {
    // 创建新的嵌入模型实例
    pub fn new(client: Client, model: &str, ndims: usize) -> Self {
        Self {
            // 设置客户端
            client,
            // 设置模型名称
            model: model.to_string(),
            // 设置维度数
            ndims,
        }
    }
}
