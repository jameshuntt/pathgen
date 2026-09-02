use std::io;

/// Shared operations for text formats that are organized by named sections.
///
/// This is intentionally separate from [`crate::StructuredDocument`]. Markdown
/// has headings and optional frontmatter, not a universal TOML/JSON/YAML-style
/// nested value tree.
pub trait SectionedDocument {
    fn get_section<'a>(document: &'a str, heading_path: &[&str]) -> Option<&'a str>;
    fn replace_section(document: &str, heading_path: &[&str], new_body: &str) -> io::Result<String>;
    fn append_section(document: &str, level: u8, heading: &str, body: &str) -> io::Result<String>;
}
