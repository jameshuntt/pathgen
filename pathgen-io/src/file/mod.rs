//! File codecs/sinks.
//!
//! The Rust module names use `file::toml` / `file::json` style, while Cargo
//! features use hyphenated names such as `file-toml` and `file-json`.

#[cfg(feature = "file-txt")]
pub mod txt;

#[cfg(feature = "file-json")]
pub mod json;

#[cfg(feature = "file-toml")]
pub mod toml;

#[cfg(feature = "file-yaml")]
pub mod yaml;

#[cfg(feature = "file-jsonl")]
pub mod jsonl;

#[cfg(feature = "file-xml")]
pub mod xml;

#[cfg(feature = "file-markdown")]
pub mod markdown;

#[cfg(feature = "file-cbor")]
pub mod cbor;

#[cfg(feature = "file-docx")]
pub mod docx;

#[cfg(feature = "file-pdf")]
pub mod pdf;

#[cfg(feature = "file-npy")]
pub mod npy;

#[cfg(feature = "file-npz")]
pub mod npz;
