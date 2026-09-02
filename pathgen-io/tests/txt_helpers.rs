#![cfg(feature = "file-txt")]

use std::{path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

use pathgen_io::file::txt::{from_txt_file, to_txt_file};

fn unique_file(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!("pathgen-{name}-{nanos}.txt"))
}

#[test]
fn txt_roundtrip_file() {
    let path = unique_file("txt-roundtrip");

    to_txt_file(&path, "hello pathgen").unwrap();
    let loaded = from_txt_file(&path).unwrap();

    assert_eq!(loaded, "hello pathgen");
    std::fs::remove_file(path).unwrap();
}
