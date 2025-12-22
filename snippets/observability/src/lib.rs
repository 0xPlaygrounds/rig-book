use rig::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::openai,
};
use tracing::{info, instrument};

#[instrument(name = "process_user_query")]
pub async fn process_query(user_input: &str) -> Result<String, Box<dyn std::error::Error>> {
    info!("Processing user query");

    let openai_client = openai::Client::from_env();

    let agent = openai_client
        .agent("gpt-5.2")
        .preamble("You are a helpful assistant.")
        .build();

    // This completion call will emit spans automatically
    let response = agent.prompt(user_input).await?;

    info!("Query processed successfully");
    Ok(response)
}
