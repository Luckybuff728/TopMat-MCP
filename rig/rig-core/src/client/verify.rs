// 导入验证相关的客户端 trait
use crate::client::{AsVerify, ProviderClient};
// 导入 Future 装箱类型用于异步操作
use futures::future::BoxFuture;
// 导入错误处理宏
use thiserror::Error;

// 派生 Debug 和 Error trait 用于错误处理
#[derive(Debug, Error)]
// 定义验证错误枚举
pub enum VerifyError {
    // 无效认证错误
    #[error("invalid authentication")]
    InvalidAuthentication,
    // 提供商错误，包含错误消息字符串
    #[error("provider error: {0}")]
    ProviderError(String),
    // HTTP 错误，从 reqwest::Error 转换而来
    #[error("http error: {0}")]
    HttpError(
        // 自动从 reqwest::Error 转换
        #[from]
        // 标记为错误源
        #[source]
        reqwest::Error,
    ),
}

/// 可以验证配置的提供商客户端。
/// 客户端类型之间的转换需要 Clone。
// 定义验证客户端 trait，继承 ProviderClient 和 Clone
pub trait VerifyClient: ProviderClient + Clone {
    /// 验证配置。
    // 定义验证方法，返回异步 Future
    fn verify(&self) -> impl Future<Output = Result<(), VerifyError>> + Send;
}

// 定义动态验证客户端 trait，用于运行时多态
pub trait VerifyClientDyn: ProviderClient {
    /// 验证配置。
    // 定义动态验证方法，返回装箱的 Future
    fn verify(&self) -> BoxFuture<'_, Result<(), VerifyError>>;
}

// 为实现了 VerifyClient 的类型自动实现 VerifyClientDyn
impl<T> VerifyClientDyn for T
where
    // T 必须实现 VerifyClient
    T: VerifyClient,
{
    // 实现动态验证方法
    fn verify(&self) -> BoxFuture<'_, Result<(), VerifyError>> {
        // 将具体的验证方法装箱为动态 Future
        Box::pin(self.verify())
    }
}

// 为实现了 VerifyClientDyn 的类型自动实现 AsVerify
impl<T> AsVerify for T
where
    // T 必须实现 VerifyClientDyn、Clone 且生命周期为 'static
    T: VerifyClientDyn + Clone + 'static,
{
    // 实现验证转换方法
    fn as_verify(&self) -> Option<Box<dyn VerifyClientDyn>> {
        // 克隆自身并装箱为动态类型
        Some(Box::new(self.clone()))
    }
}
