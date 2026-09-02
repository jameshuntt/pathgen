//! Additive, opt-in workspace-location resolution (feature = "workspace").
//!
//! No inheritance, no global state: a caller-invoked upward directory walk
//! (like git's .git discovery) from the calling crate's CARGO_MANIFEST_DIR to
//! the workspace root, which is identified by `workspace_locations.toml`.
//! Intended as a dev-dependency for tests/fixtures; never needed at runtime.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// File name that marks the workspace root and lists the named locations.
pub const LOCATIONS_FILE: &str = "workspace_locations.toml";

/// Why a workspace location could not be resolved.
#[derive(Debug)]
pub enum WorkspaceLocationError {
    /// No directory from `started_at` upward contains [`LOCATIONS_FILE`].
    RootNotFound {
        /// The directory the upward walk began in.
        started_at: PathBuf,
    },
    /// The file was found but does not declare this name under `[locations]`.
    UnknownLocation {
        /// The requested location name.
        name: String,
    },
    /// A line under `[locations]` is not `name = "relative/path"`.
    Malformed {
        /// 1-based line number in [`LOCATIONS_FILE`].
        line: usize,
    },
    /// Reading the locations file failed.
    Io(std::io::Error),
}

impl std::fmt::Display for WorkspaceLocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RootNotFound { started_at } => write!(f, "no {LOCATIONS_FILE} above {}", started_at.display()),
            Self::UnknownLocation { name } => write!(f, "location '{name}' not declared in {LOCATIONS_FILE}"),
            Self::Malformed { line } => write!(f, "{LOCATIONS_FILE} line {line}: expected name = \"path\""),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}
impl std::error::Error for WorkspaceLocationError {}
impl From<std::io::Error> for WorkspaceLocationError {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}

/// Walk upward from `start` until a directory containing LOCATIONS_FILE is found.
pub fn workspace_root_from(start: &Path) -> Result<PathBuf, WorkspaceLocationError> {
    let mut cur = start.to_path_buf();
    loop {
        if cur.join(LOCATIONS_FILE).is_file() { return Ok(cur); }
        if !cur.pop() {
            return Err(WorkspaceLocationError::RootNotFound { started_at: start.to_path_buf() });
        }
    }
}

/// Parse the deliberately-minimal `[locations]` table: `name = "relative/path"` lines.
pub fn load_locations(root: &Path) -> Result<BTreeMap<String, PathBuf>, WorkspaceLocationError> {
    let text = std::fs::read_to_string(root.join(LOCATIONS_FILE))?;
    let mut out = BTreeMap::new();
    let mut in_table = false;
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if line.starts_with('[') { in_table = line == "[locations]"; continue; }
        if !in_table { continue; }
        let (name, rest) = line.split_once('=').ok_or(WorkspaceLocationError::Malformed { line: i + 1 })?;
        let value = rest.trim().trim_matches('"');
        if value.is_empty() { return Err(WorkspaceLocationError::Malformed { line: i + 1 }); }
        out.insert(name.trim().to_string(), root.join(value));
    }
    Ok(out)
}

/// Resolve one named location, walking up from `start` (typically CARGO_MANIFEST_DIR).
pub fn location_from(start: &Path, name: &str) -> Result<PathBuf, WorkspaceLocationError> {
    let root = workspace_root_from(start)?;
    load_locations(&root)?
        .remove(name)
        .ok_or_else(|| WorkspaceLocationError::UnknownLocation { name: name.to_string() })
}

/// Convenience macro: resolve a location from the *calling crate's* manifest dir.
#[macro_export]
macro_rules! workspace_location {
    ($name:expr) => {
        $crate::workspace::location_from(::std::path::Path::new(env!("CARGO_MANIFEST_DIR")), $name)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolves_from_nested_crate_dir() {
        let root = std::env::temp_dir().join(format!("pathgen_ws_{}", std::process::id()));
        let nested = root.join("crates").join("inner");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(root.join("docs").join("walk")).unwrap();
        std::fs::write(root.join(LOCATIONS_FILE), "# comment\n[locations]\nwalkthroughs = \"docs/walk\"\n").unwrap();
        assert_eq!(workspace_root_from(&nested).unwrap(), root);
        let walk = location_from(&nested, "walkthroughs").unwrap();
        assert!(walk.is_dir(), "{walk:?}");
        assert!(matches!(location_from(&nested, "nope"), Err(WorkspaceLocationError::UnknownLocation { .. })));
        assert!(matches!(workspace_root_from(Path::new("/")), Err(WorkspaceLocationError::RootNotFound { .. })));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
