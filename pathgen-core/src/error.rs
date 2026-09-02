use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    path::PathBuf,
};

/// Error type used across path policy resolution, validation, and filesystem helpers.
#[derive(Debug)]
pub enum PathgenError {
    EmptyAppName,

    InvalidAppName { input: String, reason: &'static str },

    UnsafeSubpath { input: String, reason: &'static str },

    UnknownStandardPath { input: String },

    UnknownAppPathRole { input: String },

    MissingParent { path: PathBuf },

    UnsupportedArtifactEndpoint { endpoint: &'static str, reason: &'static str },

    InvalidArtifactTemplate { template: String, reason: &'static str },

    MissingArtifactTemplateVar { template: String, key: String },

    ArtifactContractMismatch {
        field: &'static str,
        expected: &'static str,
        actual: &'static str,
    },

    ArtifactVersionMismatch { expected: u32, actual: u32 },

    ArtifactEndpointNotDeclared {
        endpoint: &'static str,
        direction: &'static str,
    },

    ArtifactOpDescriptorNotRegistered {
        route_id: String,
        endpoint: &'static str,
    },

    DuplicateArtifactOpDescriptor {
        route_id: &'static str,
        endpoint: &'static str,
    },

    ArtifactOutputPlanMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },

    InvalidArtifactFileWritePolicy { field: &'static str, reason: &'static str },

    /// A value that a policy in some domain refuses; `domain` names the policy
    /// in words (`"adapter boundary"`, `"chain target metadata"`), `field` the
    /// value, `reason` why.
    Invalid {
        domain: &'static str,
        field: &'static str,
        reason: &'static str,
    },

    /// [`Invalid`](Self::Invalid) with a reason built at runtime.
    InvalidDetail {
        domain: &'static str,
        field: &'static str,
        reason: String,
    },

    /// Two values that had to agree in some domain and did not.
    Mismatch {
        domain: &'static str,
        field: &'static str,
        expected: String,
        actual: String,
    },

    MisalignedArtifactOpDescriptor { addr: usize, align: usize },

    Io(io::Error),
}

impl Display for PathgenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PathgenError::EmptyAppName => {
                write!(f, "app name cannot be empty")
            }
            PathgenError::InvalidAppName { input, reason } => {
                write!(f, "invalid app name `{}`: {}", input, reason)
            }
            PathgenError::UnsafeSubpath { input, reason } => {
                write!(f, "unsafe subpath `{}`: {}", input, reason)
            }
            PathgenError::UnknownStandardPath { input } => {
                write!(f, "unknown standard path: {}", input)
            }
            PathgenError::UnknownAppPathRole { input } => {
                write!(f, "unknown app path role: {}", input)
            }
            PathgenError::MissingParent { path } => {
                write!(f, "path has no parent directory: {}", path.display())
            }
            PathgenError::UnsupportedArtifactEndpoint { endpoint, reason } => {
                write!(f, "unsupported artifact endpoint `{}`: {}", endpoint, reason)
            }
            PathgenError::InvalidArtifactTemplate { template, reason } => {
                write!(f, "invalid artifact template `{}`: {}", template, reason)
            }
            PathgenError::MissingArtifactTemplateVar { template, key } => {
                write!(f, "artifact template `{}` is missing variable `{}`", template, key)
            }
            PathgenError::ArtifactContractMismatch { field, expected, actual } => {
                write!(f, "artifact contract mismatch for {}: expected `{}`, got `{}`", field, expected, actual)
            }
            PathgenError::ArtifactVersionMismatch { expected, actual } => {
                write!(f, "artifact version mismatch: expected `{}`, got `{}`", expected, actual)
            }
            PathgenError::ArtifactEndpointNotDeclared { endpoint, direction } => {
                write!(f, "artifact endpoint `{}` is not declared for {}", endpoint, direction)
            }
            PathgenError::ArtifactOpDescriptorNotRegistered { route_id, endpoint } => {
                write!(f, "artifact operation descriptor is not registered for route `{}` endpoint `{}`", route_id, endpoint)
            }
            PathgenError::DuplicateArtifactOpDescriptor { route_id, endpoint } => {
                write!(f, "duplicate artifact operation descriptor for route `{}` endpoint `{}`", route_id, endpoint)
            }
            PathgenError::ArtifactOutputPlanMismatch { field, expected, actual } => {
                write!(f, "artifact output plan mismatch for {}: expected `{}`, got `{}`", field, expected, actual)
            }
            PathgenError::InvalidArtifactFileWritePolicy { field, reason } => {
                write!(f, "invalid artifact file write policy for {}: {}", field, reason)
            }
            PathgenError::Invalid { domain, field, reason } => {
                write!(f, "invalid {} for {}: {}", domain, field, reason)
            }
            PathgenError::InvalidDetail { domain, field, reason } => {
                write!(f, "invalid {} for {}: {}", domain, field, reason)
            }
            PathgenError::Mismatch { domain, field, expected, actual } => {
                write!(f, "{} mismatch for {}: expected `{}`, got `{}`", domain, field, expected, actual)
            }
            PathgenError::MisalignedArtifactOpDescriptor { addr, align } => {
                write!(f, "artifact operation descriptor address {addr:#x} is not aligned to {align} bytes")
            }
            PathgenError::Io(err) => {
                write!(f, "io error: {}", err)
            }
        }
    }
}

impl Error for PathgenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PathgenError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for PathgenError {
    fn from(value: io::Error) -> Self {
        PathgenError::Io(value)
    }
}
