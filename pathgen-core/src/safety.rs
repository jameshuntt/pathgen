use std::path::{Component, Path, PathBuf};

use crate::PathgenError;

/// Validates an app/service name before it becomes part of a path.
///
/// This intentionally accepts a conservative service-name alphabet:
/// ASCII alphanumeric, `-`, `_`, and `.`.
pub fn validate_app_name(app_name: &str) -> Result<(), PathgenError> {
    if app_name.trim().is_empty() {
        return Err(PathgenError::EmptyAppName);
    }

    if app_name != app_name.trim() {
        return Err(PathgenError::InvalidAppName {
            input: app_name.to_string(),
            reason: "app name cannot have leading or trailing whitespace",
        });
    }

    if app_name == "." || app_name == ".." {
        return Err(PathgenError::InvalidAppName {
            input: app_name.to_string(),
            reason: "app name cannot be dot or parent directory",
        });
    }

    if app_name.len() > 128 {
        return Err(PathgenError::InvalidAppName {
            input: app_name.to_string(),
            reason: "app name is too long; maximum is 128 bytes",
        });
    }

    if app_name.starts_with('-') {
        return Err(PathgenError::InvalidAppName {
            input: app_name.to_string(),
            reason: "app name cannot start with '-'",
        });
    }

    let ok = app_name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));

    if !ok {
        return Err(PathgenError::InvalidAppName {
            input: app_name.to_string(),
            reason: "allowed characters are ASCII letters, digits, '-', '_', and '.'",
        });
    }

    Ok(())
}

/// Safely joins a child path under a base path.
///
/// Rejected:
/// - absolute paths
/// - parent traversal through `..`
/// - platform prefixes
/// - empty child paths
/// - child paths without a normal component, such as `.`
pub fn checked_join(base: &Path, child: impl AsRef<Path>) -> Result<PathBuf, PathgenError> {
    let child = child.as_ref();
    let input = child.display().to_string();

    if child.as_os_str().is_empty() {
        return Err(PathgenError::UnsafeSubpath {
            input,
            reason: "subpath cannot be empty",
        });
    }

    let mut has_normal_component = false;

    for component in child.components() {
        match component {
            Component::Normal(_) => {
                has_normal_component = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(PathgenError::UnsafeSubpath {
                    input,
                    reason: "parent directory traversal is not allowed",
                });
            }
            Component::RootDir => {
                return Err(PathgenError::UnsafeSubpath {
                    input,
                    reason: "absolute paths are not allowed as subpaths",
                });
            }
            Component::Prefix(_) => {
                return Err(PathgenError::UnsafeSubpath {
                    input,
                    reason: "platform path prefixes are not allowed as subpaths",
                });
            }
        }
    }

    if !has_normal_component {
        return Err(PathgenError::UnsafeSubpath {
            input,
            reason: "subpath must contain at least one normal path component",
        });
    }

    Ok(base.join(child))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn app_name_accepts_sane_service_names() {
        validate_app_name("archive-api").unwrap();
        validate_app_name("archive_api").unwrap();
        validate_app_name("archive.api.v1").unwrap();
        validate_app_name("A1").unwrap();
    }

    #[test]
    fn app_name_rejects_empty_and_whitespace() {
        assert!(validate_app_name("").is_err());
        assert!(validate_app_name(" ").is_err());
        assert!(validate_app_name(" app").is_err());
        assert!(validate_app_name("app ").is_err());
    }

    #[test]
    fn app_name_rejects_path_separators_and_shellish_names() {
        assert!(validate_app_name("bad/name").is_err());
        assert!(validate_app_name("bad\\name").is_err());
        assert!(validate_app_name("bad name").is_err());
        assert!(validate_app_name("-bad").is_err());
        assert!(validate_app_name("..").is_err());
    }

    #[test]
    fn checked_join_accepts_relative_children() {
        let joined = checked_join(Path::new("/etc/app"), "config.toml").unwrap();
        assert_eq!(joined, PathBuf::from("/etc/app/config.toml"));

        let nested = checked_join(Path::new("/var/log/app"), "workers/one.log").unwrap();
        assert_eq!(nested, PathBuf::from("/var/log/app/workers/one.log"));
    }

    #[test]
    fn checked_join_rejects_absolute_children() {
        assert!(checked_join(Path::new("/etc/app"), "/tmp/evil.toml").is_err());
    }

    #[test]
    fn checked_join_rejects_parent_traversal() {
        assert!(checked_join(Path::new("/etc/app"), "../evil.toml").is_err());
        assert!(checked_join(Path::new("/etc/app"), "nested/../../evil.toml").is_err());
    }

    #[test]
    fn checked_join_rejects_empty_and_dot_only() {
        assert!(checked_join(Path::new("/etc/app"), "").is_err());
        assert!(checked_join(Path::new("/etc/app"), ".").is_err());
    }
}
