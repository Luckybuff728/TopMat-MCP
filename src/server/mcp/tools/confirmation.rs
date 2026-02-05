//! 人机协同 (Human-in-the-Loop) 确认工具
//!
//! 该工具用于强制 Agent 在执行高开销操作前暂停并等待用户确认。

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// HITL 中断信号常量
pub const HITL_SIGNAL_WAIT_FOR_USER: &str = "HITL_SIGNAL_WAIT_FOR_USER";

/// 请求确认工具
///
/// 当 Agent 调用此工具时，后端会中断当前的 multi_turn 循环，
/// 将控制权交还给用户。用户确认后，Agent 可以继续执行。
#[derive(Deserialize, Serialize, Default)]
pub struct RequestConfirmation;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmationToolError(String);
impl std::fmt::Display for ConfirmationToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ConfirmationToolError {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmationArgs {
    /// 待用户确认的计划摘要
    pub plan_summary: String,
}

impl Tool for RequestConfirmation {
    const NAME: &'static str = "request_confirmation";
    type Error = ConfirmationToolError;
    type Args = ConfirmationArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "向用户请求确认。在执行高开销操作（如调用子智能体、运行仿真、启动优化）之前，必须调用此工具。系统将暂停并等待用户回复确认。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "plan_summary": {
                        "type": "string",
                        "description": "待用户确认的计划摘要，简述即将执行的操作及其目的。"
                    }
                },
                "required": ["plan_summary"]
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 返回特殊的哨兵信号，后端会检测到此信号并中断循环
        Ok(HITL_SIGNAL_WAIT_FOR_USER.to_string())
    }
}
