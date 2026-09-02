use std::{fs, io::Result as IoResult, path::Path};

use xmltree::{Element, EmitterConfig, XMLNode};

use crate::{structured::invalid_data, MergePolicy, ResultExt, StructuredDocument};

/// Structured-document adapter for simple XML element trees.
///
/// This uses `xmltree::Element`. It preserves element/attribute structure but
/// should not be treated as a perfect lexical-preservation editor for comments,
/// whitespace, or formatting-sensitive XML workflows.
#[derive(Debug, Clone, Copy, Default)]
pub struct XmlDocument;

pub fn parse_xml_str(s: &str) -> IoResult<Element> {
    XmlDocument::parse_str(s)
}

pub fn to_xml_string(value: &Element) -> IoResult<String> {
    XmlDocument::to_string(value)
}

pub fn to_xml_file<P>(path: P, value: &Element) -> IoResult<()>
where
    P: AsRef<Path>,
{
    fs::write(path, to_xml_string(value)?)
}

pub fn from_xml_file<P>(path: P) -> IoResult<Element>
where
    P: AsRef<Path>,
{
    XmlDocument::load_file(path)
}

pub fn get_nested<'a>(value: &'a Element, path: &[&str]) -> Option<&'a Element> {
    XmlDocument::get_nested(value, path)
}

pub fn set_nested(value: &mut Element, path: &[&str], new_val: Element) -> IoResult<()> {
    XmlDocument::set_nested(value, path, new_val)
}

pub fn merge_xml(a: &mut Element, b: &Element) -> IoResult<()> {
    XmlDocument::merge(a, b)
}

pub fn merge_xml_with_policy(a: &mut Element, b: &Element, policy: MergePolicy) -> IoResult<()> {
    XmlDocument::merge_with_policy(a, b, policy)
}

impl StructuredDocument for XmlDocument {
    type Value = Element;

    fn parse_str(s: &str) -> IoResult<Self::Value> {
        Element::parse(s.as_bytes()).invalid_data()
    }

    fn to_string(value: &Self::Value) -> IoResult<String> {
        let mut bytes = Vec::new();
        value
            .write_with_config(&mut bytes, EmitterConfig::new().perform_indent(true))
            .invalid_data()?;
        String::from_utf8(bytes).invalid_data()
    }

    fn get_nested<'a>(value: &'a Self::Value, path: &[&str]) -> Option<&'a Self::Value> {
        let mut current = value;
        for key in path {
            current = find_child_element(current, key)?;
        }
        Some(current)
    }

    fn set_nested(
        value: &mut Self::Value,
        path: &[&str],
        new_value: Self::Value,
    ) -> IoResult<()> {
        if path.is_empty() {
            *value = new_value;
            return Ok(());
        }

        let mut current = value;
        for key in &path[..path.len() - 1] {
            current = find_or_insert_child_element(current, key);
        }

        let last = path[path.len() - 1];
        if let Some(index) = child_element_index(current, last) {
            current.children[index] = XMLNode::Element(new_value);
        } else {
            current.children.push(XMLNode::Element(new_value));
        }
        Ok(())
    }

    fn merge_with_policy(
        base: &mut Self::Value,
        overlay: &Self::Value,
        policy: MergePolicy,
    ) -> IoResult<()> {
        merge_elements(base, overlay, policy)
    }
}

fn find_child_element<'a>(element: &'a Element, name: &str) -> Option<&'a Element> {
    element.children.iter().find_map(|node| match node {
        XMLNode::Element(child) if child.name == name => Some(child),
        _ => None,
    })
}

fn child_element_index(element: &Element, name: &str) -> Option<usize> {
    element
        .children
        .iter()
        .position(|node| matches!(node, XMLNode::Element(child) if child.name == name))
}

fn find_or_insert_child_element<'a>(element: &'a mut Element, name: &str) -> &'a mut Element {
    if let Some(index) = child_element_index(element, name) {
        return match &mut element.children[index] {
            XMLNode::Element(child) => child,
            _ => unreachable!("child_element_index returns only element nodes"),
        };
    }

    element.children.push(XMLNode::Element(Element::new(name)));
    match element.children.last_mut().expect("inserted child") {
        XMLNode::Element(child) => child,
        _ => unreachable!("inserted element node"),
    }
}

fn merge_elements(base: &mut Element, overlay: &Element, policy: MergePolicy) -> IoResult<()> {
    for (key, value) in &overlay.attributes {
        match base.attributes.get(key) {
            Some(existing) if policy == MergePolicy::ErrorOnConflict && existing != value => {
                return Err(invalid_data(format!("XML attribute merge conflict for `{key}`")));
            }
            _ => {
                base.attributes.insert(key.clone(), value.clone());
            }
        }
    }

    for overlay_child in &overlay.children {
        match overlay_child {
            XMLNode::Element(overlay_element) => {
                if let Some(index) = child_element_index(base, &overlay_element.name) {
                    match &mut base.children[index] {
                        XMLNode::Element(base_element) => {
                            merge_elements(base_element, overlay_element, policy)?;
                        }
                        _ => unreachable!("child_element_index returns only element nodes"),
                    }
                } else {
                    base.children.push(XMLNode::Element(overlay_element.clone()));
                }
            }
            XMLNode::Text(overlay_text) => {
                if policy == MergePolicy::ErrorOnConflict {
                    let existing_text = base.children.iter().find_map(|node| match node {
                        XMLNode::Text(text) => Some(text),
                        _ => None,
                    });
                    if let Some(existing) = existing_text {
                        if existing != overlay_text {
                            return Err(invalid_data("XML text merge conflict"));
                        }
                    }
                }

                base.children.retain(|node| !matches!(node, XMLNode::Text(_)));
                base.children.push(XMLNode::Text(overlay_text.clone()));
            }
            other => {
                if policy != MergePolicy::ErrorOnConflict {
                    base.children.push(other.clone());
                }
            }
        }
    }

    Ok(())
}
