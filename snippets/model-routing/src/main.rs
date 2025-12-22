use std::collections::HashMap;

use rig::{
    OneOrMany,
    agent::Agent,
    client::{CompletionClient, EmbeddingsClient, ProviderClient},
    completion::Prompt,
    embeddings::EmbeddingModel,
    providers::openai::{self, responses_api::ResponsesCompletionModel},
    vector_store::{VectorSearchRequest, VectorStoreIndex, in_memory_store::InMemoryVectorStore},
};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simple LLM-based router impl\n---\n");
    llm_based_router().await?;
    println!("Typed router (with embeddings) impl\n---\n");

    let openai_client = openai::Client::from_env();
    let coding_agent = openai_client
        .agent("gpt-5")
        .preamble("You are an expert coding assistant specializing in Rust programming.")
        .build();

    let math_agent = openai_client
        .agent("gpt-5")
        .preamble("You are a mathematics expert who excels at solving complex problems.")
        .build();

    let rtr = TypedRouter::new()
        .add_route("rust", coding_agent)
        .add_route("maths", math_agent);

    let semantic_router = create_semantic_router(&openai_client).await?;

    let prompt = "How do I use async with Rust?";

    let route_name = semantic_route_query(prompt, &semantic_router, &openai_client).await?;
    println!("Route name selected: {route_name}");

    let response = rtr
        .fetch_agent(&route_name)
        .unwrap()
        .prompt(prompt)
        .await
        .unwrap();

    println!("Response: {response}");

    Ok(())
}

/// A simple end-to-end example of how you can write an LLM-based router implementation in a single function.
/// In production, you would probably want to abstract parts of this using the type system
/// as this is primarily hard-coded to use whatever agents are in this function
pub async fn llm_based_router() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the OpenAI client
    let openai_client = openai::Client::from_env();

    // Create specialized agents
    let coding_agent = openai_client
        .agent("gpt-5")
        .preamble("You are an expert coding assistant specializing in Rust programming.")
        .build();

    let math_agent = openai_client
        .agent("gpt-5")
        .preamble("You are a mathematics expert who excels at solving complex problems.")
        .build();

    let router = openai_client
        .agent("gpt-5-mini") // we can afford to use a less expensive model here as the computation required is significantly less
        .preamble(
            "Please return a word from the allowed options list,
            depending on which word the user's question is more closely related to. Skip all prose.

            Options: [\'rust', 'maths']
            ",
        )
        .build();

    let prompt = "How do I use async with Rust?";
    println!("Prompt: {prompt}");

    let topic = router.prompt(prompt).await?;
    println!("Topic selected: {topic}");

    let res = if topic.contains("math") {
        math_agent.prompt(prompt).await?
    } else if topic.contains("rust") {
        coding_agent.prompt(prompt).await?
    } else {
        return Err(format!("No route found in text: {topic}").into());
    };

    println!("Response: {res}");

    Ok(())
}

/// A type alias for an Agent that uses the OpenAI Responses API integration.
type OpenAIAgent = Agent<ResponsesCompletionModel>;

/// A typed route to hold any `OpenAIAgent` and a string identifier.
struct TypedRouter {
    routes: HashMap<String, OpenAIAgent>,
}

impl TypedRouter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn add_route(mut self, route_loc: &str, agent: Agent<ResponsesCompletionModel>) -> Self {
        self.routes.insert(route_loc.to_string(), agent);
        self
    }

    pub fn fetch_agent(&self, route: &str) -> Option<&OpenAIAgent> {
        self.routes.get(route)
    }
}

/// A typed route definition. Has a name, description and examples.
/// These are all concatenated together when embedded to add more meaning to the embedding.
#[derive(Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
struct RouteDefinition {
    name: String,
    description: String,
    examples: Vec<String>,
}

/// Creates a semantic router.
async fn create_semantic_router(
    openai_client: &openai::Client,
) -> Result<InMemoryVectorStore<RouteDefinition>, Box<dyn std::error::Error>> {
    let routes = vec![
        RouteDefinition {
            name: "rust".to_string(),
            description:
                "Programming, code, and software development in the Rust programming languaage"
                    .to_string(),
            examples: vec![
                "How do I write an async function in Rust?".to_string(),
                "Debug this code".to_string(),
                "Implement a sorting algorithm in Rust".to_string(),
            ],
        },
        RouteDefinition {
            name: "math".to_string(),
            description: "Mathematics, calculations, and equations".to_string(),
            examples: vec![
                "Solve this equation".to_string(),
                "Calculate the derivative".to_string(),
                "What is 15% of 200?".to_string(),
            ],
        },
    ];

    let mut vector_store = InMemoryVectorStore::default();

    for route in routes {
        let embedding_text = format!(
            "{}: {}. Examples: {}",
            route.name,
            route.description,
            route.examples.join(", ")
        );

        let embedding = openai_client
            .embedding_model("text-embedding-ada-002")
            .embed_text(&embedding_text)
            .await?;

        vector_store.add_documents(vec![(route, OneOrMany::one(embedding))]);
    }

    Ok(vector_store)
}

/// Routes a given query through a semantic router (see `create_semantic_router`).
async fn semantic_route_query(
    query: &str,
    router: &InMemoryVectorStore<RouteDefinition>,
    openai_client: &openai::Client,
) -> Result<String, Box<dyn std::error::Error>> {
    let embedding_model = openai_client.embedding_model("text-embedding-ada-002");

    let index = router.clone().index(embedding_model);

    let req = VectorSearchRequest::builder()
        .query(query)
        .samples(1)
        .build()?;

    // Find most similar route
    let results = index.top_n::<RouteDefinition>(req).await.unwrap();

    let route_name = results
        .first()
        .map(|(_, _, route_def)| route_def.name.as_str())
        .unwrap_or("general");

    Ok(route_name.to_string())
}
