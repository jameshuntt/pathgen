use pathgen::{
    AppPathRole, Env, PathMutability, PathOwner, PathPolicy, PathSensitivity, POLICY_VERSION,
};

#[test]
fn receipt_has_policy_version() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    let receipt = paths.dir(AppPathRole::Logs).unwrap();
    assert_eq!(receipt.policy_version, POLICY_VERSION);
}

#[test]
fn receipt_records_app_env_role_path() {
    let paths = PathPolicy::new("archive-api", Env::Prod).unwrap();
    let receipt = paths.dir(AppPathRole::Logs).unwrap();

    assert_eq!(receipt.app_name, "archive-api");
    assert_eq!(receipt.env, Env::Prod);
    assert_eq!(receipt.role, AppPathRole::Logs);
    assert_eq!(receipt.path.to_string_lossy(), "/var/log/archive-api");
}

#[test]
fn config_receipt_is_confidential_readonly_root_owned() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    let receipt = paths.dir(AppPathRole::Config).unwrap();

    assert_eq!(receipt.sensitivity, PathSensitivity::Confidential);
    assert_eq!(receipt.mutability, PathMutability::ReadOnly);
    assert_eq!(receipt.owner, PathOwner::Root);
}

#[test]
fn log_receipt_is_internal_append_only_service_owned() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    let receipt = paths.dir(AppPathRole::Logs).unwrap();

    assert_eq!(receipt.sensitivity, PathSensitivity::Internal);
    assert_eq!(receipt.mutability, PathMutability::AppendOnly);
    assert_eq!(receipt.owner, PathOwner::ServiceUser);
}

#[test]
fn runtime_receipt_is_ephemeral() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    let receipt = paths.dir(AppPathRole::Runtime).unwrap();

    assert_eq!(receipt.mutability, PathMutability::Ephemeral);
}

#[test]
fn secrets_are_secret_readonly_root_owned() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    let receipt = paths.dir(AppPathRole::Secrets).unwrap();

    assert_eq!(receipt.sensitivity, PathSensitivity::Secret);
    assert_eq!(receipt.mutability, PathMutability::ReadOnly);
    assert_eq!(receipt.owner, PathOwner::Root);
}

#[test]
fn audit_line_contains_all_major_fields() {
    let paths = PathPolicy::prod("archive-api").unwrap();
    let receipt = paths.config_file("app.toml").unwrap();
    let line = receipt.audit_line();

    for expected in [
        "policy=pathgen.policy.v1",
        "app=archive-api",
        "env=prod",
        "role=config",
        "path=/etc/archive-api/app.toml",
        "sensitivity=confidential",
        "mutability=read_only",
        "owner=root",
    ] {
        assert!(line.contains(expected), "missing `{expected}` in `{line}`");
    }
}
