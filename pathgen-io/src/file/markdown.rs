use std::{fs, io, path::Path};

use crate::sectioned::SectionedDocument;

/// Sectioned-document adapter for Markdown text.
///
/// Markdown is modeled as raw text with headings and optional raw frontmatter.
/// It is deliberately not modeled as a structured key/value tree.
#[derive(Debug, Clone, Copy, Default)]
pub struct MarkdownDocument;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownFrontmatter<'a> {
    pub body: &'a str,
    pub frontmatter: &'a str,
    pub rest: &'a str,
}

pub fn to_markdown_file<P>(path: P, value: impl AsRef<str>) -> io::Result<()>
where
    P: AsRef<Path>,
{
    fs::write(path, value.as_ref())
}

pub fn from_markdown_file<P>(path: P) -> io::Result<String>
where
    P: AsRef<Path>,
{
    fs::read_to_string(path)
}

pub fn split_frontmatter(document: &str) -> Option<MarkdownFrontmatter<'_>> {
    let after_open = document.strip_prefix("---\n")?;
    let close_index = after_open.find("\n---\n")?;
    let frontmatter = &after_open[..close_index];
    let rest = &after_open[close_index + "\n---\n".len()..];
    Some(MarkdownFrontmatter {
        body: document,
        frontmatter,
        rest,
    })
}

pub fn replace_frontmatter(document: &str, new_frontmatter: &str) -> String {
    if let Some(parts) = split_frontmatter(document) {
        format!("---\n{}\n---\n{}", new_frontmatter.trim_matches('\n'), parts.rest)
    } else {
        format!("---\n{}\n---\n{}", new_frontmatter.trim_matches('\n'), document)
    }
}

pub fn get_section<'a>(document: &'a str, heading_path: &[&str]) -> Option<&'a str> {
    MarkdownDocument::get_section(document, heading_path)
}

pub fn replace_section(document: &str, heading_path: &[&str], new_body: &str) -> io::Result<String> {
    MarkdownDocument::replace_section(document, heading_path, new_body)
}

pub fn append_section(document: &str, level: u8, heading: &str, body: &str) -> io::Result<String> {
    MarkdownDocument::append_section(document, level, heading, body)
}

impl SectionedDocument for MarkdownDocument {
    fn get_section<'a>(document: &'a str, heading_path: &[&str]) -> Option<&'a str> {
        let span = find_section_span(document, heading_path)?;
        Some(document[span.body_start..span.body_end].trim_matches('\n'))
    }

    fn replace_section(document: &str, heading_path: &[&str], new_body: &str) -> io::Result<String> {
        let span = find_section_span(document, heading_path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("markdown section path not found: {}", heading_path.join(" / ")),
            )
        })?;

        let mut out = String::new();
        out.push_str(&document[..span.body_start]);
        out.push_str(new_body.trim_matches('\n'));
        out.push('\n');
        out.push_str(&document[span.body_end..]);
        Ok(out)
    }

    fn append_section(document: &str, level: u8, heading: &str, body: &str) -> io::Result<String> {
        if !(1..=6).contains(&level) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "markdown heading level must be between 1 and 6",
            ));
        }
        if heading.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "markdown heading cannot be empty",
            ));
        }

        let mut out = document.trim_end_matches('\n').to_owned();
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&"#".repeat(level as usize));
        out.push(' ');
        out.push_str(heading.trim());
        out.push_str("\n\n");
        out.push_str(body.trim_matches('\n'));
        out.push('\n');
        Ok(out)
    }
}

#[derive(Debug, Clone, Copy)]
struct SectionSpan {
    body_start: usize,
    body_end: usize,
}

fn find_section_span(document: &str, heading_path: &[&str]) -> Option<SectionSpan> {
    if heading_path.is_empty() {
        return None;
    }

    let headings = collect_headings(document);
    let mut stack: Vec<&str> = Vec::new();

    for (index, heading) in headings.iter().enumerate() {
        while stack.len() >= heading.level as usize {
            stack.pop();
        }
        stack.push(heading.title);

        if stack.as_slice() == heading_path {
            let body_start = heading.line_end;
            let body_end = headings[index + 1..]
                .iter()
                .find(|candidate| candidate.level <= heading.level)
                .map(|candidate| candidate.line_start)
                .unwrap_or(document.len());
            return Some(SectionSpan { body_start, body_end });
        }
    }

    None
}

#[derive(Debug, Clone, Copy)]
struct Heading<'a> {
    level: u8,
    title: &'a str,
    line_start: usize,
    line_end: usize,
}

fn collect_headings(document: &str) -> Vec<Heading<'_>> {
    let mut headings = Vec::new();
    let mut offset = 0usize;

    for line in document.split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches(['\r', '\n']);
        if let Some((level, title)) = parse_heading(line_without_newline) {
            headings.push(Heading {
                level,
                title,
                line_start: offset,
                line_end: offset + line.len(),
            });
        }
        offset += line.len();
    }

    if !document.ends_with('\n') {
        // split_inclusive already emitted the final unterminated line. Nothing
        // else is needed; this branch documents the offset behavior.
    }

    headings
}

fn parse_heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.as_bytes().iter().take_while(|byte| **byte == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    if line.as_bytes().get(hashes) != Some(&b' ') {
        return None;
    }
    let title = line[hashes + 1..].trim();
    if title.is_empty() {
        return None;
    }
    Some((hashes as u8, title))
}
