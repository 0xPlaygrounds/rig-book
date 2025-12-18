//! This example showcases RAG usage in Rig.
//! Documents are created, embedded using OpenAI's embedding endpoint and inserted into the in-memory
//! vector store implementation.
use rig::{
    agent::Text,
    client::{CompletionClient, EmbeddingsClient, ProviderClient},
    completion::{CompletionModel, Document},
    embeddings::EmbeddingsBuilder,
    providers::openai::{Client, TEXT_EMBEDDING_ADA_002},
    vector_store::{VectorSearchRequest, VectorStoreIndex, in_memory_store::InMemoryVectorStore},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let openai_client = Client::from_env();
    let mut vector_store = InMemoryVectorStore::default();

    // Define documents to index
    let documents = vec![
        "Rig is a Rust library for building LLM-powered applications.",
        "RAG combines retrieval and generation for better accuracy.",
        "Vector stores enable semantic search over documents.",
    ];

    let embed_model = openai_client.embedding_model(TEXT_EMBEDDING_ADA_002);

    // Create embeddings and add to vector store
    let embeddings = EmbeddingsBuilder::new(embed_model.clone())
        .documents(documents)?
        .build()
        .await?;

    vector_store.add_documents(embeddings);

    // Create a vector index from the in-memory vector store
    let vector_idx = vector_store.index(embed_model);
    let query_text = "What is Rig?";
    println!("Prompt: {query_text}");

    let query = VectorSearchRequest::builder()
        .query(query_text)
        .samples(2)
        .threshold(0.8)
        .build()?;

    // Query the vector store
    let results = vector_idx.top_n::<String>(query).await?;

    if !results.is_empty() {
        println!("Found {} results", results.len());
    }

    results.iter().for_each(|(score, doc_id, doc)| {
        println!("Score: {}, ID: {}, Content: {}", score, doc_id, doc);
    });

    let documents: Vec<Document> = results
        .into_iter()
        .map(|(_, id, doc)| {
            Document {
                id,
                text: doc,
                additional_props: std::collections::HashMap::new(), // whatever extra properties you want here
            }
        })
        .collect();

    let completion_model = openai_client.completion_model("gpt-5.2");

    let res = completion_model
        .completion_request(query_text)
        .documents(documents)
        .send()
        .await
        .unwrap();

    for content in res.choice {
        match content {
            rig::message::AssistantContent::Text(Text { text }) => println!("Response: {text}"),
            other => println!("Received non-text response: {other:?}"),
        }
    }

    Ok(())
}
