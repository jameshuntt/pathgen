use std::path::{Path, PathBuf};

use pathgen::{checked_join, validate_app_name, PathPolicy};

#[test]
fn app_name_good_cases() {
    for name in ["app", "app-1", "app_1", "app.v1", "A1"] {
        validate_app_name(name).unwrap();
    }
}

#[test]
fn app_name_bad_cases() {
    for name in [
        "",
        " ",
        " app",
        "app ",
        ".",
        "..",
        "bad/name",
        "bad\\name",
        "bad name",
        "-bad",
        "bad$var",
    ] {
        assert!(
            validate_app_name(name).is_err(),
            "expected `{name}` to be rejected"
        );
    }
}

#[test]
fn app_name_length_limit() {
    let too_long = "a".repeat(129);
    assert!(validate_app_name(&too_long).is_err());
}

#[test]
fn policy_rejects_bad_app_names() {
    assert!(PathPolicy::prod("bad/name").is_err());
    assert!(PathPolicy::prod(" bad").is_err());
    assert!(PathPolicy::prod("..").is_err());
}

#[test]
fn checked_join_preserves_base_for_normal_child() {
    assert_eq!(
        checked_join(Path::new("/base"), "child").unwrap(),
        PathBuf::from("/base/child")
    );
}

#[test]
fn checked_join_allows_nested_normal_child() {
    assert_eq!(
        checked_join(Path::new("/base"), "a/b/c").unwrap(),
        PathBuf::from("/base/a/b/c")
    );
}

#[test]
fn checked_join_rejects_absolute_child() {
    assert!(checked_join(Path::new("/base"), "/abs").is_err());
}

#[test]
fn checked_join_rejects_traversal() {
    assert!(checked_join(Path::new("/base"), "../abs").is_err());
}

#[test]
fn checked_join_rejects_traversal_after_normal_components() {
    assert!(checked_join(Path::new("/base"), "a/../b").is_err());
}

#[test]
fn checked_join_rejects_empty() {
    assert!(checked_join(Path::new("/base"), "").is_err());
}
