// 导入 serde 的序列化和反序列化 trait
use serde::{Deserialize, Serialize};
// 导入 serde_json 的 json! 宏
use serde_json::json;

// 导入工具定义结构体
use crate::completion::ToolDefinition;
// 导入工具 trait
use crate::tool::Tool;

/// Think 工具的参数
// Think 工具的参数结构体
// 派生 Deserialize trait 用于反序列化
#[derive(Deserialize)]
pub struct ThinkArgs {
    /// 要思考的想法
    // 要思考的想法内容
    pub thought: String,
}

/// Think 工具的错误类型
// Think 工具的错误类型结构体
// 派生 Debug 和 thiserror::Error trait
#[derive(Debug, thiserror::Error)]
// 错误格式化宏，显示 "Think tool error: {错误消息}"
#[error("Think tool error: {0}")]
// ThinkError 结构体，包含错误消息字符串
pub struct ThinkError(String);

/// Think 工具允许代理在复杂的工具使用情况下停下来思考。
///
/// 此工具为复杂任务期间的结构化思考提供了专用空间，
/// 特别是在处理外部信息（例如，工具调用结果）时。
/// 它实际上不执行任何操作或检索任何信息 - 它只是
/// 为模型提供一个推理复杂问题的空间。
///
/// 此工具最初来源于 Anthropic 的
/// [Think 工具](https://anthropic.com/engineering/claude-think-tool) 博客文章。
// Think 工具允许代理在复杂的工具使用情况下停下来思考
//
// 此工具为复杂任务期间的结构化思考提供了专用空间
// 特别是在处理外部信息（例如，工具调用结果）时
// 它实际上不执行任何操作或检索任何信息 - 它只是
// 为模型提供一个推理复杂问题的空间
//
// 此工具最初来源于 Anthropic 的
// [Think 工具](https://anthropic.com/engineering/claude-think-tool) 博客文章
// 派生 Deserialize 和 Serialize trait
#[derive(Deserialize, Serialize)]
// ThinkTool 结构体，零大小类型
pub struct ThinkTool;

// 为 ThinkTool 实现 Tool trait
impl Tool for ThinkTool {
    // 工具名称常量
    const NAME: &'static str = "think";

    // 错误类型
    type Error = ThinkError;
    // 参数类型
    type Args = ThinkArgs;
    // 输出类型
    type Output = String;

    // 获取工具定义
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        // 创建工具定义
        ToolDefinition {
            // 工具名称
            name: "think".to_string(),
            // 工具描述
            description: "Use the tool to think about something. It will not obtain new information
            or change the database, but just append the thought to the log. Use it when complex
            reasoning or some cache memory is needed."
                .to_string(),
            // 工具参数定义（JSON Schema 格式）
            parameters: json!({
                "type": "object",
                "properties": {
                    "thought": {
                        "type": "string",
                        "description": "A thought to think about."
                    }
                },
                "required": ["thought"]
            }),
        }
    }

    // 执行工具调用
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // The think tool doesn't actually do anything except echo back the thought
        // This is intentional - it's just a space for the model to reason through problems
        // Think 工具实际上不执行任何操作，只是回显思考内容
        // 这是有意的 - 它只是为模型提供一个推理问题的空间
        // 返回思考内容
        Ok(args.thought)
    }
}

// 测试模块，只在测试时编译
#[cfg(test)]
mod tests {
    // 导入父模块的所有内容
    use super::*;

    // 测试 Think 工具的定义
    #[tokio::test]
    async fn test_think_tool_definition() {
        // 创建 ThinkTool 实例
        let tool = ThinkTool;
        // 获取工具定义
        let definition = tool.definition("".to_string()).await;

        // 验证工具名称
        assert_eq!(definition.name, "think");
        // 验证工具描述包含预期内容
        assert!(
            definition
                .description
                .contains("Use the tool to think about something")
        );
    }

    // 测试 Think 工具的调用
    #[tokio::test]
    async fn test_think_tool_call() {
        // 创建 ThinkTool 实例
        let tool = ThinkTool;
        // 创建测试参数
        let args = ThinkArgs {
            thought: "I need to verify the user's identity before proceeding".to_string(),
        };

        // 调用工具并获取结果
        let result = tool.call(args).await.unwrap();
        // 验证返回结果与输入一致
        assert_eq!(
            result,
            "I need to verify the user's identity before proceeding"
        );
    }
}
