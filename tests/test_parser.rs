use obsidian_mcp::parser::MarkdownParser;

#[tokio::test]
async fn test_ast_chunking_and_noise_stripping() {
    let raw_markdown = r#"
# Core Rules
This is the main rule block.
It has a **bold** word and an *italic* word.

## Database
Do not use `unwrap()`.
"#;

    let chunks = MarkdownParser::chunk_by_headers(raw_markdown);

    assert_eq!(chunks.len(), 2, "Should create exactly two chunks based on headers");

    assert_eq!(chunks[0].header, "Core Rules");
    assert_eq!(chunks[0].content.trim(), "# Core Rules\nThis is the main rule block.\nIt has a **bold** word and an *italic* word.");

    assert_eq!(chunks[1].header, "Database");
    assert_eq!(chunks[1].content.trim(), "## Database\nDo not use `unwrap()`.");
}
