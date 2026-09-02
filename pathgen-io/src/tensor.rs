use std::fmt::{self, Display, Formatter};

use crate::PayloadDigest;

/// Declared tensor/data artifact family.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TensorArtifactFormat {
    Npy,
    Npz,
    Other { label: String },
}

impl TensorArtifactFormat {
    pub fn label(&self) -> &str {
        match self {
            Self::Npy => "file::npy",
            Self::Npz => "file::npz",
            Self::Other { label } => label.as_str(),
        }
    }
}

impl Display for TensorArtifactFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Minimal dtype descriptor for tensor artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TensorDType {
    F32,
    F64,
    I32,
    I64,
    U8,
    Bool,
    Other { label: String },
}

impl TensorDType {
    pub fn label(&self) -> &str {
        match self {
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::Bool => "bool",
            Self::Other { label } => label.as_str(),
        }
    }
}

impl Display for TensorDType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Shape metadata for a tensor artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TensorShape {
    pub dims: Vec<usize>,
}

impl TensorShape {
    pub fn new(dims: impl Into<Vec<usize>>) -> Self {
        Self { dims: dims.into() }
    }

    pub fn rank(&self) -> usize {
        self.dims.len()
    }

    pub fn element_count(&self) -> Option<usize> {
        self.dims
            .iter()
            .try_fold(1usize, |acc, dim| acc.checked_mul(*dim))
    }

    pub fn label(&self) -> String {
        let dims = self
            .dims
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("x");
        format!("[{}]", dims)
    }
}

impl Display for TensorShape {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

/// Metadata/evidence for one tensor array.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TensorArtifactMetadata {
    pub format: TensorArtifactFormat,
    pub name: Option<String>,
    pub dtype: TensorDType,
    pub shape: TensorShape,
    pub fortran_order: bool,
    pub bytes_len: usize,
    pub payload_digest: PayloadDigest,
}

impl TensorArtifactMetadata {
    pub fn from_payload(
        format: TensorArtifactFormat,
        name: Option<impl Into<String>>,
        dtype: TensorDType,
        shape: TensorShape,
        fortran_order: bool,
        payload: &[u8],
    ) -> Self {
        Self {
            format,
            name: name.map(|value| value.into()),
            dtype,
            shape,
            fortran_order,
            bytes_len: payload.len(),
            payload_digest: PayloadDigest::sha256(payload),
        }
    }

    pub fn audit_line(&self) -> String {
        format!(
            "tensor_format={} name={} dtype={} shape={} rank={} elements={} fortran_order={} bytes_len={} payload_digest={}",
            self.format,
            self.name.as_deref().unwrap_or("none"),
            self.dtype,
            self.shape,
            self.shape.rank(),
            self.shape
                .element_count()
                .map(|value| value.to_string())
                .unwrap_or_else(|| "overflow".to_string()),
            self.fortran_order,
            self.bytes_len,
            self.payload_digest,
        )
    }
}

pub trait TensorArtifact {
    fn tensor_metadata(&self) -> &TensorArtifactMetadata;

    fn audit_line(&self) -> String {
        self.tensor_metadata().audit_line()
    }
}

/// Manifest for multi-array tensor archives such as NPZ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorArchiveManifest {
    pub format: TensorArtifactFormat,
    pub arrays: Vec<TensorArtifactMetadata>,
}

impl TensorArchiveManifest {
    pub fn new(format: TensorArtifactFormat) -> Self {
        Self {
            format,
            arrays: Vec::new(),
        }
    }

    pub fn with_array(mut self, metadata: TensorArtifactMetadata) -> Self {
        self.arrays.push(metadata);
        self
    }

    pub fn get_array(&self, name: &str) -> Option<&TensorArtifactMetadata> {
        self.arrays
            .iter()
            .find(|array| array.name.as_deref() == Some(name))
    }

    pub fn audit_line(&self) -> String {
        format!("tensor_archive_format={} arrays={}", self.format, self.arrays.len())
    }
}
