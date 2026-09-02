use crate::{PackageDocument, PackageDocumentFormat, PackageDocumentManifest, PackagePart};

/// Manifest-only DOCX package document.
///
/// This adapter intentionally does not unzip or rewrite DOCX files yet. It
/// models the parts and digests a real DOCX adapter must preserve later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocxPackageDocument {
    manifest: PackageDocumentManifest,
}

impl DocxPackageDocument {
    pub fn new(parts: Vec<PackagePart>) -> Self {
        Self {
            manifest: PackageDocumentManifest {
                format: PackageDocumentFormat::Docx,
                parts,
            },
        }
    }

    pub fn minimal_main_document(document_xml: &[u8]) -> Self {
        Self::new(vec![
            PackagePart::new("[Content_Types].xml")
                .with_content_type("application/xml"),
            PackagePart::new("_rels/.rels")
                .with_content_type("application/vnd.openxmlformats-package.relationships+xml"),
            PackagePart::new("word/document.xml")
                .with_content_type("application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml")
                .with_payload(document_xml),
        ])
    }

    pub fn main_document_part(&self) -> Option<&PackagePart> {
        self.manifest.get_part("word/document.xml")
    }

    pub fn into_manifest(self) -> PackageDocumentManifest {
        self.manifest
    }
}

impl PackageDocument for DocxPackageDocument {
    fn package_format(&self) -> &PackageDocumentFormat {
        self.manifest.package_format()
    }

    fn parts(&self) -> &[PackagePart] {
        self.manifest.parts()
    }
}
