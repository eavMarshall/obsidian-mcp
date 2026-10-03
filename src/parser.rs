use pulldown_cmark::{Event, Parser as CmarkParser, Tag, TagEnd};

pub struct MarkdownParser;

#[derive(Debug, PartialEq)]
pub struct Chunk {
    pub header: String,
    pub content: String,
}

impl MarkdownParser {
    /// Parses a raw markdown string and chunks it into sections based on headers.
    /// This is the core of our "AST-Based Chunking" token-saving feature.
    pub fn chunk_by_headers(markdown: &str) -> Vec<Chunk> {
        let parser = CmarkParser::new(markdown);
        let mut chunks = Vec::new();
        
        let mut current_header = String::from("ROOT");
        let mut current_content = String::new();
        let mut in_header = false;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { .. }) => {
                    // Push the previous chunk if it has content
                    if !current_content.trim().is_empty() {
                        chunks.push(Chunk {
                            header: current_header.clone(),
                            content: current_content.trim().to_string(),
                        });
                    }
                    current_content.clear();
                    current_header.clear();
                    in_header = true;
                }
                Event::End(TagEnd::Heading(_)) => {
                    in_header = false;
                }
                Event::Text(text) => {
                    if in_header {
                        current_header.push_str(&text);
                    } else {
                        current_content.push_str(&text);
                    }
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    current_content.push_str("```");
                    if let pulldown_cmark::CodeBlockKind::Fenced(lang) = kind {
                        current_content.push_str(&lang);
                    }
                    current_content.push('\n');
                }
                Event::End(TagEnd::CodeBlock) => {
                    if !current_content.ends_with('\n') {
                        current_content.push('\n');
                    }
                    current_content.push_str("```\n");
                }
                Event::Code(code) => {
                    current_content.push('`');
                    current_content.push_str(&code);
                    current_content.push('`');
                }
                Event::SoftBreak | Event::HardBreak => {
                    current_content.push('\n');
                }
                _ => {
                    // Ignore other formatting to save tokens (stripping noise)
                }
            }
        }

        // Push the final chunk
        if !current_content.trim().is_empty() {
            chunks.push(Chunk {
                header: current_header.clone(),
                content: current_content.trim().to_string(),
            });
        }

        chunks
    }

    /// Extracts all [[wikilinks]] from a markdown document
    pub fn extract_links(markdown: &str) -> Vec<String> {
        let re = regex::Regex::new(r"\[\[(.*?)\]\]").unwrap();
        let mut links = Vec::new();
        
        for cap in re.captures_iter(markdown) {
            if let Some(matched) = cap.get(1) {
                // Handle aliases like [[Note Title|Alias]]
                let link_core = matched.as_str().split('|').next().unwrap_or("").trim().to_string();
                if !link_core.is_empty() {
                    links.push(link_core);
                }
            }
        }
        
        links
    }
}
