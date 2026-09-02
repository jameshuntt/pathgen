use serde::{de::DeserializeOwned, Serialize};
use std::{fs, io::Result as IoResult, path::Path};

use crate::{structured::invalid_data, MergePolicy, ResultExt, StructuredDocument};

/// Structured-document adapter for JSON value trees.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonDocument;

pub fn to_json_vec<T: Serialize>(value: &T) -> IoResult<Vec<u8>> {
    serde_json::to_vec(value).invalid_data()
}

pub fn to_json_vec_pretty<T: Serialize>(value: &T) -> IoResult<Vec<u8>> {
    serde_json::to_vec_pretty(value).invalid_data()
}

pub fn to_json_string_pretty<T: Serialize>(value: &T) -> IoResult<String> {
    serde_json::to_string_pretty(value).invalid_data()
}

pub fn to_json_file<T, P>(path: P, value: &T) -> IoResult<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    fs::write(path, to_json_vec_pretty(value)?)
}

pub fn from_json_file<T, P>(path: P) -> IoResult<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).invalid_data()
}

pub fn from_json_str<T: DeserializeOwned>(s: &str) -> IoResult<T> {
    serde_json::from_str(s).invalid_data()
}

pub fn load_raw_json<P>(path: P) -> IoResult<serde_json::Value>
where
    P: AsRef<Path>,
{
    JsonDocument::load_file(path)
}

pub fn get_nested<'a>(
    value: &'a serde_json::Value,
    path: &[&str],
) -> Option<&'a serde_json::Value> {
    JsonDocument::get_nested(value, path)
}

pub fn set_nested(
    value: &mut serde_json::Value,
    path: &[&str],
    new_val: serde_json::Value,
) -> IoResult<()> {
    JsonDocument::set_nested(value, path, new_val)
}

pub fn merge_json(a: &mut serde_json::Value, b: &serde_json::Value) -> IoResult<()> {
    JsonDocument::merge(a, b)
}

pub fn merge_json_with_policy(
    a: &mut serde_json::Value,
    b: &serde_json::Value,
    policy: MergePolicy,
) -> IoResult<()> {
    JsonDocument::merge_with_policy(a, b, policy)
}

impl StructuredDocument for JsonDocument {
    type Value = serde_json::Value;

    fn parse_str(s: &str) -> IoResult<Self::Value> {
        serde_json::from_str(s).invalid_data()
    }

    fn to_string(value: &Self::Value) -> IoResult<String> {
        serde_json::to_string_pretty(value).invalid_data()
    }

    fn get_nested<'a>(value: &'a Self::Value, path: &[&str]) -> Option<&'a Self::Value> {
        path.iter().try_fold(value, |acc, key| acc.get(*key))
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
            if !current.is_object() {
                return Err(invalid_data(format!(
                    "expected JSON object while setting nested key `{key}`"
                )));
            }

            let object = current.as_object_mut().expect("checked is_object");
            current = object
                .entry((*key).to_owned())
                .or_insert_with(|| serde_json::Value::Object(Default::default()));
        }

        if !current.is_object() {
            return Err(invalid_data("expected JSON object at nested assignment target"));
        }

        let last = path[path.len() - 1];
        current
            .as_object_mut()
            .expect("checked is_object")
            .insert(last.to_owned(), new_value);
        Ok(())
    }

    fn merge_with_policy(
        base: &mut Self::Value,
        overlay: &Self::Value,
        policy: MergePolicy,
    ) -> IoResult<()> {
        match (base, overlay) {
            (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
                for (key, overlay_value) in overlay_map {
                    match base_map.get_mut(key) {
                        Some(base_value) => {
                            JsonDocument::merge_with_policy(base_value, overlay_value, policy)?;
                        }
                        None => {
                            base_map.insert(key.clone(), overlay_value.clone());
                        }
                    }
                }
                Ok(())
            }
            (serde_json::Value::Array(base_items), serde_json::Value::Array(overlay_items))
                if policy == MergePolicy::AppendArrays =>
            {
                base_items.extend(overlay_items.iter().cloned());
                Ok(())
            }
            (base_value, overlay_value) if policy == MergePolicy::ErrorOnConflict => {
                if base_value == overlay_value {
                    Ok(())
                } else {
                    Err(invalid_data("JSON merge conflict"))
                }
            }
            (base_value, overlay_value) => {
                *base_value = overlay_value.clone();
                Ok(())
            }
        }
    }
}
