# Tool calling

The full example for this section can be found [on the GitHub repo.](https://github.com/0xPlaygrounds/rig-book/blob/main/snippets/tool-calling/src/main.rs)

## What is tool calling?
Tool calling (also called "function calling") essentially involves the LLM sending a prompt response containing a content part called a "tool call", using a tool that you have defined as part of your request. Your agent then executes the tool, typically represented as a function, then sends the tool result back to the LLM. The LLM will then use the tool result to generate a response that uses the tool result.

Tools are core to the agentic loop and what differentiates agents from "just an LLM call". By using tool calls, you can turn an LLM into a fully-fledged system that can autonomously execute tasks.

## Do I need tools?
If your LLM application needs to interact with your infrastructure (for example, your Postgres database) or make calculations independently of the other application logic, then you absolutely need tools!

If you're building a simple chat application or using an LLM for data classification tasks though, probably not. These kinds of tasks rely typically more on the raw conversational ability of an LLM, rather than interacting with an agent's external environment.

## Tool calling in Rig
Functionally, tools in Rig can be defined as types that implement the `rig::tool::Tool` trait.

A simple example would be be a tool that adds two numbers together:
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

## Practical usage
Agents in Rig allow you to simply add tools in the builder type:

```rust
let agent = openai_client.agent("gpt-5")
  .tool(Adder)
  .build();
```
