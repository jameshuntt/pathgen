use serde::{de::DeserializeOwned, Serialize};
use std::{fs, io::Result as IoResult, path::Path};

use crate::{structured::invalid_data, MergePolicy, ResultExt, StructuredDocument};

/// Structured-document adapter for YAML value trees.
///
/// This uses `serde_yaml::Value`, so comments, anchors, and some YAML-specific
/// presentation details are not preserved across parse/stringify roundtrips.
#[derive(Debug, Clone, Copy, Default)]
pub struct YamlDocument;

pub fn to_yaml_string<T: Serialize>(value: &T) -> IoResult<String> {
    serde_yaml::to_string(value).invalid_data()
}

pub fn to_yaml_file<T, P>(path: P, value: &T) -> IoResult<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    fs::write(path, to_yaml_string(value)?)
}

pub fn from_yaml_file<T, P>(path: P) -> IoResult<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let contents = fs::read_to_string(path)?;
    serde_yaml::from_str(&contents).invalid_data()
}

pub fn from_yaml_str<T: DeserializeOwned>(s: &str) -> IoResult<T> {
    serde_yaml::from_str(s).invalid_data()
}

pub fn load_raw_yaml<P>(path: P) -> IoResult<serde_yaml::Value>
where
    P: AsRef<Path>,
{
    YamlDocument::load_file(path)
}

pub fn get_nested<'a>(
    value: &'a serde_yaml::Value,
    path: &[&str],
) -> Option<&'a serde_yaml::Value> {
    YamlDocument::get_nested(value, path)
}

pub fn set_nested(
    value: &mut serde_yaml::Value,
    path: &[&str],
    new_val: serde_yaml::Value,
) -> IoResult<()> {
    YamlDocument::set_nested(value, path, new_val)
}

pub fn merge_yaml(a: &mut serde_yaml::Value, b: &serde_yaml::Value) -> IoResult<()> {
    YamlDocument::merge(a, b)
}

pub fn merge_yaml_with_policy(
    a: &mut serde_yaml::Value,
    b: &serde_yaml::Value,
    policy: MergePolicy,
) -> IoResult<()> {
    YamlDocument::merge_with_policy(a, b, policy)
}

impl StructuredDocument for YamlDocument {
    type Value = serde_yaml::Value;

    fn parse_str(s: &str) -> IoResult<Self::Value> {
        serde_yaml::from_str(s).invalid_data()
    }

    fn to_string(value: &Self::Value) -> IoResult<String> {
        serde_yaml::to_string(value).invalid_data()
    }

    fn get_nested<'a>(value: &'a Self::Value, path: &[&str]) -> Option<&'a Self::Value> {
        let mut current = value;
        for key in path {
            let mapping = current.as_mapping()?;
            current = mapping.get(serde_yaml::Value::String((*key).to_owned()))?;
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
            if !current.is_mapping() {
                return Err(invalid_data(format!(
                    "expected YAML mapping while setting nested key `{key}`"
                )));
            }

            let key_value = serde_yaml::Value::String((*key).to_owned());
            let mapping = current.as_mapping_mut().expect("checked is_mapping");
            if !mapping.contains_key(&key_value) {
                mapping.insert(
                    key_value.clone(),
                    serde_yaml::Value::Mapping(Default::default()),
                );
            }
            current = mapping.get_mut(&key_value).expect("inserted or existed");
        }

        if !current.is_mapping() {
            return Err(invalid_data("expected YAML mapping at nested assignment target"));
        }

        let last = serde_yaml::Value::String(path[path.len() - 1].to_owned());
        current
            .as_mapping_mut()
            .expect("checked is_mapping")
            .insert(last, new_value);
        Ok(())
    }

    fn merge_with_policy(
        base: &mut Self::Value,
        overlay: &Self::Value,
        policy: MergePolicy,
    ) -> IoResult<()> {
        match (base, overlay) {
            (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(overlay_map)) => {
                for (key, overlay_value) in overlay_map {
                    match base_map.get_mut(key) {
                        Some(base_value) => {
                            YamlDocument::merge_with_policy(base_value, overlay_value, policy)?;
                        }
                        None => {
                            base_map.insert(key.clone(), overlay_value.clone());
                        }
                    }
                }
                Ok(())
            }
            (serde_yaml::Value::Sequence(base_items), serde_yaml::Value::Sequence(overlay_items))
                if policy == MergePolicy::AppendArrays =>
            {
                base_items.extend(overlay_items.iter().cloned());
                Ok(())
            }
            (base_value, overlay_value) if policy == MergePolicy::ErrorOnConflict => {
                if base_value == overlay_value {
                    Ok(())
                } else {
                    Err(invalid_data("YAML merge conflict"))
                }
            }
            (base_value, overlay_value) => {
                *base_value = overlay_value.clone();
                Ok(())
            }
        }
    }
}
