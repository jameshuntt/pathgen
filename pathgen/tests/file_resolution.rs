use std::path::PathBuf;

use pathgen::{AppPathRole, PathPolicy};

#[test]
fn config_file_resolves_under_config_dir() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    assert_eq!(
        paths.config_file("app.toml").unwrap().path,
        PathBuf::from("/etc/archive-api/app.toml")
    );
}

#[test]
fn log_file_resolves_under_log_dir() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    assert_eq!(
        paths.log_file("server.log").unwrap().path,
        PathBuf::from("/var/log/archive-api/server.log")
    );
}

#[test]
fn runtime_socket_resolves_under_runtime_dir() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    assert_eq!(
        paths.socket("api.sock").unwrap().path,
        PathBuf::from("/run/archive-api/api.sock")
    );
}

#[test]
fn nested_relative_file_paths_are_allowed() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    assert_eq!(
        paths
            .file(AppPathRole::Logs, "workers/worker-1.log")
            .unwrap()
            .path,
        PathBuf::from("/var/log/archive-api/workers/worker-1.log")
    );
}

#[test]
fn absolute_file_names_are_rejected() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    assert!(paths.file(AppPathRole::Config, "/tmp/evil.toml").is_err());
}

#[test]
fn parent_traversal_file_names_are_rejected() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    assert!(paths.file(AppPathRole::Config, "../evil.toml").is_err());
    assert!(paths
        .file(AppPathRole::Config, "nested/../../evil.toml")
        .is_err());
}

#[test]
fn empty_file_names_are_rejected() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    assert!(paths.file(AppPathRole::Config, "").is_err());
}

#[test]
fn dot_only_file_names_are_rejected() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    assert!(paths.file(AppPathRole::Config, ".").is_err());
}
