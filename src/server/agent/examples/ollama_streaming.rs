use rig::agent::stream_to_stdout;
use rig::prelude::*;
use rig::providers::ollama;

use rig::streaming::StreamingPrompt;

#[tokio::main]

async fn main() -> Result<(), anyhow::Error> {
    // 创建具有单一上下文提示的流式代理

    let agent = ollama::Client::new()
        .agent("llama3.2")
        .preamble("Be precise and concise.")
        .temperature(0.5)
        .build();

    // 流式传输响应并在数据块到达时打印

    let mut stream = agent
        .stream_prompt("When and where and what type is the next solar eclipse?")
        .await;

    let res = stream_to_stdout(&mut stream).await?;

    println!("Token usage response: {usage:?}", usage = res.usage());
    println!("Final text response: {message:?}", message = res.response());
    Ok(())
}
