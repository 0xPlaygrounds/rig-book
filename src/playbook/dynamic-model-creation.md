# Dynamic model creation
This section will talk about some of the challenges around creating model provider clients dynamically and how we can make this as convenient as possible.

Due to the Rust type system, dynamic model client creation is made a bit more difficult than in more dynamic programming languages like Python due to having to specify a type for everything. However, that doesn't mean it is made impossible: it simply has more tradeoffs for doing so.

Let's have a look at some of our possible options for achieving such a feat.

## Enum dispatch
The simplest (and easiest!) way to set up dynamic model client creation is typically to have a single enum under which all the clients you want to go will use:

```rust
enum Agents {
    OpenAI(rig::client::Client<OpenAIResponsesExt, reqwest::Client>),
    Anthropic(rig::client::Client<AnthropicExt, reqwest::Client>),
}

impl DynamicClient {
    fn openai() -> Self {
        let client = rig::providers::openai::Client::from_env();
        
        Self::OpenAI(client)
    }

    fn anthropic() -> Self {
        let client = rig::providers::openai::Client::from_env();
    
        Self::OpenAI(client)
    }

    async fn prompt(&self, prompt: &str) -> Result<String, PromptError> {
        match self {
            Self::Anthropic(agent) => agent.prompt(prompt).await,
            Self::OpenAI(agent) => agent.prompt(prompt).await,
        }
    }
}
```
While this is probably the most convenient method of model creation, you may find that needing to match on every enum variant every single time you need to match the client is quite messy and painful - especially if you aim to support every single provider that Rig also does.

To make our dynamic enum much easier to use, we'll create a provider registry that stores a hashmap of strings tied to function pointers that will create an instance of `Agents`. We can then create some functions as below that essentially allow dynamic creation of agents based on the inputted string:
```rust
struct AgentConfig<'a> {
    name: &'a str,
    preamble: &'a str,
}

struct ProviderRegistry(HashMap<&'static str, fn(&AgentConfig) -> Agents>);

impl ProviderRegistry {
    pub fn new() -> Self {
        Self(HashMap::from_iter([
            ("anthropic", anthropic_agent as fn(&AgentConfig) -> Agents),
            ("openai", openai_agent as fn(&AgentConfig) -> Agents),
        ]))
    }

    pub fn agent(&self, provider: &str, agent_config: &AgentConfig) -> Option<Agents> {
        self.0.get(provider).map(|p| p(agent_config))
    }
}

fn anthropic_agent(AgentConfig { name, preamble }: &AgentConfig) -> Agents {
    let agent = anthropic::Client::from_env()
        .agent(CLAUDE_3_7_SONNET)
        .name(name)
        .preamble(preamble)
        .build();

    Agents::Anthropic(agent)
}

fn openai_agent(AgentConfig { name, preamble }: &AgentConfig) -> Agents {
    let agent = openai::Client::from_env()
        .completions_api()
        .agent(GPT_4O)
        .name(name)
        .preamble(preamble)
        .build();

    Agents::OpenAI(agent)
}
```

Once done, we can then use this in the example like below:

```rust
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
```

## The tradeoffs of using dynamic model creation
Of course, when it comes to using dynamic model creation factory patterns like this, there are some non-trivial tradeoffs that need to be made:

- If not using `typemap`, this abstraction creates a pocket of type unsafety which generally makes it more difficult to maintain if you want to extend it
- Lack of concrete typing means you lose any and all model type specific methods
- You need to update the typemap every now and then when new models come out
- Performance hit at runtime (although in *this* particular case, the performance hit should generally be quite minimal)

However, in some cases if you are running a service that (for example) provides multiple options to users for different providers, this may be a preferable alternative to enum dispatch.
