// 导入代理和完成模型相关类型
use crate::{
    agent::Agent,
    completion::{CompletionModel, Prompt, PromptError, ToolDefinition},
    tool::Tool,
};
// 导入 JSON Schema 相关类型
use schemars::{JsonSchema, schema_for};
// 导入序列化和反序列化 trait
use serde::{Deserialize, Serialize};

// 派生调试、克隆、序列化、反序列化和 JSON Schema trait
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
// 定义代理工具参数结构体
pub struct AgentToolArgs {
    /// 代理要调用的提示。
    // 要传递给代理的提示文本
    prompt: String,
}

// 为代理实现工具 trait，支持泛型完成模型
impl<M: CompletionModel> Tool for Agent<M> {
    // 定义工具名称为常量
    const NAME: &'static str = "agent_tool";

    // 定义错误类型为提示错误
    type Error = PromptError;
    // 定义参数类型为代理工具参数
    type Args = AgentToolArgs;
    // 定义输出类型为字符串
    type Output = String;

    // 异步函数：获取工具定义
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        // 返回工具定义结构体
        ToolDefinition {
            // 设置工具名称
            name: <Self as Tool>::name(self),
            // 格式化工具描述，包含代理的前言
            description: format!(
                "允许代理通过提示调用另一个代理的工具。该代理的前言如下：
                ---
                {}",
                // 获取代理的前言，如果为空则使用默认值
                self.preamble.clone().unwrap_or_default()
            ),
            // 将代理工具参数的 JSON Schema 转换为值
            parameters: serde_json::to_value(schema_for!(AgentToolArgs))
                // 如果转换失败则 panic（这种情况不应该发生）
                .expect("converting JSON schema to JSON value should never fail"),
        }
    }

    // 异步函数：调用工具
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 使用代理的提示方法处理参数中的提示
        self.prompt(args.prompt).await
    }

    // 获取工具名称
    fn name(&self) -> String {
        // 返回代理的名称，如果为空则使用默认工具名称
        self.name.clone().unwrap_or_else(|| Self::NAME.to_string())
    }
}
