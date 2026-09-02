//! pathgen facade crate.
//!
//! By default this re-exports the dependency-light `pathgen-core` API.
//! Enable features such as `file-json`, `file-toml`, `file-yaml`, `file-txt`,
//! or `sql-sqlite` to pull in `pathgen-io` sinks through this facade, and
//! `derive` for the `ArtifactContract` / `ReceiptContract` derive macros.

pub use pathgen_core::*;

#[cfg(feature = "io")]
pub use pathgen_io::*;

#[cfg(feature = "macros")]
pub use pathgen_macros::{ArtifactContract, ReceiptContract};

#[cfg(feature = "workspace")]
pub mod workspace;
