use std::sync::Arc;
use rig::agent::Agent;
use rig::completion::{CompletionModel, Prompt};
use rig::streaming::StreamingPrompt;

/// MCP Agent 包装器
/// 
/// 这个包装器持有一个 Agent 和 MCP 客户端的引用，
/// 确保 MCP 连接在整个 Agent 生命周期内保持活跃。
/// 这对于流式处理特别重要，因为流式响应可能在函数返回后继续执行。
/// 
/// 类型参数 `C` 是 MCP 客户端的类型，必须实现 Send + Sync
pub struct McpAgent<M, C = Arc<dyn std::any::Any + Send + Sync>> 
where
    M: CompletionModel + Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    agent: Agent<M>,
    _mcp_client: C,
}

impl<M, C> McpAgent<M, C>
where
    M: CompletionModel + Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    /// 创建新的 McpAgent 实例
    /// 
    /// # 参数
    /// * `agent` - Rig Agent 实例
    /// * `mcp_client` - MCP 服务客户端（任何实现 Send + Sync 的类型）
    pub fn new(agent: Agent<M>, mcp_client: C) -> Self {
        tracing::debug!("创建 McpAgent 包装器");
        Self {
            agent,
            _mcp_client: mcp_client,
        }
    }

    /// 发送单轮提示并等待完整响应
    /// 
    /// # 参数
    /// * `prompt` - 用户提示文本
    /// 
    /// # 返回
    /// 返回模型的响应字符串，如果出错则返回错误
    pub async fn prompt(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("McpAgent: 执行 prompt");
        self.agent.prompt(prompt).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// 发送多轮提示并等待完整响应
    /// 
    /// # 参数
    /// * `prompt` - 用户提示文本
    /// * `max_turns` - 最大轮次
    /// 
    /// # 返回
    /// 返回 String，包含最终响应
    pub async fn prompt_multi_turn(
        &self,
        prompt: &str,
        max_turns: usize,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("McpAgent: 执行 prompt_multi_turn，最大轮次={}", max_turns);
        self.agent.prompt(prompt).multi_turn(max_turns).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// 获取内部 Agent 的引用
    /// 
    /// 对于需要流式处理或其他高级功能，直接使用内部 Agent
    /// 例如：`mcp_agent.inner().stream_prompt(prompt).multi_turn(5).await`
    pub fn inner(&self) -> &Agent<M> {
        &self.agent
    }

    /// 获取 MCP 客户端的引用
    pub fn mcp_client(&self) -> &C {
        &self._mcp_client
    }
}

// 实现 Clone，如果需要在多个地方使用同一个 McpAgent
impl<M, C> Clone for McpAgent<M, C>
where
    M: CompletionModel + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            agent: self.agent.clone(),
            _mcp_client: self._mcp_client.clone(),
        }
    }
}

impl<M, C> std::fmt::Debug for McpAgent<M, C>
where
    M: CompletionModel + Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpAgent")
            .field("has_mcp_client", &true)
            .finish()
    }
}

