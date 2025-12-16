# Retrieval-Augmented Generation (RAG)

## What is RAG?
Retrieval Augmented Generation (or RAG) retrieves relevant documents from a data store based on a given query and includes them in an LLM prompt to assist with grounding a response in factual information. The goal of doing this is to reduce hallucinations and include data that may be missing from a model's training data (or that it cannot retrieve from a web search).

## How is RAG carried out?
Before we talk about RAG, there are two very important concepts to cover: embeddings, and cosine similarity.

Embeddings are numerical representations ("vectors") that carry semantic meaning. They are created through using embedding models, whether local or from a model provider like OpenAI or Gemini. By doing this, we can mathematically measure semantic similarity between texts - ie, how related they are.

Cosine similarity is the default metric that is used to calculate semantic similarity, with a higher score meaning more semantic similarity. It is quite important within RAG as it is very easy to use to find similarly related documents, and is additionally used in recommender and hybrid/semantic search systems.

RAG starts with document ingestion. Developers will split up documents to be ingested with a chunking strategy (typically using fixed token sizes like 512-1000, or semantic boundaries like paragraphs) for more focused chunks, then embed each chunk and insert the embeddings into a vector store. A vector store can be a database with a vector search plugin (like `pgvector`), or a vector database. Each chunk's metadata is also stored alongside the embedding - the metadata from vector search results will then be included in LLM prompts. 

When the user asks a question, the system will embed the query (again using an embedding model). The query will need to use the exact same model as what was used to embed the documents, as it is impossible to carry out similarity calculations on vectors generated from two different models. Although you can use other metrics for your RAG system, cosine similarity is the default as it is usually the most applicable to a given vector search for RAG.

This then gets passed into the vector search for whatever vector store they're using and should output some resulting relevant documents according to the query, along with their metadata payloads.

## Do I need RAG?
If you are writing a customer support bot or chatbot that relies on some documentation that you (or the company you work for) possess, then RAG is absolutely vital. Being able to ground the LLM in the most up-to-date information rather than hoping it'll somehow magically remember the right information is crucial - if you don't use RAG for this kind of use case, not doing so can carry reputational risk.

However if you are using an LLM for simple data classification tasks for which the categories of data are already well known (ie, "is this a cat or a dog?"), you probably don't need RAG.

## RAG with Rig
Rig provides built-in support for RAG through two ways:
- the `VectorStoreIndex` trait for fetching documents
- the `InsertDocuments` trait for inserting documents

Rig integrations all primarily use cosine similarity as the method of measuring how similar two documents are.

### Supported Vector Stores

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

### Basic example
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

## Modern RAG Patterns
Since its inception, RAG has grown a great deal both in terms of complexity and ways that you can build on the basic pattern.

### Re-ranking
Re-ranking is simple: re-order initial search results for better relevance, accuracy and context understanding. By using re-ranking models, it allows search results to be scored more deeply than vector search which allows for a dramatic improvement in effectiveness.

To get started with re-ranking, the `fastembed` Rust crate has a [`TextRerank`](https://docs.rs/fastembed/latest/fastembed/struct.TextRerank.html) type that can help you re-rank your search results.

### Hybrid search
While semantic search is crucial to RAG, because it is not the same as full-text search it can sometimes miss out on results that might actually contain the target term but may not be considered as relevant. Hybrid search solves this: by combining full-text search and semantic search, you can combine both together.

Hybrid search is a bit more complicated to showcase, but to carry it out: store the documents in a regular database (as well as a vector store), then query both your database and vector store at retrieval time and combine the two lists of results using something like [Reciprocal Rank Fusion](https://www.elastic.co/docs/reference/elasticsearch/rest-apis/reciprocal-rank-fusion) or weighted scoring.

## Use cases for RAG
Retrieval augmented generation is quite a broad topic with an almost infinite amount of use cases. However for our purposes, there are a few cases where it really shines - which we'll discuss below.

### Documentation Q&A
One of the classic use cases for RAG is chunking a PDF document and asking an LLM questions about the document. This allows for a chatbot-like workflow and mitigates needing to read the entire document.

### Memory
RAG can serve as a useful basis of information retrieval for [agentic memory.](./memory.html) With RAG, it allows you to store any kind of document. This means you can store conversation summaries, things that your users (or your company!) may want you to remember, as well as the more typical facts and chunked documents.

### Tool RAG
By storing tool definitions in a vector store, we can also conditionally fetch tool definitions using RAG. This is hugely important as modern agents often need to keep large lists of tools, which can cause hallucinations and degraded model output due to a finite context window. Using tool RAG can also save on token costs, as sending large lists of tools to a model provider can also incur a large token cost over time.

Rig supports this out of the box in the builder type:

```rust
let tools = some_toolset(); // this function is pseudo-code and simply represents the toolset
let in_memory = InMemoryVectorStore::default();

let agent = openai_client.agent("gpt-5")
    .dynamic_tools(2, in_memory, some_toolset)
    .build();
```

The `dynamic_tools` function will store the maximum number of results to return, the toolset as well as the vector store. At context assembly time, the agent will attempt to use RAG to fetch some relevant tool definitions to send to the LLM. The called tools will be executed from the toolset.

## Limitations of RAG
While RAG has become a cornerstone of LLM application development, it also comes with challenges that require careful design to be overcome. Below is a non-exhaustive list of common issues and how to fix them:
- Relevant information can be split across multiple chunks. For this, solutions include overlapping chunks (where a 10-20% overlap captures context at the boundaries), or parent-child chunking where you retrieve small chunks but provide original larger chunks to the LLM
- The information you ingest may have contradictory information - you can fix this with filtering by metadata to retrieve only relevant data, using recency-based weighting and considering the source authority (where official docs should be prioritised over community forums, for example).
- Over time, the data stored in your vector store can become stale and outdated. You can fix this with using `created_at` and `last_updated` fields, using versioning and TTL mechanisms as well as monitoring for changes in the source data and triggering re-embedding.
