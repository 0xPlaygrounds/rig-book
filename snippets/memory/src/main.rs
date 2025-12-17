use rig::agent::Text;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mem = ConversationMemory::with_max_messages(2);

    let prompt1 = "What is the Rust programming language?";
    let _ = call_agent_with_chat_history(prompt1, &mut mem.messages).await?;

    let prompt2 = "Do you think Rust is a good language for AI development? I want to build an AI agent with Rust.";
    let _ = call_agent_with_chat_history(prompt2, &mut mem.messages).await?;

    println!("Message history: {:?}", mem.messages);

    let model = Client::from_env().completion_model("gpt-5.2");

    println!("Attempting to compact conversation and create a summary...");
    mem.compact(&model).await?;

    // SAFETY: We can guarantee that the summary exists here, barring any provider errors
    // which will cause fn main to return early
    let summary = mem.summary.unwrap();
    println!("Conversation summary: {summary}");

    let summary_message_plus_prompt =
        format!("Previous conversation summary:\n{summary}\n\nSorry what did we just talk about?",);

    let _ = call_agent_with_chat_history(&summary_message_plus_prompt, &mut mem.messages).await?;

    Ok(())
}

use rig::providers::openai::Client;
use rig::providers::openai::responses_api::{AdditionalParameters, Reasoning, ReasoningEffort};
use rig::{
    OneOrMany,
    completion::{CompletionModel, Message},
    message::{AssistantContent, UserContent},
};

pub struct ConversationMemory {
    messages: Vec<Message>,
    max_messages: usize,
    summary: Option<String>,
}

impl ConversationMemory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            max_messages: 20,
            summary: None,
        }
    }

    pub fn with_max_messages(max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
            summary: None,
        }
    }

    pub fn add_user_message(&mut self, input: &str) {
        let message = Message::User {
            content: OneOrMany::one(UserContent::text(input)),
        };

        self.messages.push(message);
    }

    pub fn add_assistant_message(&mut self, input: &str) {
        let message = Message::Assistant {
            content: OneOrMany::one(AssistantContent::text(input)),
            id: None,
        };

        self.messages.push(message);
    }

    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub async fn compact<T>(&mut self, model: &T) -> Result<(), Box<dyn std::error::Error>>
    where
        T: CompletionModel,
    {
        if self.messages.len() <= self.max_messages {
            return Ok(());
        }

        // Create a prompt asking the LLM to summarize the conversation
        let summary_prompt = format!(
            "Please provide a concise summary of the following conversation, \
             capturing key points, decisions, and context:\n\n{}",
            self.format_messages_for_summary()
        );

        // Request the summary from the LLM
        let response = model.completion_request(&summary_prompt).send().await?;

        let AssistantContent::Text(Text { text }) = response.choice.first() else {
            return Err("Model returned non-text response".into());
        };

        self.summary = Some(text);
        self.messages.clear();

        Ok(())
    }

    fn format_messages_for_summary(&self) -> String {
        self.messages
            .iter()
            .map(|msg| match msg {
                Message::User { content } => {
                    let text_content = content
                        .iter()
                        .filter_map(|x| {
                            if let UserContent::Text(Text { text }) = x {
                                Some(text.to_owned())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<String>>()
                        .join("\n");

                    format!("User: {text_content}")
                }
                Message::Assistant { content, .. } => {
                    let text_content = content
                        .iter()
                        .filter_map(|x| {
                            if let AssistantContent::Text(Text { text }) = x {
                                Some(text.to_owned())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<String>>()
                        .join("\n");

                    format!("Assistant: {text_content}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

async fn call_agent_with_chat_history(
    prompt: &str,
    history: &mut Vec<Message>,
) -> Result<String, Box<dyn std::error::Error>> {
    let openai_client = Client::from_env();

    // Reasoning with OpenAI requires a verified org which may break this example with GPT-5
    // however, using *no* reasoning is enabled with GPT-5.1 and GPT-5.2
    // hence adding the additional parameters here
    let additional_params = AdditionalParameters {
        reasoning: Some(Reasoning {
            effort: Some(ReasoningEffort::None),
            summary: None,
        }),
        ..Default::default()
    }
    .to_json();

    let agent = openai_client
        .agent("gpt-5.2")
        .preamble("You are a helpful assistant. Be concise.")
        .name("Bob") // used in logging
        .additional_params(additional_params)
        .build();

    println!("User: {prompt}");

    let response_text = agent.prompt(prompt).with_history(history).await?;
    println!("Assistant: {response_text}");

    Ok(response_text)
}
