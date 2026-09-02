use crate::{RenderedDocument, RenderedDocumentEvidence, RenderedDocumentFormat};

/// Evidence-only PDF rendered document adapter.
///
/// This is not a PDF parser. It records the final rendered bytes and optional
/// page/text metadata so PDF artifacts have honest rendered-document semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfRenderedDocument {
    evidence: RenderedDocumentEvidence,
}

impl PdfRenderedDocument {
    pub fn from_bytes(payload: &[u8]) -> Self {
        Self {
            evidence: RenderedDocumentEvidence::from_bytes(RenderedDocumentFormat::Pdf, payload),
        }
    }

    pub fn from_bytes_with_pages(payload: &[u8], page_count: u32) -> Self {
        Self {
            evidence: RenderedDocumentEvidence::from_bytes(RenderedDocumentFormat::Pdf, payload)
                .with_page_count(page_count),
        }
    }

    pub fn with_text_extractable(mut self, text_extractable: bool) -> Self {
        self.evidence.text_extractable = Some(text_extractable);
        self
    }

    pub fn evidence(&self) -> &RenderedDocumentEvidence {
        &self.evidence
    }
}

impl RenderedDocument for PdfRenderedDocument {
    fn rendered_evidence(&self) -> &RenderedDocumentEvidence {
        &self.evidence
    }
}
