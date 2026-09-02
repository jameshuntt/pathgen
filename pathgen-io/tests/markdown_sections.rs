#[cfg(feature = "file-markdown")]
#[test]
fn markdown_frontmatter_can_be_replaced_or_inserted() {
    use pathgen_io::file::markdown::{replace_frontmatter, split_frontmatter};

    let doc = "---\ntitle: Old\n---\n# Intro\nBody\n";
    let parts = split_frontmatter(doc).unwrap();
    assert_eq!(parts.frontmatter.trim(), "title: Old");
    assert!(parts.rest.starts_with("# Intro"));

    let replaced = replace_frontmatter(doc, "title: New");
    assert!(replaced.starts_with("---\ntitle: New\n---\n# Intro"));

    let inserted = replace_frontmatter("# Intro\n", "title: Inserted");
    assert!(inserted.starts_with("---\ntitle: Inserted\n---\n# Intro"));
}

#[cfg(feature = "file-markdown")]
#[test]
fn markdown_sections_can_be_read_replaced_and_appended() {
    use pathgen_io::file::markdown::{append_section, get_section, replace_section};

    let doc = "# Root\nintro\n\n## Install\nold body\n\n## Usage\nrun it\n";

    assert_eq!(get_section(doc, &["Root", "Install"]).unwrap(), "old body");

    let replaced = replace_section(doc, &["Root", "Install"], "new body").unwrap();
    assert!(replaced.contains("## Install\nnew body\n"));
    assert!(replaced.contains("## Usage\nrun it"));

    let appended = append_section(&replaced, 2, "Audit", "receipt details").unwrap();
    assert!(appended.contains("## Audit\n\nreceipt details\n"));
}
