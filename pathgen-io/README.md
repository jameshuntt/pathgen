# pathgen-io

Feature-backed sinks for [`pathgen-core`](https://crates.io/crates/pathgen-core)
artifact routes: file outputs with write policies and receipts, structured
documents (JSON, TOML, YAML, XML, CBOR), record streams (JSONL), sectioned
text and Markdown, rendered documents, tensor archives, and an SQLite sink.
Every codec is behind a Cargo feature (`file-json`, `file-toml`, `sql-sqlite`,
…); with no features enabled the crate only brings `sha2`.

Most users want the `pathgen` facade crate, which re-exports this one behind
the same feature names.
