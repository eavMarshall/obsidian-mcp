use obsidian_mcp::parser::MarkdownParser;

#[tokio::test]
async fn test_ast_chunking_cases() {
    // Case 1: Multiple H1 headers
    let raw_h1 = "### Header A\nContent A\n### Header B\nContent B";
    let chunks_h1 = MarkdownParser::chunk_by_headers(raw_h1);
    assert_eq!(chunks_h1.len(), 2);
    assert_eq!(chunks_h1[0].header, "Header A");
    assert_eq!(chunks_h1[1].header, "Header B");
    
    // Case 2: Deep nested headers
    let raw_nested = "## Header 1\n## Header 2\n### Header 3\nContent 3";
    let chunks_nested = MarkdownParser::chunk_by_headers(raw_nested);
    // The current chunking logic might just group everything under the nearest header
    // or treat all header levels as chunk boundaries.
    assert_eq!(chunks_nested.last().unwrap().header, "Header 3");
    
    // Case 3: Code block collision (headers inside code blocks shouldn't trigger chunking)
    let raw_code = "## Real Header\n```markdown\n## Fake Header\n```";
    let chunks_code = MarkdownParser::chunk_by_headers(raw_code);
    assert_eq!(chunks_code.len(), 1);
    assert_eq!(chunks_code[0].header, "Real Header");
    assert!(chunks_code[0].content.contains("## Fake Header"));

    // Case 4: List parsing within chunks
    let raw_list = "## List Header\n- Item 1\n- Item 2\n  - Subitem 1";
    let chunks_list = MarkdownParser::chunk_by_headers(raw_list);
    assert_eq!(chunks_list.len(), 1);
    assert!(chunks_list[0].content.contains("- Subitem 1"));
}
