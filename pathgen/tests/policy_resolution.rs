use std::path::PathBuf;

use pathgen::{AppPathRole, Env, PathPolicy};

#[test]
fn prod_config_dir() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::Config).unwrap().path,
        PathBuf::from("/etc/video-archive-api")
    );
}

#[test]
fn prod_log_dir() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::Logs).unwrap().path,
        PathBuf::from("/var/log/video-archive-api")
    );
}

#[test]
fn prod_cache_dir() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::Cache).unwrap().path,
        PathBuf::from("/var/cache/video-archive-api")
    );
}

#[test]
fn prod_runtime_dir() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::Runtime).unwrap().path,
        PathBuf::from("/run/video-archive-api")
    );
}

#[test]
fn prod_temp_dir() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::Temp).unwrap().path,
        PathBuf::from("/tmp/video-archive-api")
    );
}

#[test]
fn prod_state_dir() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::State).unwrap().path,
        PathBuf::from("/var/lib/video-archive-api")
    );
}

#[test]
fn prod_data_dir() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::Data).unwrap().path,
        PathBuf::from("/var/lib/video-archive-api/data")
    );
}

#[test]
fn prod_project_root() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::ProjectRoot).unwrap().path,
        PathBuf::from("/opt/video-archive-api")
    );
}

#[test]
fn prod_systemd_units_dir() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::SystemdUnits).unwrap().path,
        PathBuf::from("/etc/systemd/system")
    );
}

#[test]
fn prod_certs_dir() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::Certs).unwrap().path,
        PathBuf::from("/etc/video-archive-api/certs")
    );
}

#[test]
fn prod_secrets_dir() {
    let paths = PathPolicy::prod("video-archive-api").unwrap();
    assert_eq!(
        paths.dir(AppPathRole::Secrets).unwrap().path,
        PathBuf::from("/etc/video-archive-api/secrets")
    );
}

#[test]
fn dev_layout_is_isolated() {
    let paths = PathPolicy::new("video-archive-api", Env::Dev).unwrap();
    assert_eq!(
        paths.dir(AppPathRole::Runtime).unwrap().path,
        PathBuf::from("/tmp/pathgen-dev/video-archive-api/runtime")
    );
}

#[test]
fn test_layout_is_isolated() {
    let paths = PathPolicy::new("video-archive-api", Env::Test).unwrap();
    assert_eq!(
        paths.dir(AppPathRole::Runtime).unwrap().path,
        PathBuf::from("/tmp/pathgen-test/video-archive-api/runtime")
    );
}

#[test]
fn custom_test_base_is_supported() {
    let paths = PathPolicy::test("video-archive-api")
        .unwrap()
        .with_test_base("/tmp/custom-tests");

    assert_eq!(
        paths.dir(AppPathRole::Logs).unwrap().path,
        PathBuf::from("/tmp/custom-tests/video-archive-api/logs")
    );
}

#[test]
fn custom_dev_base_is_supported() {
    let paths = PathPolicy::dev("video-archive-api")
        .unwrap()
        .with_dev_base("/tmp/custom-dev");

    assert_eq!(
        paths.dir(AppPathRole::Logs).unwrap().path,
        PathBuf::from("/tmp/custom-dev/video-archive-api/logs")
    );
}
