//! pathgen-io
//! ==========
//!
//! Feature-backed sinks for pathgen-core.
//!
//! `pathgen-core` declares artifact routes/endpoints and resolves safe paths.
//! This crate performs the side effects: filesystem helpers, file sinks, and
//! optional JSON/TOML/YAML/XML/CBOR/SQLite codecs behind Cargo features.

pub mod file;
pub mod file_output;
pub mod fs_ext;
pub mod output_plan;
pub mod package;
pub mod record_stream;
pub mod rendered;
pub mod result_ext;
pub mod sectioned;
pub mod structured;
pub mod tensor;

#[cfg(feature = "sql-sqlite")]
pub mod sql;

pub use file_output::{
    ArtifactFileConflictPolicy, ArtifactFileLockPolicy, ArtifactFileWritePolicy,
    ArtifactFileWriteStrategy, ArtifactWriteClock, ArtifactWriteReceipt, ArtifactWriteStatus,
    BuildableFileOutput, FixedArtifactWriteClock, SystemArtifactWriteClock,
};
pub use fs_ext::PathPolicyFsExt;
pub use output_plan::{ArtifactOutputPlan, PayloadDigest, ResolvedArtifactTarget};
pub use package::{PackageDocument, PackageDocumentFormat, PackageDocumentManifest, PackagePart};
pub use record_stream::RecordStreamDocument;
pub use rendered::{RenderedDocument, RenderedDocumentEvidence, RenderedDocumentFormat};
pub use result_ext::ResultExt;
pub use sectioned::SectionedDocument;
pub use structured::{MergePolicy, StructuredDocument};
pub use tensor::{TensorArtifact, TensorArtifactFormat, TensorArtifactMetadata, TensorArchiveManifest, TensorDType, TensorShape};
