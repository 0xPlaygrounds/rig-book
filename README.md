## The Rig Playbook
This repo is the GitHub repo for the Rig playbook.

📖 **Live at: [book.rig.rs](https://book.rig.rs)**

## Development
Below is some guidance on how to get started with running the various parts of this repo if you would like to contribute.

### Code snippets
If you are interested in contributing to or running any of the examples, you will need the following installed:
- [the Rust programming language](https://rust-lang.org/tools/install/)

You will also need an OpenAI API key (set as `OPENAI_API_KEY` env variable).

### The book
If you are interested in contributing to the book itself, you will need the following installed:
- [the Rust programming language](https://rust-lang.org/tools/install/)
- `mdbook` (`cargo install mdbook`)
- `mdbook-mermaid` (`cargo install mdbook-mermaid`)

You then need to run the following to get the book to generate and hot-reload the HTML for development:

```
mdbook serve
```
