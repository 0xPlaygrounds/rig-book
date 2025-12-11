# Retrieval-Augmented Generation (RAG)

## What is RAG?
Retrieval Augmented Generation (or RAG), broadly speaking, is a technique that describes using document retrieval to augment a prompt.

This essentially means that instead of a prompt where you have plain use text:

```
Prompt: 
What is Einstein famous for?
```

You can now inject context into your prompts:

```
Prompt: 
What is Einstein famous for?

Context:
<insert some RAG context here>
```

The goal is that the model will then generate more accurate answers because it has the context to help support writing a fact-based answer.

## RAG with Rig
Rig provides built-in support for RAG through two ways:
- the `VectorStoreIndex` trait for fetching documents
- the `InsertDocuments` trait for inserting documents

Rig integrations all primarily use cosine similarity as the method of measuring how similar two documents are.

Here's a basic example of setting up RAG with Rig:

```rust
use rig::{
    embeddings::EmbeddingsBuilder,
    providers::openai::{Client, TEXT_EMBEDDING_ADA_002},
    vector_store::{in_memory_store::InMemoryVectorStore, VectorStore, InsertDocuments},
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

    // Create embeddings and add to vector store
    let embeddings = EmbeddingsBuilder::new(openai_client.embedding_model(TEXT_EMBEDDING_ADA_002))
        .documents(documents)
        .build()
        .await?;

    vector_store.add_documents(embeddings).await?;

    // Query the vector store
    let results = vector_store
        .top_n::<String>("What is Rig?", 2)
        .await?;

    for (score, doc_id, doc) in results {
        println!("Score: {}, ID: {}, Content: {}", score, doc_id, doc);
    }

    Ok(())
}
```

Once you've fetched your document, you can then use it in an LLM completion call:

```rust
let documents: Vec<Document> = results.into_iter().map(|(score, id, doc)| {
    Document {
        id,
        text: doc,
        additional_props: std::collections::HashMap::new() // whatever extra properties you want here
    }
}).collect();

let req =  CompletionRequest {
    documents,
    // fill in the rest of the fields here!
}
```

## Using RAG with Agents

Rig makes it straightforward to equip agents with RAG capabilities. You can attach a vector store to an agent, and it will automatically retrieve relevant context before generating responses:

```rust
use rig::{
    agent::Agent,
    completion::Prompt,
    providers::openai::{Client, GPT_4},
    vector_store::in_memory_store::InMemoryVectorStore,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let openai_client = Client::from_env();

    // Set up vector store with your documents (as shown above)
    let vector_store = InMemoryVectorStore::default();

    // Create an agent with RAG capabilities
    let agent = openai_client
        .agent(GPT_4)
        .preamble("You are a helpful assistant that answers questions using the provided context.")
        .dynamic_context(2, vector_store) // Retrieve top 2 relevant documents
        .build();

    // Use the agent
    let response = agent
        .prompt("What is Rig and how does it help with LLM applications?")
        .await?;

    println!("Agent response: {}", response);

    Ok(())
}
```

The `dynamic_context` method configures the agent to automatically retrieve the specified number of relevant documents for each query and include them in the context sent to the language model.

## Use cases for RAG
Retrieval augmented generation is quite a broad topic with an almost infinite amount of use cases. However for our purposes, there are a few cases where it really shines - which we'll discuss below.

### Memory
RAG can serve as a useful basis of information retrieval for agentic memory (which we'll cover later on in this playbook!). With RAG, it allows you to store any kind of document. This means you can store conversation summaries, things that your users (or your company!) may want you to remember, as well as the more typical facts and chunked documents.

### Tool RAG
By storing tool definitions in a vector store, we can also conditionally fetch tool definitions using RAG. This greatly assists with scoping down a given request result such that a model won't be overloaded with tools and it will also reduce the number of tokens used.

Rig supports this out of the box:

```rust
let tools = some_toolset(); // this function is pseudo-code and simply represents the toolset
let in_memoery = InMemoryVectorStore::default();
let agent = openai_client.agent("gpt-5")
    .dynamic_tools(2, in_memory, some_toolset)
    .build();
```

## Supported Vector Stores

By default, Rig has an in-memory vector store that is ideal for development and small-scale applications without any external dependencies.

However, if you'd like to use a durable vector store (or simply want to take Rig to production!), here are some of the vector stores we support:
- HelixDB
- LanceDB
- Milvus (both locally and through Zilliz Cloud)
- MongoDB
- Neo4j
- PostgresQL
- Qdrant
- ScyllaDB
- Sqlite
- SurrealDB
