use serde::{de::DeserializeOwned, Serialize};
use std::{fs, io, io::Result as IoResult, path::Path};

use crate::{record_stream::RecordStreamDocument, ResultExt};

/// Record-stream adapter for JSON Lines / NDJSON files.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonLinesDocument;

impl RecordStreamDocument for JsonLinesDocument {
    type Record = serde_json::Value;

    fn parse_records(s: &str) -> IoResult<Vec<Self::Record>> {
        parse_jsonl_values(s)
    }

    fn records_to_string(records: &[Self::Record]) -> IoResult<String> {
        let mut out = String::new();
        for record in records {
            out.push_str(&serde_json::to_string(record).invalid_data()?);
            out.push('\n');
        }
        Ok(out)
    }

    fn record_to_line(record: &Self::Record) -> IoResult<String> {
        serde_json::to_string(record).invalid_data()
    }
}

pub fn parse_jsonl_values(s: &str) -> IoResult<Vec<serde_json::Value>> {
    let mut records = Vec::new();

    for (index, line) in s.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let value = serde_json::from_str(line).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid JSONL record at line {}: {e}", index + 1),
            )
        })?;
        records.push(value);
    }

    Ok(records)
}

pub fn parse_jsonl_records<T>(s: &str) -> IoResult<Vec<T>>
where
    T: DeserializeOwned,
{
    let mut records = Vec::new();

    for (index, line) in s.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let value = serde_json::from_str(line).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid JSONL record at line {}: {e}", index + 1),
            )
        })?;
        records.push(value);
    }

    Ok(records)
}

pub fn records_to_jsonl_string<T>(records: &[T]) -> IoResult<String>
where
    T: Serialize,
{
    let mut out = String::new();
    for record in records {
        out.push_str(&serde_json::to_string(record).invalid_data()?);
        out.push('\n');
    }
    Ok(out)
}

pub fn to_jsonl_file<T, P>(path: P, records: &[T]) -> IoResult<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    fs::write(path, records_to_jsonl_string(records)?)
}

pub fn from_jsonl_file<T, P>(path: P) -> IoResult<Vec<T>>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let contents = fs::read_to_string(path)?;
    parse_jsonl_records(&contents)
}

pub fn append_jsonl_record<T, P>(path: P, record: &T) -> IoResult<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(record).invalid_data()?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")
}
