use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use pathgen_core::{AppPathRole, PathPolicy};

fn unique_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!("pathgen-{name}-{nanos}"))
}

#[test]
fn ensure_dir_creates_role_directory() {
    let root = unique_root("ensure-dir");
    let paths = PathPolicy::prod("archive-api").unwrap().with_root(&root);

    let resolved = paths.ensure_dir(AppPathRole::Logs).unwrap();
    assert!(resolved.path.exists());
    assert!(resolved.path.is_dir());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ensure_parent_for_file_creates_parent_only() {
    let root = unique_root("ensure-parent");
    let paths = PathPolicy::prod("archive-api").unwrap().with_root(&root);

    let resolved = paths
        .ensure_parent_for_file(AppPathRole::Logs, "workers/one.log")
        .unwrap();
    let parent = resolved.path.parent().unwrap();

    assert!(parent.exists());
    assert!(!resolved.path.exists());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn validate_dir_exists_succeeds_after_ensure_dir() {
    let root = unique_root("validate-exists");
    let paths = PathPolicy::prod("archive-api").unwrap().with_root(&root);

    paths.ensure_dir(AppPathRole::Runtime).unwrap();
    let resolved = paths.validate_dir_exists(AppPathRole::Runtime).unwrap();

    assert!(resolved.path.exists());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn validate_dir_exists_errors_before_create() {
    let root = unique_root("validate-missing");
    let paths = PathPolicy::prod("archive-api").unwrap().with_root(&root);

    assert!(paths.validate_dir_exists(AppPathRole::Runtime).is_err());
}

#[cfg(unix)]
#[test]
fn ensure_dir_mode_sets_requested_mode() {
    use std::os::unix::fs::PermissionsExt;

    let root = unique_root("mode");
    let paths = PathPolicy::prod("archive-api").unwrap().with_root(&root);

    let resolved = paths.ensure_dir_mode(AppPathRole::Runtime, 0o750).unwrap();
    let mode = fs::metadata(&resolved.path).unwrap().permissions().mode() & 0o777;

    assert_eq!(mode, 0o750);

    fs::remove_dir_all(root).unwrap();
}
