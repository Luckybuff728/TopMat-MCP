//! 通义千问（Qwen）基础使用示例
//!
//! 本示例展示如何使用 Rig 框架与通义千问 API 进行交互
//!
//! 运行示例：
//! ```bash
//! DASHSCOPE_API_KEY=your_api_key cargo run --example qwen_basic
//! ```

use rig::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::qwen,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 从环境变量创建通义千问客户端
    let client = qwen::Client::from_env();

    println!("=== 通义千问基础对话示例 ===\n");

    // 创建代理
    let agent = client
        .agent(qwen::QWEN_PLUS)
        .preamble("你是一个有用的AI助手。")
        .build();

    // 发送简单提示
    let response = agent
        .prompt("你是谁？请简单介绍一下自己。")
        .await?;

    println!("模型回复：\n{}\n", response);

    println!("=== 创意写作示例 ===\n");

    // 创建创意写作代理
    let creative_agent = client
        .agent(qwen::QWEN_PLUS)
        .preamble("你是一位富有创意的诗人。")
        .temperature(0.9)
        .build();
    
    let poem = creative_agent
        .prompt("写一首关于秋天的短诗")
        .await?;

    println!("生成的诗歌：\n{}\n", poem);

    Ok(())
}
