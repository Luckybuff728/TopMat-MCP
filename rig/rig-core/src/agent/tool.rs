use crate::{
    agent::Agent,
    completion::{CompletionModel, Prompt, PromptError, ToolDefinition},
    streaming::{StreamedAssistantContent, StreamingPrompt},
    tool::{TOOL_STREAM_SENDER, Tool},
};
use futures::StreamExt;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentToolArgs {
    /// The prompt for the agent to call.
    prompt: String,
}

impl<M: CompletionModel + 'static> Tool for Agent<M>
where
    <M as CompletionModel>::StreamingResponse: Send + Sync,
{
    const NAME: &'static str = "agent_tool";

    type Error = PromptError;
    type Args = AgentToolArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let description = format!(
            "
            Prompt a sub-agent to do a task for you.

            Agent name: {name}
            Agent description: {description}
            Agent system prompt: {sysprompt}
            ",
            name = self.name(),
            description = self.description.clone().unwrap_or_default(),
            sysprompt = self.preamble.clone().unwrap_or_default()
        );
        ToolDefinition {
            name: <Self as Tool>::name(self),
            description,
            parameters: serde_json::to_value(schema_for!(AgentToolArgs))
                .expect("converting JSON schema to JSON value should never fail"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Check if there's a stream sender available in the task-local context
        let maybe_sender = TOOL_STREAM_SENDER.try_with(|s| s.clone()).ok();

        if let Some(sender) = maybe_sender {
            // Use streaming: forward chunks to the sender
            let mut stream = self.stream_prompt(&args.prompt).multi_turn(20).await;
            let mut collected = String::new();

            while let Some(item) = stream.next().await {
                match item {
                    Ok(crate::agent::MultiTurnStreamItem::StreamItem(
                        StreamedAssistantContent::Text(text),
                    )) => {
                        collected.push_str(&text.text);
                        let _ = sender.send(text.text);
                    }
                    Ok(crate::agent::MultiTurnStreamItem::FinalResponse(res)) => {
                        collected = res.response().to_string();
                        break;
                    }
                    Err(e) => {
                        return Err(PromptError::CompletionError(
                            crate::completion::CompletionError::ResponseError(e.to_string()),
                        ));
                    }
                    _ => {}
                }
            }
            Ok(collected)
        } else {
            // Fallback to non-streaming prompt
            self.prompt(args.prompt).await
        }
    }

    fn name(&self) -> String {
        self.name.clone().unwrap_or_else(|| Self::NAME.to_string())
    }
}
