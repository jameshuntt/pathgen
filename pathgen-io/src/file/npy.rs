use crate::{TensorArtifact, TensorArtifactFormat, TensorArtifactMetadata, TensorDType, TensorShape};

/// Evidence-only NPY tensor artifact adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpyTensorArtifact {
    metadata: TensorArtifactMetadata,
}

impl NpyTensorArtifact {
    pub fn from_payload(
        dtype: TensorDType,
        shape: TensorShape,
        fortran_order: bool,
        payload: &[u8],
    ) -> Self {
        Self {
            metadata: TensorArtifactMetadata::from_payload(
                TensorArtifactFormat::Npy,
                None::<String>,
                dtype,
                shape,
                fortran_order,
                payload,
            ),
        }
    }

    pub fn metadata(&self) -> &TensorArtifactMetadata {
        &self.metadata
    }
}

impl TensorArtifact for NpyTensorArtifact {
    fn tensor_metadata(&self) -> &TensorArtifactMetadata {
        &self.metadata
    }
}
