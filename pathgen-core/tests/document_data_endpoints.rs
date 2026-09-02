use pathgen_core::{ArtifactEndpoint, ArtifactOpTag};

#[test]
fn document_and_data_file_endpoints_have_labels_extensions_and_extended_tags() {
    let endpoints = [
        (ArtifactEndpoint::FileDocx, "file::docx", Some("docx")),
        (ArtifactEndpoint::FilePdf, "file::pdf", Some("pdf")),
        (ArtifactEndpoint::FileNpy, "file::npy", Some("npy")),
        (ArtifactEndpoint::FileNpz, "file::npz", Some("npz")),
    ];

    for (endpoint, label, extension) in endpoints {
        assert!(endpoint.is_file());
        assert_eq!(endpoint.label(), label);
        assert_eq!(endpoint.extension(), extension);
        assert_eq!(ArtifactOpTag::from_endpoint(endpoint), ArtifactOpTag::ExtendedFile);
    }
}
