use rig::agent::stream_to_stdout;
use rig::prelude::*;
use rig::providers::xai;
use rig::streaming::StreamingPrompt;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 创建具有单一上下文提示的流式代理
    let agent = xai::Client::from_env()
        .agent(xai::GROK_3_MINI)
        .preamble("Be precise and concise.")
        .temperature(0.5)
        .build();

    // 流式传输响应并在数据块到达时打印
    let mut stream = agent
        .stream_prompt("When and where and what type is the next solar eclipse?")
        .await;

    let _ = stream_to_stdout(&mut stream).await?;

    Ok(())
}
