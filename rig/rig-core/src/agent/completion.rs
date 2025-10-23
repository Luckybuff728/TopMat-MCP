// 导入提示请求相关类型
use super::prompt_request::{self, PromptRequest};
// 导入完成模型和流式处理相关类型
use crate::{
    agent::prompt_request::streaming::StreamingPromptRequest,
    completion::{
        Chat, Completion, CompletionError, CompletionModel, CompletionRequestBuilder, Document,
        GetTokenUsage, Message, Prompt, PromptError,
    },
    message::ToolChoice,
    streaming::{StreamingChat, StreamingCompletion, StreamingPrompt},
    tool::ToolSet,
    vector_store::{VectorStoreError, request::VectorSearchRequest},
};
// 导入流处理扩展方法
use futures::{StreamExt, TryStreamExt, stream};
// 导入集合和原子引用计数相关类型
use std::{collections::HashMap, sync::Arc};

// 定义未知代理名称常量
const UNKNOWN_AGENT_NAME: &str = "Unnamed Agent";

/// 表示 LLM 代理的结构体。代理是 LLM 模型与前言
/// （即：系统提示）以及一组静态上下文文档和工具的组合。
/// 在提示时，所有上下文文档和工具都会提供给代理。
///
/// # 示例
/// ```
/// use rig::{completion::Prompt, providers::openai};
///
/// let openai = openai::Client::from_env();
///
/// let comedian_agent = openai
///     .agent("gpt-4o")
///     .preamble("You are a comedian here to entertain the user using humour and jokes.")
///     .temperature(0.9)
///     .build();
///
/// let response = comedian_agent.prompt("Entertain me!")
///     .await
///     .expect("Failed to prompt the agent");
/// ```
// 派生 Clone trait，允许代理结构体被克隆
#[derive(Clone)]
// 标记为非穷尽结构体，表示未来可能添加更多字段
#[non_exhaustive]
// 定义代理结构体，支持泛型完成模型
pub struct Agent<M>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    /// 用于日志记录和调试的代理名称
    // 可选的代理名称，用于标识和调试
    pub name: Option<String>,
    /// 完成模型（例如：OpenAI 的 gpt-3.5-turbo-1106，Cohere 的 command-r）
    // 完成模型实例，使用原子引用计数包装
    pub model: Arc<M>,
    /// 系统提示
    // 可选的系统提示，定义代理的行为
    pub preamble: Option<String>,
    /// 代理始终可用的上下文文档
    // 静态上下文文档列表
    pub static_context: Vec<Document>,
    /// 代理始终可用的工具（通过其名称标识）
    // 静态工具名称列表
    pub static_tools: Vec<String>,
    /// 模型的温度
    // 控制模型输出的随机性
    pub temperature: Option<f64>,
    /// 完成的最大 token 数
    // 限制生成响应的最大长度
    pub max_tokens: Option<u64>,
    /// 传递给模型的附加参数
    // 额外的模型配置参数
    pub additional_params: Option<serde_json::Value>,
    /// 向量存储列表，包含样本数量
    // 动态上下文存储，包含样本数量和向量存储索引
    pub dynamic_context: Arc<Vec<(usize, Box<dyn crate::vector_store::VectorStoreIndexDyn>)>>,
    /// 动态工具
    // 动态工具存储，包含样本数量和向量存储索引
    pub dynamic_tools: Arc<Vec<(usize, Box<dyn crate::vector_store::VectorStoreIndexDyn>)>>,
    /// 实际工具实现
    // 工具集，包含所有可用的工具
    pub tools: Arc<ToolSet>,
    /// 底层 LLM 是否应在提供响应之前强制使用工具。
    // 工具选择策略，控制模型是否必须使用工具
    pub tool_choice: Option<ToolChoice>,
}

// 为代理实现方法，支持泛型完成模型
impl<M> Agent<M>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    /// Returns the name of the agent.
    // 返回代理名称的内部方法
    pub(crate) fn name(&self) -> &str {
        // 返回代理名称，如果为空则返回未知代理名称
        self.name.as_deref().unwrap_or(UNKNOWN_AGENT_NAME)
    }
}

// 为代理实现 Completion trait，支持泛型完成模型
impl<M> Completion<M> for Agent<M>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    // 异步函数：创建完成请求构建器
    async fn completion(
        // 接收自身引用
        &self,
        // 提示消息，必须实现 Into<Message> 和 Send trait
        prompt: impl Into<Message> + Send,
        // 聊天历史
        chat_history: Vec<Message>,
    ) -> Result<CompletionRequestBuilder<M>, CompletionError> {
        // 将提示转换为消息
        let prompt = prompt.into();

        // Find the latest message in the chat history that contains RAG text
        // 在聊天历史中查找包含 RAG 文本的最新消息
        // 获取当前提示的 RAG 文本
        let rag_text = prompt.rag_text();
        // 如果当前提示没有 RAG 文本，则从聊天历史中查找
        let rag_text = rag_text.or_else(|| {
            // 从聊天历史中反向迭代查找包含 RAG 文本的消息
            chat_history
                .iter()
                .rev()
                .find_map(|message| message.rag_text())
        });

        // 创建完成请求构建器
        let completion_request = self
            // 调用模型的完成请求方法
            .model
            .completion_request(prompt)
            // 设置消息历史
            .messages(chat_history)
            // 设置温度参数
            .temperature_opt(self.temperature)
            // 设置最大 token 数
            .max_tokens_opt(self.max_tokens)
            // 设置附加参数
            .additional_params_opt(self.additional_params.clone())
            // 设置静态上下文文档
            .documents(self.static_context.clone());
        // 如果有前言，则添加到完成请求中
        let completion_request = if let Some(preamble) = &self.preamble {
            // 添加前言到完成请求
            completion_request.preamble(preamble.to_owned())
        } else {
            // 直接使用完成请求
            completion_request
        };

        // If the agent has RAG text, we need to fetch the dynamic context and tools
        // 如果代理有 RAG 文本，我们需要获取动态上下文和工具
        // 根据是否有 RAG 文本进行匹配处理
        let agent = match &rag_text {
            // 如果有 RAG 文本
            Some(text) => {
                // 获取动态上下文
                let dynamic_context = stream::iter(self.dynamic_context.iter())
                    // 对每个动态上下文进行处理
                    .then(|(num_sample, index)| async {
                        // 创建向量搜索请求
                        let req = VectorSearchRequest::builder().query(text).samples(*num_sample as u64).build().expect("Creating VectorSearchRequest here shouldn't fail since the query and samples to return are always present");
                        // 执行向量搜索并处理结果
                        Ok::<_, VectorStoreError>(
                            index
                                // 获取前 N 个结果
                                .top_n(req)
                                .await?
                                // 转换为迭代器
                                .into_iter()
                                // 映射每个结果
                                .map(|(_, id, doc)| {
                                    // Pretty print the document if possible for better readability
                                    // 如果可能，美化打印文档以提高可读性
                                    let text = serde_json::to_string_pretty(&doc)
                                        .unwrap_or_else(|_| doc.to_string());

                                    // 创建文档结构体
                                    Document {
                                        // 设置文档 ID
                                        id,
                                        // 设置文档文本
                                        text,
                                        // 初始化附加属性为空映射
                                        additional_props: HashMap::new(),
                                    }
                                })
                                // 收集为向量
                                .collect::<Vec<_>>(),
                        )
                    })
                    // 折叠所有结果
                    .try_fold(vec![], |mut acc, docs| async {
                        // 扩展累积结果
                        acc.extend(docs);
                        // 返回累积结果
                        Ok(acc)
                    })
                    // 等待异步操作完成
                    .await
                    // 将向量存储错误映射为完成错误
                    .map_err(|e| CompletionError::RequestError(Box::new(e)))?;

                // 获取动态工具
                let dynamic_tools = stream::iter(self.dynamic_tools.iter())
                    // 对每个动态工具进行处理
                    .then(|(num_sample, index)| async {
                        // 创建向量搜索请求
                        let req = VectorSearchRequest::builder().query(text).samples(*num_sample as u64).build().expect("Creating VectorSearchRequest here shouldn't fail since the query and samples to return are always present");
                        // 执行向量搜索并处理结果
                        Ok::<_, VectorStoreError>(
                            index
                                // 获取前 N 个 ID
                                .top_n_ids(req)
                                .await?
                                // 转换为迭代器
                                .into_iter()
                                // 映射获取 ID
                                .map(|(_, id)| id)
                                // 收集为向量
                                .collect::<Vec<_>>(),
                        )
                    })
                    // 折叠所有结果
                    .try_fold(vec![], |mut acc, docs| async {
                        // 遍历每个文档 ID
                        for doc in docs {
                            // 如果工具集中存在该工具
                            if let Some(tool) = self.tools.get(&doc) {
                                // 获取工具定义并添加到累积结果
                                acc.push(tool.definition(text.into()).await)
                            } else {
                                // 记录警告：工具实现未找到
                                tracing::warn!("Tool implementation not found in toolset: {}", doc);
                            }
                        }
                        // 返回累积结果
                        Ok(acc)
                    })
                    // 等待异步操作完成
                    .await
                    // 将向量存储错误映射为完成错误
                    .map_err(|e| CompletionError::RequestError(Box::new(e)))?;

                // 获取静态工具
                let static_tools = stream::iter(self.static_tools.iter())
                    // 过滤映射每个工具名称
                    .filter_map(|toolname| async move {
                        // 如果工具集中存在该工具
                        if let Some(tool) = self.tools.get(toolname) {
                            // 获取工具定义
                            Some(tool.definition(text.into()).await)
                        } else {
                            // 记录警告：工具实现未找到
                            tracing::warn!(
                                "Tool implementation not found in toolset: {}",
                                toolname
                            );
                            // 返回 None
                            None
                        }
                    })
                    // 收集为向量
                    .collect::<Vec<_>>()
                    // 等待异步操作完成
                    .await;

                // 返回带动态上下文和工具的完成请求
                completion_request
                    // 设置动态上下文文档
                    .documents(dynamic_context)
                    // 设置工具（静态工具和动态工具合并）
                    .tools([static_tools.clone(), dynamic_tools].concat())
            }
            // 如果没有 RAG 文本
            None => {
                // 获取静态工具（无 RAG 文本情况）
                let static_tools = stream::iter(self.static_tools.iter())
                    // 过滤映射每个工具名称
                    .filter_map(|toolname| async move {
                        // 如果工具集中存在该工具
                        if let Some(tool) = self.tools.get(toolname) {
                            // TODO: tool definitions should likely take an `Option<String>`
                            // TODO: 工具定义可能应该接受 `Option<String>`
                            // 获取工具定义（使用空字符串）
                            Some(tool.definition("".into()).await)
                        } else {
                            // 记录警告：工具实现未找到
                            tracing::warn!(
                                "Tool implementation not found in toolset: {}",
                                toolname
                            );
                            // 返回 None
                            None
                        }
                    })
                    // 收集为向量
                    .collect::<Vec<_>>()
                    // 等待异步操作完成
                    .await;

                // 返回带静态工具的完成请求
                completion_request.tools(static_tools)
            }
        };

        // 返回完成请求构建器
        Ok(agent)
    }
}

// Here, we need to ensure that usage of `.prompt` on agent uses these redefinitions on the opaque
//  `Prompt` trait so that when `.prompt` is used at the call-site, it'll use the more specific
//  `PromptRequest` implementation for `Agent`, making the builder's usage fluent.
//
// References:
//  - https://github.com/rust-lang/rust/issues/121718 (refining_impl_trait)
// 这里，我们需要确保在代理上使用 `.prompt` 时使用这些对不透明 `Prompt` trait 的重新定义，
// 这样当在调用点使用 `.prompt` 时，它将使用针对 `Agent` 的更具体的 `PromptRequest` 实现，
// 使构建器的使用变得流畅。
//
// 参考：
//  - https://github.com/rust-lang/rust/issues/121718 (refining_impl_trait)

// 允许细化实现 trait
#[allow(refining_impl_trait)]
// 为代理实现 Prompt trait
impl<M> Prompt for Agent<M>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    // 提示方法的实现
    fn prompt(
        // 接收自身引用
        &self,
        // 提示消息，必须实现 Into<Message> 和 Send trait
        prompt: impl Into<Message> + Send,
    ) -> PromptRequest<'_, prompt_request::Standard, M, ()> {
        // 创建新的提示请求
        PromptRequest::new(self, prompt)
    }
}

// 允许细化实现 trait
#[allow(refining_impl_trait)]
// 为代理引用实现 Prompt trait
impl<M> Prompt for &Agent<M>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    // 添加追踪检测，跳过 self 和 prompt 参数，记录代理名称
    #[tracing::instrument(skip(self, prompt), fields(agent_name = self.name()))]
    // 提示方法的实现
    fn prompt(
        // 接收自身引用
        &self,
        // 提示消息，必须实现 Into<Message> 和 Send trait
        prompt: impl Into<Message> + Send,
    ) -> PromptRequest<'_, prompt_request::Standard, M, ()> {
        // 创建新的提示请求（解引用代理）
        PromptRequest::new(*self, prompt)
    }
}

// 允许细化实现 trait
#[allow(refining_impl_trait)]
// 为代理实现 Chat trait
impl<M> Chat for Agent<M>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    // 添加追踪检测，跳过 self、prompt 和 chat_history 参数，记录代理名称
    #[tracing::instrument(skip(self, prompt, chat_history), fields(agent_name = self.name()))]
    // 异步聊天方法的实现
    async fn chat(
        // 接收自身引用
        &self,
        // 提示消息，必须实现 Into<Message> 和 Send trait
        prompt: impl Into<Message> + Send,
        // 可变聊天历史
        mut chat_history: Vec<Message>,
    ) -> Result<String, PromptError> {
        // 创建新的提示请求并设置聊天历史
        PromptRequest::new(self, prompt)
            // 设置聊天历史
            .with_history(&mut chat_history)
            // 等待异步操作完成
            .await
    }
}

// 为代理实现 StreamingCompletion trait
impl<M> StreamingCompletion<M> for Agent<M>
where
    // M 必须实现 CompletionModel trait
    M: CompletionModel,
{
    // 异步流式完成方法的实现
    async fn stream_completion(
        // 接收自身引用
        &self,
        // 提示消息，必须实现 Into<Message> 和 Send trait
        prompt: impl Into<Message> + Send,
        // 聊天历史
        chat_history: Vec<Message>,
    ) -> Result<CompletionRequestBuilder<M>, CompletionError> {
        // Reuse the existing completion implementation to build the request
        // This ensures streaming and non-streaming use the same request building logic
        // 重用现有的完成实现来构建请求
        // 这确保了流式和非流式使用相同的请求构建逻辑
        // 调用现有的完成方法
        self.completion(prompt, chat_history).await
    }
}

// 为代理实现 StreamingPrompt trait
impl<M> StreamingPrompt<M, M::StreamingResponse> for Agent<M>
where
    // M 必须实现 CompletionModel trait 并且具有静态生命周期
    M: CompletionModel + 'static,
    // M 的流式响应必须实现 GetTokenUsage trait
    M::StreamingResponse: GetTokenUsage,
{
    // 流式提示方法的实现
    fn stream_prompt(&self, prompt: impl Into<Message> + Send) -> StreamingPromptRequest<M, ()> {
        // 将代理克隆并包装在 Arc 中
        let arc = Arc::new(self.clone());
        // 创建新的流式提示请求
        StreamingPromptRequest::new(arc, prompt)
    }
}

// 为代理实现 StreamingChat trait
impl<M> StreamingChat<M, M::StreamingResponse> for Agent<M>
where
    // M 必须实现 CompletionModel trait 并且具有静态生命周期
    M: CompletionModel + 'static,
    // M 的流式响应必须实现 GetTokenUsage trait
    M::StreamingResponse: GetTokenUsage,
{
    // 流式聊天方法的实现
    fn stream_chat(
        // 接收自身引用
        &self,
        // 提示消息，必须实现 Into<Message> 和 Send trait
        prompt: impl Into<Message> + Send,
        // 聊天历史
        chat_history: Vec<Message>,
    ) -> StreamingPromptRequest<M, ()> {
        // 将代理克隆并包装在 Arc 中
        let arc = Arc::new(self.clone());
        // 创建新的流式提示请求并设置聊天历史
        StreamingPromptRequest::new(arc, prompt).with_history(chat_history)
    }
}
