//! 通义千问（Qwen）工具调用示例
//!
//! 本示例展示如何使用 Rig 框架实现通义千问的 Function Calling 功能
//!
//! 运行示例：
//! ```bash
//! DASHSCOPE_API_KEY=your_api_key cargo run --example qwen_tools
//! ```

use rig::{
    agent::AgentBuilder,
    client::ProviderClient,
    completion::ToolDefinition,
    providers::qwen,
    tool::Tool,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

// 定义天气查询工具
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct WeatherArgs {
    /// 城市或县区，比如北京市、杭州市、余杭区等
    location: String,
}

#[derive(Debug)]
struct WeatherTool;

impl Tool for WeatherTool {
    const NAME: &'static str = "get_current_weather";

    type Error = String;
    type Args = WeatherArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "当你想查询指定城市的天气时非常有用".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "城市或县区，比如北京市、杭州市、余杭区等"
                    }
                },
                "required": ["location"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 模拟天气查询
        let weather = match args.location.as_str() {
            loc if loc.contains("北京") => json!({
                "temperature": 15,
                "weather": "晴天",
                "location": "北京市"
            }),
            loc if loc.contains("杭州") => json!({
                "temperature": 22,
                "weather": "多云",
                "location": "杭州市"
            }),
            _ => json!({
                "temperature": 20,
                "weather": "未知",
                "location": args.location
            }),
        };

        Ok(weather.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 从环境变量创建通义千问客户端
    let client = qwen::Client::from_env();

    // 创建带工具的代理
    let agent = client
        .agent(qwen::QWEN_PLUS)
        .preamble("你是一个有用的助手，可以查询天气信息。")
        .tool(WeatherTool)
        .build();

    println!("=== 通义千问工具调用示例 ===\n");

    // 测试工具调用
    let response = agent.prompt("杭州今天天气怎么样？").await?;

    println!("代理回复：\n{}\n", response);

    // 测试多轮对话
    println!("=== 多轮对话 ===\n");
    
    let response2 = agent.prompt("那北京呢？").await?;
    
    println!("代理回复：\n{}\n", response2);

    Ok(())
}
