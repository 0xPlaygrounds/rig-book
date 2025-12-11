# Dynamic model creation
This section will talk about some of the challenges around creating model provider clients dynamically and how we can make this as convenient as possible.

Due to the Rust type system, dynamic model client creation is made a bit more difficult than in more dynamic programming languages like Python due to having to specify a type for everything. However, that doesn't mean it is made impossible: it simply has more tradeoffs for doing so.

Let's have a look at some of our possible options for achieving such a feat.

## Enum dispatch
The simplest (and easiest!) way to set up dynamic model client creation is typically to have a single enum under which all the clients you want to go will use:

```rust
enum DynamicClient {
    OpenAI(rig::client::Client<OpenAIExt, reqwest::Client>),
    Anthropic(rig::client::Client<AnthropicExt, reqwest::Client>),
    // .. etc
}

impl DynamicClient {
    fn openai() -> Self {
        let client = rig::providers::openai::Client::from_env();
        
        Self::OpenAI(client)
    }
}
```
While this is probably the most convenient method of model creation, you may find that needing to match on every enum variant every single time you need to match the client is quite messy and painful - especially if you aim to support every single provider that Rig also does.

The general advice on this is that you should aim to support the 3-4 top providers you absolutely know your users will use, and then go from there. While it is great to have support for every single model provider that exists, if the consumer of your software is an end user then it's quite likely they will only be using 1 or 2 of the most popular ones (Gemini, Anthropic, OpenAI).

## Using dynamic dispatch
Of course, dynamic dispatch (at a slight performance hit) can also be used to create clients. 

First, we will start with a `ProviderFactory` trait that will be responsible for creating a given client as well as getting the name and default model:

```rust
use rig::completion::CompletionModel;

/// Factory trait for creating provider clients
pub trait ProviderFactory: Send + Sync {
    fn create_client(&self, model_name: &str, config: &str) -> Result<Box<dyn CompletionModel>, Box<dyn std::error::Error>>;
    
    /// Get the provider name
    fn name(&self) -> &str;
    
    /// Get the default model name for this provider
    fn default_model(&self) -> &str;
}
```
Next, we'll create the registry - a container type that essentially holds a list of provider factories. We'll add methods for registering new factories, checking a provider exists and a wrapper for creating a completion model.

```rust
use std::collections::HashMap;
use std::sync::Arc;

pub struct ProviderRegistry {
    factories: HashMap<String, Arc<dyn ProviderFactory>>,
}

impl ProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register a new provider factory
    pub fn register(&mut self, factory: impl ProviderFactory + 'static) {
        let name = factory.name().to_string();
        self.factories.insert(name, Arc::new(factory));
    }
    
    /// Creates a client from the internal registry. Takes a provider model name in the form `foo:bar`.
    pub fn create_client(&self, provider_model: &str) -> Result<Box<dyn CompletionModel>, Box<dyn std::error::Error>> {
        let (provider_name, model_name) = self.parse_provider_model(provider_model)?;
        
        let factory = self.factories
            .get(provider_name)
            .ok_or_else(|| format!("Provider '{}' not registered", provider_name))?;
        
        let model = model_name.unwrap_or_else(|| factory.default_model());
        factory.create_client(model, config)
    }

    /// Parse provider:model format
    fn parse_provider_model<'a>(&self, input: &'a str) -> Result<(&'a str, Option<&'a str>), Box<dyn std::error::Error>> {
        if let Some((provider, model)) = input.split_once(':') {
            if provider.is_empty() {
                return Err("Provider name cannot be empty".into());
            }
            if model.is_empty() {
                return Err("Model name cannot be empty when using ':' delimiter".into());
            }
            Ok((provider, Some(model)))
        } else {
            Ok((input, None))
        }
    }

    /// Check if a provider is registered
    pub fn has_provider(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }

    /// List all registered provider names
    pub fn list_providers(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```
There are some items of note here:
- We use a colon-delimited format to parse the provider name, as well as the model. This essentially eschews all type safety for convenience, since now you can use a string to get the client.
- While we don't have providers added here by default, once we've defined one or more providers you can add them in as required.

Next, we need to provide an implementation of `ProviderFactory` for one or more clients. Let's take OpenAI as an example.
```rust
pub struct OpenAIFactory;

impl ProviderFactory for OpenAIFactory {
    fn create_client(&self, model_name: &str, api_key: &str) -> Result<Box<dyn CompletionModel>, Box<dyn std::error::Error>> {
        let client = rig::providers::openai::Client::new(api_key);
        let model = client.completion_model(model_name);
        Ok(Box::new(model))
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        "gpt-4"
    }
}

/// Create a registry with common providers pre-registered
pub fn default_registry() -> ProviderRegistry {
    let mut registry = ProviderRegistry::new();
    registry.register(OpenAIFactory);
    registry
}
```

## The tradeoffs of using dynamic model creation
Of course, when it comes to using dynamic model creation factory patterns like this, there are some non-trivial tradeoffs that need to be made:

- If not using `typemap`, this abstraction creates a pocket of type unsafety which generally makes it more difficult to maintain if you want to extend it
- Lack of concrete typing means you lose any and all model type specific methods
- You need to update the typemap every now and then when new models come out
- Performance hit at runtime (although in *this* particular case, the performance hit should generally be quite minimal)

However, in some cases if you are running a service that (for example) provides multiple options to users for different providers, this may be a preferable alternative to enum dispatch.
