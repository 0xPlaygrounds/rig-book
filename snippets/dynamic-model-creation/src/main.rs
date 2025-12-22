use std::collections::HashMap;

use rig::agent::Agent;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{Prompt, PromptError};
use rig::providers::anthropic::completion::CLAUDE_3_7_SONNET;
use rig::providers::openai::GPT_4O;
use rig::providers::{anthropic, openai};

#[tokio::main]
async fn main() {
    let registry = ProviderRegistry::new();

    let prompt = "How much does 4oz of parmesan cheese weigh?";
    println!("Prompt: {prompt}");

    let helpful_cfg = AgentConfig {
        name: "Assistant",
        preamble: "You are a helpful assistant",
    };

    let openai_agent = registry.agent("openai", &helpful_cfg).unwrap();

    let oai_response = openai_agent.prompt(prompt).await.unwrap();
    println!("Helpful response (OpenAI): {oai_response}");

    let unhelpful_cfg = AgentConfig {
        name: "Assistant",
        preamble: "You are an unhelpful assistant",
    };

    let anthropic_agent = registry.agent("anthropic", &unhelpful_cfg).unwrap();

    let anthropic_response = anthropic_agent.prompt(prompt).await.unwrap();
    println!("Unhelpful response (Anthropic): {anthropic_response}");
}

enum Agents {
    Anthropic(Agent<anthropic::completion::CompletionModel>),
    OpenAI(Agent<openai::completion::CompletionModel>),
}

impl Agents {
    async fn prompt(&self, prompt: &str) -> Result<String, PromptError> {
        match self {
            Self::Anthropic(agent) => agent.prompt(prompt).await,
            Self::OpenAI(agent) => agent.prompt(prompt).await,
        }
    }
}

struct AgentConfig<'a> {
    name: &'a str,
    preamble: &'a str,
}

// In production you would likely want to create some sort of `RegistryKey` type instead of
// allowing arbitrary strings, for improved type safety
struct ProviderRegistry(HashMap<&'static str, fn(&AgentConfig) -> Agents>);

/// A function that creates an instance of `Agents` (using the Anthropic variant)
fn anthropic_agent(AgentConfig { name, preamble }: &AgentConfig) -> Agents {
    let agent = anthropic::Client::from_env()
        .agent(CLAUDE_3_7_SONNET)
        .name(name)
        .preamble(preamble)
        .build();

    Agents::Anthropic(agent)
}

/// A function that creates an instance of `Agents` (using the OpenAI variant)
fn openai_agent(AgentConfig { name, preamble }: &AgentConfig) -> Agents {
    let agent = openai::Client::from_env()
        .completions_api()
        .agent(GPT_4O)
        .name(name)
        .preamble(preamble)
        .build();

    Agents::OpenAI(agent)
}

impl ProviderRegistry {
    /// Creates a new instance of ProviderRegistry.
    /// This is instantiated with both the Anthropic and OpenAI variants (and their corresponding function pointers)
    pub fn new() -> Self {
        Self(HashMap::from_iter([
            ("anthropic", anthropic_agent as fn(&AgentConfig) -> Agents),
            ("openai", openai_agent as fn(&AgentConfig) -> Agents),
        ]))
    }

    /// Attempt to retrieve an Agent.
    /// If none exists, it will simply return None
    pub fn agent(&self, provider: &str, agent_config: &AgentConfig) -> Option<Agents> {
        self.0.get(provider).map(|p| p(agent_config))
    }
}
