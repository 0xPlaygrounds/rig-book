# Tool calling in Rig
Functionally, tools in Rig can be defined as types that implement the `rig::tool::Tool` trait.

A simple example of this might be a tool that adds two numbers together:
```rust
use serde::{Deserialize, Serialize};
#[derive(Deserialize)]
struct OperationArgs {
    x: i32,
    y: i32,
}

#[derive(Debug, thiserror::Error)]
#[error("Math error")]
struct MathError;

#[derive(Deserialize, Serialize)]
struct Adder;

impl Tool for Adder {
    const NAME: &'static str = "add";
    type Error = MathError;
    type Args = OperationArgs;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "add".to_string(),
            description: "Add x and y together".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "number",
                        "description": "The first number to add"
                    },
                    "y": {
                        "type": "number",
                        "description": "The second number to add"
                    }
                },
                "required": ["x", "y"],
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        println!("[tool-call] Adding {} and {}", args.x, args.y);
        let result = args.x + args.y;
        Ok(result)
    }
}
```

You can also use the `rig::tool_macro` macro:

```rust
#[rig::tool_macro(
    description = "Adds two numbers together".
)]
async fn add(x: i32, y: i32) -> i32 {
    x + y
}
```


## Built-in Tools
By default, you can call Agents and vector stores as tools. This makes it much easier to write multi-agent systems!

If you are trying to use an agent as a tool, don't forget to add a name and description by using their respective functions.

## Practical usage and applications
Without tools, LLMs are pretty much just theoretical word guessers.

### Agents
Agents can automatically execute tools that give it on your behalf. You can do so by simply adding the tool to your Agent:

```rust
let agent = openai_client.agent("gpt-5")
    .preamble("You are a helpful assistant.")
    .tool(Adder)
    .build();
```
