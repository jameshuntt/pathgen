//! pathgen-core
//! ============
//!
//! Dependency-light route/path/artifact declaration core.
//!
//! This crate owns semantic path policy, artifact endpoint descriptors, and
//! tagged operation handles. It intentionally does not import JSON/YAML/TOML,
//! SQLite clients or filesystem sink adapters.

pub mod artifact;
pub mod endpoint;
pub mod env;
pub mod error;
pub mod fs_ops;
pub mod op_handle;
pub mod policy;
pub mod registry;
pub mod receipt;
pub mod role;
pub mod safety;
pub mod standard_path;

pub use artifact::{
    ArtifactContract, ArtifactOutputDescriptor, PathTemplateVars, ResolvedArtifactOutput,
};
pub use endpoint::{ArtifactEndpoint, ArtifactOpTag};
pub use env::Env;
pub use error::PathgenError;
pub use op_handle::{ArtifactOpDescriptor, ArtifactOpHandle};
pub use registry::{ArtifactOpDirection, ArtifactOpRegistry};
pub use policy::{PathPolicy, POLICY_VERSION};
pub use receipt::{PathMutability, PathOwner, PathSensitivity, ResolvedPath};
pub use role::AppPathRole;
pub use safety::{checked_join, validate_app_name};
pub use standard_path::StandardPath;
