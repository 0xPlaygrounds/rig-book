use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    call_agent().await?;
    Ok(())
}

async fn call_agent() -> Result<(), Box<dyn std::error::Error>> {
    let openai_client = Client::from_env();

    let agent = openai_client
        .agent("gpt-5")
        .preamble("You are a helpful assistant.")
        .name("Bob") // used in logging
        .build();

    let prompt = "What is the Rust programming language?";
    println!("{prompt}");

    let response_text = agent.prompt(prompt).await?;

    println!("Response: {response_text}");

    Ok(())
}
