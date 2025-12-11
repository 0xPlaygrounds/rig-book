# What are AI Agents, and why build them in Rust?
Functionally, AI Agents are loops that call a model provider (whether local or a third-party) by adding a prompt and acts on the response. Models can return a request to call a tool (usually represented as a function), which is a crucial part as to how this all works.

In most conventional AI agent frameworks, this means the following:
- Agents that receive tool call requests must be able to automatically execute the tool, then return the result back to the LLM
- If there are no tool calls and there's only a text response, the agent returns the text response and optionally awaits further user input depending on the context the agent is used in.
- Agents may also be able to carry on until a given condition has been reached, rather than simply terminating at the text response. However, using this effectively may require usage of multi-agent architecture which this book will be covering.

Although the primary language to build these in has historically been Python via Langchain (with many, **many** web developers now using the likes of AI SDK and Mastra to do the work instead), the primary reason for building them in Rust is the same as building any other project in Rust: speed, reliability and developer experience. With LLMs starting to reach a plateau, the work has increasingly shifted to building systems for AI and engineering the context around agents, rather than just trying to outright improve the models.

Despite the primary bulk of work for an AI agent being API call-based (hence the term "GPT wrapper"), there is still much work to be done around optimising more computationally heavy aspects of agents. A non-exhaustive list of these use cases can be found below:
- LLM-assisted data pipelines (ie using an LLM as part of a data pipeline)
- Speech training for voice agents
- Preparing data for training a model
