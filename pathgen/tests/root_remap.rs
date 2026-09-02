use std::path::PathBuf;

use pathgen::{AppPathRole, PathPolicy};

#[test]
fn root_remap_for_log_file() {
    let paths = PathPolicy::prod("archive-api")
        .unwrap()
        .with_root("/rootfs");
    assert_eq!(
        paths.log_file("server.log").unwrap().path,
        PathBuf::from("/rootfs/var/log/archive-api/server.log")
    );
}

#[test]
fn root_remap_for_config_file() {
    let paths = PathPolicy::prod("archive-api")
        .unwrap()
        .with_root("/rootfs");
    assert_eq!(
        paths.config_file("app.toml").unwrap().path,
        PathBuf::from("/rootfs/etc/archive-api/app.toml")
    );
}

#[test]
fn root_remap_for_runtime_socket() {
    let paths = PathPolicy::prod("archive-api")
        .unwrap()
        .with_root("/rootfs");
    assert_eq!(
        paths.socket("api.sock").unwrap().path,
        PathBuf::from("/rootfs/run/archive-api/api.sock")
    );
}

#[test]
fn root_remap_for_systemd_units() {
    let paths = PathPolicy::prod("archive-api")
        .unwrap()
        .with_root("/rootfs");
    assert_eq!(
        paths.dir(AppPathRole::SystemdUnits).unwrap().path,
        PathBuf::from("/rootfs/etc/systemd/system")
    );
}

#[test]
fn root_remap_still_rejects_unsafe_file_name() {
    let paths = PathPolicy::prod("archive-api")
        .unwrap()
        .with_root("/rootfs");
    assert!(paths.log_file("../../evil.log").is_err());
}
