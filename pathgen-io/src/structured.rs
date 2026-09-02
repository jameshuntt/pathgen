use std::{fs, io, path::Path};

/// Merge behavior for structured document overlays.
///
/// The default enterprise-safe behavior is recursive map/table merge with
/// scalar replacement and array/sequence replacement. More specialized formats
/// can interpret the policy in their own honest terms, but they should not
/// silently invent semantics for formats that do not support a tree model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergePolicy {
    /// Recursively merge maps/tables. Non-map values from the overlay replace
    /// the base value. Arrays/sequences replace by default.
    #[default]
    Replace,
    /// Recursively merge maps/tables. Arrays/sequences append overlay items.
    AppendArrays,
    /// Recursively merge maps/tables, but return an error when a leaf value
    /// would be overwritten by a different leaf value.
    ErrorOnConflict,
}


/// Shared operations for formats with an actual structured value tree.
///
/// This trait is intended for TOML, JSON, YAML, and similar document formats.
/// It is deliberately not implemented for raw TXT because plain text does not
/// have honest nested-path or recursive-merge semantics.
pub trait StructuredDocument {
    type Value: Clone;

    fn parse_str(s: &str) -> io::Result<Self::Value>;
    fn to_string(value: &Self::Value) -> io::Result<String>;

    fn load_file<P>(path: P) -> io::Result<Self::Value>
    where
        P: AsRef<Path>,
    {
        let contents = fs::read_to_string(path)?;
        Self::parse_str(&contents)
    }

    fn write_file<P>(path: P, value: &Self::Value) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        fs::write(path, Self::to_string(value)?)
    }

    fn get_nested<'a>(value: &'a Self::Value, path: &[&str]) -> Option<&'a Self::Value>;

    fn set_nested(
        value: &mut Self::Value,
        path: &[&str],
        new_value: Self::Value,
    ) -> io::Result<()>;

    fn merge_with_policy(
        base: &mut Self::Value,
        overlay: &Self::Value,
        policy: MergePolicy,
    ) -> io::Result<()>;

    fn merge(base: &mut Self::Value, overlay: &Self::Value) -> io::Result<()> {
        Self::merge_with_policy(base, overlay, MergePolicy::default())
    }
}

#[allow(dead_code)]
pub(crate) fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
