use rig::{
    client::{EmbeddingsClient, ProviderClient},
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

    let model = openai_client.embedding_model(TEXT_EMBEDDING_ADA_002);

    // Create embeddings and add to vector store
    let embeddings = EmbeddingsBuilder::new(model.clone())
        .documents(documents)?
        .build()
        .await?;

    vector_store.add_documents(embeddings);

    // Create a vector index from the in-memory vector store
    let vector_idx = vector_store.index(model);

    let query = VectorSearchRequest::builder()
        .query("What is Rig?")
        .samples(2)
        .build()?;

    // Query the vector store
    let results = vector_idx.top_n::<String>(query).await?;

    for (score, doc_id, doc) in results {
        println!("Score: {}, ID: {}, Content: {}", score, doc_id, doc);
    }

    Ok(())
}
