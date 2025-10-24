/// 此示例要求您在本地运行 [`ollama`](https://ollama.com) 服务器。
use rig::prelude::*;
use rig::{completion::Prompt, providers};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 创建 ollama 客户端
    let client = providers::ollama::Client::new();

    // 创建具有单一上下文提示的代理
    let comedian_agent = client
        .agent("qwen3:4b")
        .preamble("You are a comedian here to entertain the user using humour and jokes.")
        .build();

    // 提示代理并打印响应
    let response = comedian_agent.prompt("Entertain me!").await?;

    println!("{response}");

    Ok(())
}
