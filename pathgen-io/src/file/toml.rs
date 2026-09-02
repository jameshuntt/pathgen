use serde::{de::DeserializeOwned, Serialize};
use std::{fs, fs::File, io::{ErrorKind, Read, Result as IoResult}, path::Path};
use ::toml::Value;

use crate::{structured::invalid_data, MergePolicy, ResultExt, StructuredDocument};

/// Structured-document adapter for TOML value trees.
#[derive(Debug, Clone, Copy, Default)]
pub struct TomlDocument;

pub fn to_toml_file<T, P>(path: P, value: &T) -> IoResult<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    let toml_str = to_toml_string(value)?;
    fs::write(path, toml_str)
}

pub fn from_toml_file<T, P>(path: P) -> IoResult<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let mut contents = String::new();
    File::open(path)?.read_to_string(&mut contents)?;
    ::toml::from_str(&contents).invalid_data()
}

pub fn from_toml_file_or_default<T, P>(path: P) -> IoResult<T>
where
    T: DeserializeOwned + Default,
    P: AsRef<Path>,
{
    match File::open(path) {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            ::toml::from_str(&contents).invalid_data()
        }
        Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e),
    }
}

pub fn to_toml_string<T: Serialize>(value: &T) -> IoResult<String> {
    ::toml::to_string(value).invalid_data()
}

pub fn from_toml_str<T: DeserializeOwned>(s: &str) -> IoResult<T> {
    ::toml::from_str(s).invalid_data()
}

pub fn load_raw_toml<P>(path: P) -> IoResult<Value>
where
    P: AsRef<Path>,
{
    TomlDocument::load_file(path)
}

pub fn get_nested<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    TomlDocument::get_nested(value, path)
}

pub fn try_set_nested(value: &mut Value, path: &[&str], new_val: Value) -> IoResult<()> {
    TomlDocument::set_nested(value, path, new_val)
}

pub fn merge_toml_with_policy(a: &mut Value, b: &Value, policy: MergePolicy) -> IoResult<()> {
    TomlDocument::merge_with_policy(a, b, policy)
}

impl StructuredDocument for TomlDocument {
    type Value = Value;

    fn parse_str(s: &str) -> IoResult<Self::Value> {
        ::toml::from_str(s).invalid_data()
    }

    fn to_string(value: &Self::Value) -> IoResult<String> {
        ::toml::to_string(value).invalid_data()
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
            if !current.is_table() {
                return Err(invalid_data(format!(
                    "expected TOML table while setting nested key `{key}`"
                )));
            }

            let table = current.as_table_mut().expect("checked is_table");
            current = table
                .entry((*key).to_owned())
                .or_insert_with(|| Value::Table(Default::default()));
        }

        if !current.is_table() {
            return Err(invalid_data("expected TOML table at nested assignment target"));
        }

        let last = path[path.len() - 1];
        current
            .as_table_mut()
            .expect("checked is_table")
            .insert(last.to_owned(), new_value);
        Ok(())
    }

    fn merge_with_policy(
        base: &mut Self::Value,
        overlay: &Self::Value,
        policy: MergePolicy,
    ) -> IoResult<()> {
        match (base, overlay) {
            (Value::Table(base_table), Value::Table(overlay_table)) => {
                for (key, overlay_value) in overlay_table {
                    match base_table.get_mut(key) {
                        Some(base_value) => {
                            TomlDocument::merge_with_policy(base_value, overlay_value, policy)?;
                        }
                        None => {
                            base_table.insert(key.clone(), overlay_value.clone());
                        }
                    }
                }
                Ok(())
            }
            (Value::Array(base_items), Value::Array(overlay_items))
                if policy == MergePolicy::AppendArrays =>
            {
                base_items.extend(overlay_items.iter().cloned());
                Ok(())
            }
            (base_value, overlay_value) if policy == MergePolicy::ErrorOnConflict => {
                if base_value == overlay_value {
                    Ok(())
                } else {
                    Err(invalid_data("TOML merge conflict"))
                }
            }
            (base_value, overlay_value) => {
                *base_value = overlay_value.clone();
                Ok(())
            }
        }
    }
}
