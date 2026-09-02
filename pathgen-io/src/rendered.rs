use std::fmt::{self, Display, Formatter};

use crate::PayloadDigest;

/// Declared rendered/final-form document family.
///
/// Rendered documents such as PDFs are not editable section trees. They are
/// final artifacts with bytes, optional page counts, and extracted metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderedDocumentFormat {
    Pdf,
    Other { label: String },
}

impl RenderedDocumentFormat {
    pub fn label(&self) -> &str {
        match self {
            Self::Pdf => "file::pdf",
            Self::Other { label } => label.as_str(),
        }
    }
}

impl Display for RenderedDocumentFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Evidence/metadata for a rendered document payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderedDocumentEvidence {
    pub format: RenderedDocumentFormat,
    pub bytes_len: usize,
    pub payload_digest: PayloadDigest,
    pub page_count: Option<u32>,
    pub text_extractable: Option<bool>,
    pub metadata_digest: Option<PayloadDigest>,
}

impl RenderedDocumentEvidence {
    pub fn from_bytes(format: RenderedDocumentFormat, payload: &[u8]) -> Self {
        Self {
            format,
            bytes_len: payload.len(),
            payload_digest: PayloadDigest::sha256(payload),
            page_count: None,
            text_extractable: None,
            metadata_digest: None,
        }
    }

    pub fn with_page_count(mut self, page_count: u32) -> Self {
        self.page_count = Some(page_count);
        self
    }

    pub fn with_text_extractable(mut self, text_extractable: bool) -> Self {
        self.text_extractable = Some(text_extractable);
        self
    }

    pub fn with_metadata_payload(mut self, metadata_payload: &[u8]) -> Self {
        self.metadata_digest = Some(PayloadDigest::sha256(metadata_payload));
        self
    }

    pub fn audit_line(&self) -> String {
        format!(
            "rendered_format={} bytes_len={} payload_digest={} pages={} text_extractable={} metadata_digest={}",
            self.format,
            self.bytes_len,
            self.payload_digest,
            self.page_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            self.text_extractable
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            self.metadata_digest
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string()),
        )
    }
}

pub trait RenderedDocument {
    fn rendered_evidence(&self) -> &RenderedDocumentEvidence;

    fn audit_line(&self) -> String {
        self.rendered_evidence().audit_line()
    }
}
