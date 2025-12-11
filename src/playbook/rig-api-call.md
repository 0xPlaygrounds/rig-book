# Making API calls with Rig
Let's get started by writing your first API call.

To get started, create a new project:

```bash
cargo init my-first-project
cd my-first-project
```

Next, we'll add some required dependencies:
```bash
cargo add rig-core@0.25.0 tokio -F tokio/macros,rt-multi-thread
```

Now we're ready to write some code!

## Provider clients
You can create a provider client in one of two ways:
- `Client::new()` takes the API key directly
- `Client::from_env()` - attempts to use environment variables for API keys and will panic on none being provided

An example can be found below:

```rust
use rig::providers::openai::Client;
#[tokio::main]
async fn main() {
    /// uses `OPENAI_API_KEY` environment variable
    let openai_client = Client::from_env();
}
```

## Agents
To create an agent, you'll need to create it using the client we just made.

```rust
use rig::providers::openai::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let openai_client = Client::from_env();
    
    let agent = openai_client.agent("gpt-5")
        .preamble("You are a helpful assistant.")
        .name("Bob") // used in logging
        .build();
    
    let prompt = "What is the Rust programming language?";
    println!("{prompt}");
    
    let response_text = agent.prompt(prompt).await?;
    
    println!("Response: {response_text}");
}
```

### Streaming
Streaming responses with agents is quite simple! To do so, instead of using `prompt()`, you use `stream_prompt()` instead:

```rust
use futures::Stream;

let mut stream = agent.stream_prompt(prompt).await;

while let Some(item) = stream.next().await {
    // .. do some work here
}
```

## Calling completion models directly
Calling completion models is also quite easy. To do so, you'll need to import the `CompletionsClient` trait (from the `client` module) and use the method.

```rust
/// the completion model trait is provided through the CompletionsClient trait!
use rig::client::CompletionsClient;

let openai_client = Client::from_env();

let openai_completions_model = openai_client.completion_model("gpt-5");
```

Next, there's two ways you can call a completion request through the `CompletionModel` trait.

The first one is creating a `CompletionRequest` and calling `CompletionModel::completion()`. You can see the code below:

```rust
//NOTE: OneOrMany is an abstraction that ensures there's always at least one element
let message = Message::User {
    content: OneOrMany::one(UserContent::text("What is the Rust programming language?"))
};

let req = CompletionRequest {
    messages: OneOrMany::one(message),
    premamble: Some("You are a helpful assistant.".to_string()),
    ..Default::default()
};

let response = openai_completions_model.completion(req).await?;
```

You can also `CompletionModel::completion_request` with the prompt text we want to use, then using the builder methods:

```rust
let response = openai_completions_model
    .completion_request("What is the Rust programming language?")
    .preamble("You are a helpful assistant")
    .send()
    .await?;
```

## Should I use agents or direct completion models?
If you just want a way to prompt a model and don't care about the specifics of the contents of the completion model itself, use `rig::agent::Agent`. Agents will also automatically handle tool calling for you.

## Which method should I use with the completion model?
It's a matter of preference. You can use `CompletionRequest` for fully manual conversation history management, or you can use the builder for a more elegant style of usage.
