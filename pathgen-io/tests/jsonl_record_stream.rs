#[cfg(feature = "file-jsonl")]
#[test]
fn jsonl_record_stream_roundtrip_and_append() {
    use pathgen_io::file::jsonl::{append_jsonl_record, from_jsonl_file, parse_jsonl_values, to_jsonl_file};
    use serde_json::json;

    let dir = std::env::temp_dir().join(format!(
        "pathgen-jsonl-{}-{}",
        std::process::id(),
        "roundtrip"
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("events.jsonl");

    let records = vec![json!({"event":"created"}), json!({"event":"validated"})];
    to_jsonl_file(&path, &records).unwrap();
    append_jsonl_record(&path, &json!({"event":"written"})).unwrap();

    let loaded: Vec<serde_json::Value> = from_jsonl_file(&path).unwrap();
    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded[2]["event"], "written");

    let parsed = parse_jsonl_values("{\"a\":1}\n\n{\"b\":2}\n").unwrap();
    assert_eq!(parsed.len(), 2);
}

#[cfg(feature = "file-jsonl")]
#[test]
fn jsonl_reports_line_number_for_invalid_record() {
    use pathgen_io::file::jsonl::parse_jsonl_values;

    let err = parse_jsonl_values("{\"ok\":true}\nnot-json\n").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("line 2"));
}
