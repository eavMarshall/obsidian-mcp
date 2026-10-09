use pulldown_cmark::{Event, Options, Parser as CmarkParser, Tag, TagEnd};
use std::sync::OnceLock;

pub struct MarkdownParser;

#[derive(Debug, PartialEq)]
pub struct Chunk<'a> {
    pub header: std::borrow::Cow<'a, str>,
    pub content: &'a str,
}

static WIKILINK_REGEX: OnceLock<regex::Regex> = OnceLock::new();

impl MarkdownParser {
    pub fn chunk_by_headers(markdown: &str) -> Vec<Chunk<'_>> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        
        let parser = CmarkParser::new_ext(markdown, options).into_offset_iter();
        let mut chunks = Vec::new();
        
        let mut current_header = String::from("ROOT");
        let mut chunk_start_offset = 0;
        let mut in_header = false;

        for (event, range) in parser {
            match event {
                Event::Start(Tag::Heading { .. }) => {
                    let header_text = &markdown[range.start..range.end];
                    let is_case = header_text.starts_with("## ") || header_text.starts_with("### ") || header_text.starts_with("##\t") || header_text.starts_with("###\t");
                    if is_case {
                        let chunk_end_offset = range.start;
                        if chunk_start_offset < chunk_end_offset {
                            let content = &markdown[chunk_start_offset..chunk_end_offset];
                            if !content.trim().is_empty() {
                                chunks.push(Chunk {
                                    header: std::borrow::Cow::Owned(std::mem::take(&mut current_header)),
                                    content: content.trim(),
                                });
                            }
                        }
                        
                        chunk_start_offset = range.start;
                        current_header.clear();
                        in_header = true;
                    }
                }
                Event::End(TagEnd::Heading(_)) => {
                    if in_header {
                        in_header = false;
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if in_header {
                        current_header.push(' ');
                    }
                }
                Event::Text(text) => {
                    if in_header {
                        current_header.push_str(&text);
                    }
                }
                Event::Code(code) => {
                    if in_header {
                        current_header.push_str(&code);
                    }
                }
                _ => {}
            }
        }

        if chunk_start_offset < markdown.len() {
            if let Some(content) = markdown.get(chunk_start_offset..) {
                if !content.trim().is_empty() {
                    chunks.push(Chunk {
                        header: std::borrow::Cow::Owned(std::mem::take(&mut current_header)),
                        content: content.trim(),
                    });
                }
            }
        }

        chunks
    }

    pub fn extract_links(markdown: &str) -> Vec<String> {
        let re = WIKILINK_REGEX.get_or_init(|| regex::Regex::new(r"\[\[(.*?)\]\]").unwrap());
        
        let mut code_ranges = Vec::new();
        let mut current_block_start = None;
        let parser = CmarkParser::new(markdown).into_offset_iter();
        for (event, range) in parser {
            match event {
                Event::Start(Tag::CodeBlock(_)) => {
                    current_block_start = Some(range.start);
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some(start) = current_block_start.take() {
                        code_ranges.push(start..range.end);
                    }
                }
                Event::Code(_) => {
                    code_ranges.push(range);
                }
                _ => {}
            }
        }
        if let Some(start) = current_block_start.take() {
            code_ranges.push(start..markdown.len());
        }
        
        let mut links = Vec::new();
        
        for cap in re.captures_iter(markdown) {
            let mat = cap.get(0).unwrap(); // Get full match to check offset
            let is_in_code = code_ranges.iter().any(|r| r.contains(&mat.start()));
            
            if !is_in_code {
                if let Some(matched) = cap.get(1) {
                    let link_core = matched.as_str().split('|').next().unwrap_or("").trim().to_string();
                    if !link_core.is_empty() && !link_core.starts_with("obsidian://") {
                        links.push(link_core);
                    }
                }
            }
        }
        
        links
    }

    pub fn rename_links(markdown: &str, old_targets: &[String], new_target: &str) -> String {
        let re = WIKILINK_REGEX.get_or_init(|| regex::Regex::new(r"\[\[(.*?)\]\]").unwrap());
        
        let mut code_ranges = Vec::new();
        let mut current_block_start = None;
        let parser = CmarkParser::new(markdown).into_offset_iter();
        for (event, range) in parser {
            match event {
                Event::Start(Tag::CodeBlock(_)) => { current_block_start = Some(range.start); }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some(start) = current_block_start.take() { code_ranges.push(start..range.end); }
                }
                Event::Code(_) => { code_ranges.push(range); }
                _ => {}
            }
        }
        if let Some(start) = current_block_start.take() { code_ranges.push(start..markdown.len()); }
        
        let mut result = String::with_capacity(markdown.len());
        let mut last_match_end = 0;

        for cap in re.captures_iter(markdown) {
            let mat = cap.get(0).unwrap();
            let inner_mat = cap.get(1).unwrap();
            let is_in_code = code_ranges.iter().any(|r| r.contains(&mat.start()));
            
            result.push_str(&markdown[last_match_end..inner_mat.start()]);

            if !is_in_code {
                let inner_text = inner_mat.as_str();
                let split_pipe: Vec<&str> = inner_text.splitn(2, '|').collect();
                let split_hash: Vec<&str> = split_pipe[0].splitn(2, '#').collect();
                let link_base = split_hash[0].trim();
                let link_lower = link_base.to_lowercase();
                
                if old_targets.iter().any(|t| t == &link_lower) {
                    // Match found, replace link_base with new_target
                    // Preserve leading/trailing whitespace around link_base? Typically we just emit the new target.
                    result.push_str(new_target);
                    if split_hash.len() > 1 {
                        result.push('#');
                        result.push_str(split_hash[1]);
                    }
                    if split_pipe.len() > 1 {
                        result.push('|');
                        result.push_str(split_pipe[1]);
                    }
                } else {
                    result.push_str(inner_text);
                }
            } else {
                result.push_str(inner_mat.as_str());
            }
            
            last_match_end = inner_mat.end();
        }
        
        result.push_str(&markdown[last_match_end..]);
        result
    }
}
