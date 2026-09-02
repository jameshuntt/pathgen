#[cfg(feature = "file-docx")]
#[test]
fn docx_package_manifest_tracks_named_parts() {
    use pathgen_io::{PackageDocument, PayloadDigest};

    let doc = pathgen_io::file::docx::DocxPackageDocument::minimal_main_document(b"<w:document />");

    assert_eq!(doc.package_format().label(), "file::docx");
    assert!(doc.has_part("word/document.xml"));

    let main = doc.main_document_part().unwrap();
    assert_eq!(main.bytes_len, Some(b"<w:document />".len()));
    assert_eq!(main.payload_digest, Some(PayloadDigest::sha256(b"<w:document />")));
    assert!(doc.audit_line().contains("package_format=file::docx"));
}

#[cfg(feature = "file-pdf")]
#[test]
fn pdf_rendered_document_records_final_form_evidence() {
    use pathgen_io::RenderedDocument;

    let pdf = pathgen_io::file::pdf::PdfRenderedDocument::from_bytes_with_pages(b"%PDF-1.7", 2)
        .with_text_extractable(false);

    let evidence = pdf.rendered_evidence();
    assert_eq!(evidence.format.label(), "file::pdf");
    assert_eq!(evidence.bytes_len, b"%PDF-1.7".len());
    assert_eq!(evidence.page_count, Some(2));
    assert_eq!(evidence.text_extractable, Some(false));
    assert!(pdf.audit_line().contains("rendered_format=file::pdf"));
}

#[cfg(feature = "file-npy")]
#[test]
fn npy_tensor_artifact_records_dtype_shape_and_digest() {
    use pathgen_io::{TensorArtifact, TensorDType, TensorShape};

    let tensor = pathgen_io::file::npy::NpyTensorArtifact::from_payload(
        TensorDType::F32,
        TensorShape::new(vec![2, 3]),
        false,
        b"npy-bytes",
    );

    let metadata = tensor.tensor_metadata();
    assert_eq!(metadata.format.label(), "file::npy");
    assert_eq!(metadata.dtype.label(), "f32");
    assert_eq!(metadata.shape.rank(), 2);
    assert_eq!(metadata.shape.element_count(), Some(6));
    assert!(tensor.audit_line().contains("shape=[2x3]"));
}

#[cfg(all(feature = "file-npy", feature = "file-npz"))]
#[test]
fn npz_tensor_archive_tracks_named_arrays() {
    use pathgen_io::{TensorArtifactFormat, TensorArtifactMetadata, TensorDType, TensorShape};

    let weights = TensorArtifactMetadata::from_payload(
        TensorArtifactFormat::Npz,
        Some("weights"),
        TensorDType::F64,
        TensorShape::new(vec![4, 4]),
        false,
        b"weights-bytes",
    );
    let bias = TensorArtifactMetadata::from_payload(
        TensorArtifactFormat::Npz,
        Some("bias"),
        TensorDType::F64,
        TensorShape::new(vec![4]),
        false,
        b"bias-bytes",
    );

    let archive = pathgen_io::file::npz::NpzTensorArchive::new(vec![weights, bias]);

    assert_eq!(archive.manifest().format.label(), "file::npz");
    assert_eq!(archive.manifest().arrays.len(), 2);
    assert_eq!(archive.get_array("weights").unwrap().shape.element_count(), Some(16));
    assert!(archive.audit_line().contains("arrays=2"));
}
