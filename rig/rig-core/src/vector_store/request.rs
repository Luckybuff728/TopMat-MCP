// 导入 serde 的序列化和反序列化 trait
use serde::{Deserialize, Serialize};

// 导入父模块的 VectorStoreError
use super::VectorStoreError;

/// 向量搜索请求 - 在 [`super::VectorStoreIndex`] trait 中使用。
// 向量搜索请求 - 在 [`super::VectorStoreIndex`] trait 中使用
// 派生 Clone, Serialize, Deserialize, Debug trait
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VectorSearchRequest {
    /// 将被嵌入并用于相似性搜索的查询。
    // 将被嵌入并用于相似性搜索的查询
    query: String,
    /// 可能返回的最大样本数。如果添加相似性搜索阈值，如果没有足够的结果满足阈值，您可能收到少于输入数量的结果。
    // 可能返回的最大样本数。如果添加相似性搜索阈值，如果没有足够的结果满足阈值，您可能收到少于输入数量的结果
    samples: u64,
    /// 相似性搜索阈值。如果存在，任何距离小于此值的结果都可能从最终结果中省略。
    // 相似性搜索阈值。如果存在，任何距离小于此值的结果都可能从最终结果中省略
    threshold: Option<f64>,
    /// 向量存储所需的任何附加参数。
    // 向量存储所需的任何附加参数
    additional_params: Option<serde_json::Value>,
}

// 为 VectorSearchRequest 实现方法
impl VectorSearchRequest {
    /// 创建一个 [`VectorSearchRequestBuilder`]，您可以使用它来实例化此结构体。
    // 创建一个 [`VectorSearchRequestBuilder`]，您可以使用它来实例化此结构体
    pub fn builder() -> VectorSearchRequestBuilder {
        // 返回默认的构建器实例
        VectorSearchRequestBuilder::default()
    }

    /// 将被嵌入并用于相似性搜索的查询。
    // 获取将被嵌入并用于相似性搜索的查询
    pub fn query(&self) -> &str {
        // 返回查询字符串的引用
        &self.query
    }

    /// 可能返回的最大样本数。如果添加相似性搜索阈值，如果没有足够的结果满足阈值，您可能收到少于输入数量的结果。
    // 获取可能返回的最大样本数。如果添加相似性搜索阈值，如果没有足够的结果满足阈值，您可能收到少于输入数量的结果
    pub fn samples(&self) -> u64 {
        // 返回样本数量
        self.samples
    }

    // 获取相似性搜索阈值
    pub fn threshold(&self) -> Option<f64> {
        // 返回阈值（可能为 None）
        self.threshold
    }
}

/// 用于实例化 [`VectorSearchRequest`] 的构建器结构体。
// 用于实例化 [`VectorSearchRequest`] 的构建器结构体
// 派生 Clone, Serialize, Deserialize, Debug, Default trait
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct VectorSearchRequestBuilder {
    // 查询字符串（可选）
    query: Option<String>,
    // 样本数量（可选）
    samples: Option<u64>,
    // 相似性阈值（可选）
    threshold: Option<f64>,
    // 附加参数（可选）
    additional_params: Option<serde_json::Value>,
}

// 为 VectorSearchRequestBuilder 实现方法
impl VectorSearchRequestBuilder {
    /// 设置查询（然后将被嵌入）
    // 设置查询（然后将被嵌入）
    pub fn query<T>(mut self, query: T) -> Self
    where
        // T 必须能够转换为 String
        T: Into<String>,
    {
        // 将查询转换为 String 并存储
        self.query = Some(query.into());
        // 返回 self 以支持链式调用
        self
    }

    // 设置样本数量
    pub fn samples(mut self, samples: u64) -> Self {
        // 存储样本数量
        self.samples = Some(samples);
        // 返回 self 以支持链式调用
        self
    }

    // 设置相似性阈值
    pub fn threshold(mut self, threshold: f64) -> Self {
        // 存储阈值
        self.threshold = Some(threshold);
        // 返回 self 以支持链式调用
        self
    }

    // 设置附加参数
    pub fn additional_params(
        mut self,
        // 附加参数的 JSON 值
        params: serde_json::Value,
    ) -> Result<Self, VectorStoreError> {
        // 存储附加参数
        self.additional_params = Some(params);
        // 返回 Ok(self) 以支持链式调用
        Ok(self)
    }

    // 构建 VectorSearchRequest 实例
    pub fn build(self) -> Result<VectorSearchRequest, VectorStoreError> {
        // 检查查询是否已设置
        let Some(query) = self.query else {
            // 如果查询未设置，返回错误
            return Err(VectorStoreError::BuilderError(
                "`query` is a required variable for building a vector search request".into(),
            ));
        };

        // 检查样本数量是否已设置
        let Some(samples) = self.samples else {
            // 如果样本数量未设置，返回错误
            return Err(VectorStoreError::BuilderError(
                "`samples` is a required variable for building a vector search request".into(),
            ));
        };

        // 处理附加参数
        let additional_params = if let Some(params) = self.additional_params {
            // 检查附加参数是否为 JSON 对象
            if !params.is_object() {
                // 如果不是对象，返回错误
                return Err(VectorStoreError::BuilderError(
                    "Expected JSON object for additional params, got something else".into(),
                ));
            }
            // 返回参数
            Some(params)
        } else {
            // 如果没有附加参数，返回 None
            None
        };

        // 创建并返回 VectorSearchRequest 实例
        Ok(VectorSearchRequest {
            query,
            samples,
            threshold: self.threshold,
            additional_params,
        })
    }
}
