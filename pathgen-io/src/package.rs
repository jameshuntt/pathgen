use std::fmt::{self, Display, Formatter};

use crate::PayloadDigest;

/// Declared package/container document family.
///
/// Package documents such as DOCX are zip-like containers with named parts.
/// This trait intentionally models the container manifest, not text sections or
/// a generic tree. Real zip/docx parsing can be added later behind the same
/// surface without changing route/receipt evidence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackageDocumentFormat {
    Docx,
    Other { label: String },
}

impl PackageDocumentFormat {
    pub fn label(&self) -> &str {
        match self {
            Self::Docx => "file::docx",
            Self::Other { label } => label.as_str(),
        }
    }
}

impl Display for PackageDocumentFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Manifest entry for a single named package part.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackagePart {
    pub path: String,
    pub content_type: Option<String>,
    pub bytes_len: Option<usize>,
    pub payload_digest: Option<PayloadDigest>,
}

impl PackagePart {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content_type: None,
            bytes_len: None,
            payload_digest: None,
        }
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    pub fn with_payload(mut self, payload: &[u8]) -> Self {
        self.bytes_len = Some(payload.len());
        self.payload_digest = Some(PayloadDigest::sha256(payload));
        self
    }

    pub fn audit_line(&self) -> String {
        format!(
            "part={} content_type={} bytes_len={} payload_digest={}",
            self.path,
            self.content_type.as_deref().unwrap_or("none"),
            self.bytes_len
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            self.payload_digest
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string()),
        )
    }
}

/// Common behavior for package/container documents.
pub trait PackageDocument {
    fn package_format(&self) -> &PackageDocumentFormat;
    fn parts(&self) -> &[PackagePart];

    fn get_part(&self, path: &str) -> Option<&PackagePart> {
        self.parts().iter().find(|part| part.path == path)
    }

    fn has_part(&self, path: &str) -> bool {
        self.get_part(path).is_some()
    }

    fn audit_line(&self) -> String {
        format!(
            "package_format={} parts={}",
            self.package_format(),
            self.parts().len(),
        )
    }
}

/// Generic manifest-only package document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDocumentManifest {
    pub format: PackageDocumentFormat,
    pub parts: Vec<PackagePart>,
}

impl PackageDocumentManifest {
    pub fn new(format: PackageDocumentFormat) -> Self {
        Self {
            format,
            parts: Vec::new(),
        }
    }

    pub fn with_part(mut self, part: PackagePart) -> Self {
        self.parts.push(part);
        self
    }

    pub fn push_part(&mut self, part: PackagePart) {
        self.parts.push(part);
    }
}

impl PackageDocument for PackageDocumentManifest {
    fn package_format(&self) -> &PackageDocumentFormat {
        &self.format
    }

    fn parts(&self) -> &[PackagePart] {
        &self.parts
    }
}
