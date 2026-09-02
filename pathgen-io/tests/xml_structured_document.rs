#[cfg(feature = "file-xml")]
#[test]
fn xml_structured_document_nested_set_and_merge() {
    use pathgen_io::{MergePolicy, StructuredDocument};
    use pathgen_io::file::xml::XmlDocument;
    use xmltree::{Element, XMLNode};

    let mut base = XmlDocument::parse_str(r#"<config><service enabled="false"><port>8080</port></service></config>"#).unwrap();
    let overlay = XmlDocument::parse_str(r#"<config><service enabled="true"><host>localhost</host></service></config>"#).unwrap();

    XmlDocument::merge_with_policy(&mut base, &overlay, MergePolicy::Replace).unwrap();

    let service = XmlDocument::get_nested(&base, &["service"]).unwrap();
    assert_eq!(service.attributes.get("enabled").unwrap(), "true");
    assert!(XmlDocument::get_nested(&base, &["service", "host"]).is_some());

    let mut replacement = Element::new("mode");
    replacement.children.push(XMLNode::Text("prod".to_string()));
    XmlDocument::set_nested(&mut base, &["service", "mode"], replacement).unwrap();
    assert!(XmlDocument::get_nested(&base, &["service", "mode"]).is_some());
}

#[cfg(feature = "file-xml")]
#[test]
fn xml_conflict_policy_rejects_attribute_overwrite() {
    use pathgen_io::{MergePolicy, StructuredDocument};
    use pathgen_io::file::xml::XmlDocument;

    let mut base = XmlDocument::parse_str(r#"<config><service enabled="false" /></config>"#).unwrap();
    let overlay = XmlDocument::parse_str(r#"<config><service enabled="true" /></config>"#).unwrap();

    let err = XmlDocument::merge_with_policy(&mut base, &overlay, MergePolicy::ErrorOnConflict)
        .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}
