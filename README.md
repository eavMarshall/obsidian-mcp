# Headless Markdown MCP Gateway (Rust)

A blazingly fast, lightweight Model Context Protocol (MCP) server for Markdown vaults. Rust + Alpine Docker. 

This gateway transforms local directories of Markdown files into a token-optimized Knowledge Graph for AI Agents. It does one thing perfectly: it gives AI agents the ability to read, write, search, and navigate your markdown vaults using the absolute minimum number of tokens, without mangling your data.

## Core Philosophy
- **Pure Markdown:** No hidden AI databases, no unreadable transpiled pseudo-code, no split-brain architecture. The AI reads and writes standard, human-readable markdown directly to your vaults.
- **Headless Design:** Completely decoupled from any specific UI. The AI interacts with the vaults via MCP tools and asks for your permission directly in the CLI chat before making writes.

## Token Optimization & RAG
- **AST-Based Chunking (`pulldown-cmark`):** Markdown is parsed into structural blocks (Headers, Paragraphs). When the AI requests context, it can surgically pull specific headers or blocks rather than downloading a 5,000-word file.
- **Top-K Exact Retrieval (`tantivy` + `ort`):** A blazing-fast internal index uses BM25 and lightweight local embeddings to find relevant files. It sends the exact, unmodified canonical text of the top results to the LLM.
- **Headless Dataview (`serde_yaml`):** An in-memory metadata/tag index. Progressive disclosure forces the AI to fetch lightweight YAML stubs before pulling heavy body text.
- **O(1) Differential Updates:** Blocks are hashed using `Blake3`. When a file updates, the gateway calculates instant diffs, sending only the modified `.patch` lines over the wire to the AI.

## Architecture
- **Stage 1:** `rust:alpine` compiles static binary.
- **Stage 2:** `alpine` runtime. Microscopic image size (~15MB), zero Node/Python bloat.

## Configuration
Mount `config.yaml` to map your vaults:
```yaml
vaults:
  - id: "work"
    path: "/vaults/work"
    read_only: false
  - id: "personal"
    path: "/vaults/personal"
    read_only: true
```

## Build & Run

**For Tech-Savvy Users (Native Compilation):**
If you have the Rust toolchain installed, you can build the blazing-fast executable natively:
```bash
cargo build --release
```
The resulting executable will be placed in `target/release/obsidian-mcp`.

**For Non-Tech-Savvy Users:**
You can completely skip building! Just go to the **GitHub Releases** page and download the pre-compiled native executable for your operating system (Windows `.exe`, macOS, or Linux).

**To Run (Once built or downloaded):**
No Docker is required! Just run the executable directly, or point your MCP client's configuration file straight to the binary:
```json
{
  "command": "/path/to/obsidian-mcp"
}
```

## MCP Tools
- `list_vaults()` -> `[{id, path, read_only}]`
- `search_vault(query)` -> `Top-K relevant Markdown stubs`
- `read_note(vault_id, uri)` -> `{frontmatter: {}, content: "..."}` (Supports #headers)
- `write_note(vault_id, title, content, append: bool)` -> `success`
- `get_links(vault_id, title)` -> `[forward_links, backlinks]`
- `reload_config()` -> `success`

## 🤖 AI Vault Migration
If you are an AI agent tasked with migrating or converting a user's existing unstructured Obsidian vault to be compatible with this gateway's strict referential integrity rules, please read the [AI_MIGRATION_GUIDE.md](AI_MIGRATION_GUIDE.md) for step-by-step instructions.
