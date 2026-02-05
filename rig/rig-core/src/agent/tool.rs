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
                    Ok(crate::agent::MultiTurnStreamItem::StreamItem(content)) => match content {
                        StreamedAssistantContent::Text(text) => {
                            collected.push_str(&text.text);
                            let _ = sender.send(crate::tool::ToolStreamItem::Text(text.text));
                        }
                        StreamedAssistantContent::ToolCall(tool_call_indicator) => {
                            let _ = sender.send(crate::tool::ToolStreamItem::ToolCall {
                                id: tool_call_indicator.tool_call.id,
                                name: tool_call_indicator.tool_call.function.name,
                                arguments: tool_call_indicator.tool_call.function.arguments,
                                is_agent: tool_call_indicator.is_agent,
                            });
                        }
                        StreamedAssistantContent::ToolResult {
                            id,
                            result,
                            is_agent,
                        } => {
                            let _ = sender.send(crate::tool::ToolStreamItem::ToolResult {
                                id,
                                result,
                                is_agent,
                            });
                        }
                        StreamedAssistantContent::Reasoning(reasoning) => {
                            let _ = sender.send(crate::tool::ToolStreamItem::Reasoning(
                                reasoning.reasoning.join("\n"),
                            ));
                        }
                        _ => {}
                    },
                    Ok(crate::agent::MultiTurnStreamItem::FinalResponse(res)) => {
                        collected = res.response().to_string();
                        break;
                    }
                    Err(e) => {
                        return Err(PromptError::CompletionError(
                            crate::completion::CompletionError::ResponseError(e.to_string()),
                        ));
                    }
                }
            }
            Ok(collected)
        } else {
            // Fallback to non-streaming prompt
            self.prompt(args.prompt).await
        }
    }

    fn is_agent(&self) -> bool {
        true
    }

    fn name(&self) -> String {
        self.name.clone().unwrap_or_else(|| Self::NAME.to_string())
    }
}
