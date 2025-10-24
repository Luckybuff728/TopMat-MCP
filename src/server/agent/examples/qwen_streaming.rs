//! 通义千问（Qwen）流式输出示例
//!
//! 本示例展示如何使用 Rig 框架实现通义千问的流式响应
//!
//! 运行示例：
//! ```bash
//! DASHSCOPE_API_KEY=your_api_key cargo run --example qwen_streaming
//! ```

use rig::{
    agent::stream_to_stdout,
    client::{CompletionClient, ProviderClient},
    providers::qwen,
    streaming::StreamingPrompt,
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 从环境变量创建通义千问客户端
    let client = qwen::Client::from_env();

    println!("=== 通义千问流式输出示例 ===\n");
    println!("问题：讲一个关于人工智能的故事\n");
    println!("模型回复（流式）：\n");

    // 创建代理
    let agent = client
        .agent(qwen::QWEN_PLUS)
        .preamble("你是一个擅长讲故事的AI助手。")
        .temperature(0.8)
        .build();

    // 获取流式响应
    let mut stream = agent
        .stream_prompt("讲一个关于人工智能的故事，大约200字")
        .await;

    // 将流式输出打印到标准输出
    let res = stream_to_stdout(&mut stream).await?;

    println!("\n\n=== 流式输出完成 ===");
    println!("Token 使用情况: {:?}", res.usage());

    Ok(())
}
