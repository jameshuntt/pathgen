use crate::{TensorArchiveManifest, TensorArtifactFormat, TensorArtifactMetadata};

/// Manifest-only NPZ tensor archive adapter.
///
/// This keeps NPZ honest as a multi-array archive, not a single structured doc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpzTensorArchive {
    manifest: TensorArchiveManifest,
}

impl NpzTensorArchive {
    pub fn new(arrays: Vec<TensorArtifactMetadata>) -> Self {
        Self {
            manifest: TensorArchiveManifest {
                format: TensorArtifactFormat::Npz,
                arrays,
            },
        }
    }

    pub fn manifest(&self) -> &TensorArchiveManifest {
        &self.manifest
    }

    pub fn get_array(&self, name: &str) -> Option<&TensorArtifactMetadata> {
        self.manifest.get_array(name)
    }

    pub fn audit_line(&self) -> String {
        self.manifest.audit_line()
    }
}
