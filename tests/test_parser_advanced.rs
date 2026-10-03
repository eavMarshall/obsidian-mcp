use obsidian_mcp::parser::MarkdownParser;

#[test]
fn test_complex_markdown_parsing() {
    let raw = r#"
# Intro
Welcome to the vault.
## Code Section
Here is some code:
```rust
fn main() {}
```
## Links
We link to [[AnotherNote|Alias]] and [[RawNote]].
"#;

    let chunks = MarkdownParser::chunk_by_headers(raw);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].header, "Intro");
    
    // Check that code blocks are preserved with backticks so the LLM doesn't lose syntax
    assert!(chunks[1].content.contains("`rust\nfn main() {}\n`"));
    
    // Check the exact extracted links
    let links = MarkdownParser::extract_links(raw);
    assert_eq!(links.len(), 2);
    // Alias must be stripped, leaving only the canonical note name
    assert_eq!(links[0], "AnotherNote");
    assert_eq!(links[1], "RawNote");
}
