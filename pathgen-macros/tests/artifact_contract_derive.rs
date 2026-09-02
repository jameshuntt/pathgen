use pathgen_core::{ArtifactContract as ArtifactContractTrait, ArtifactEndpoint};
use pathgen_macros::{ArtifactContract, ReceiptContract};

#[derive(ArtifactContract)]
#[artifact(
    schema = "pathgen.test.config.v1",
    route = "test.config",
    kind = "config",
    version = 1,
    canonical(file::toml),
    imports(file::toml, file::json),
    exports(file::toml, file::json, file::txt, sql::sqlite(table = "artifact_index"))
)]
struct TestConfig;

#[derive(ReceiptContract)]
#[receipt(
    schema = "pathgen.test.receipt.v1",
    route = "test.receipt",
    kind = "receipt",
    version = 7,
    canonical = file::json,
    imports(file::json),
    exports(file::json, file::jsonl, chain::target(route = "chain.test.receipt.commit"))
)]
struct TestReceipt;

#[derive(ArtifactContract)]
#[artifact(
    core = pathgen_core,
    schema = "pathgen.test.string_endpoint.v1",
    route = "test.string_endpoint",
    kind = "string_endpoint",
    version = 2,
    canonical = "file::yaml",
    exports(file::yaml)
)]
struct StringEndpointArtifact;


#[derive(ArtifactContract)]
#[artifact(
    schema = "pathgen.test.document_data.v1",
    route = "test.document_data",
    kind = "document_data",
    version = 3,
    canonical(file::docx),
    imports(file::docx, file::pdf),
    exports(file::docx, file::pdf, file::npy, file::npz)
)]
struct DocumentDataArtifact;

#[test]
fn artifact_derive_generates_contract_constants() {
    assert_eq!(TestConfig::SCHEMA, "pathgen.test.config.v1");
    assert_eq!(TestConfig::ROUTE_ID, "test.config");
    assert_eq!(TestConfig::KIND, "config");
    assert_eq!(TestConfig::VERSION, 1);
    assert_eq!(TestConfig::CANONICAL, ArtifactEndpoint::FileToml);
    assert_eq!(TestConfig::IMPORTS, &[ArtifactEndpoint::FileToml, ArtifactEndpoint::FileJson]);
    assert_eq!(
        TestConfig::EXPORTS,
        &[
            ArtifactEndpoint::FileToml,
            ArtifactEndpoint::FileJson,
            ArtifactEndpoint::FileTxt,
            ArtifactEndpoint::SqliteTable { table: "artifact_index" },
        ]
    );
}

#[test]
fn receipt_derive_is_an_artifact_contract_alias() {
    assert_eq!(TestReceipt::SCHEMA, "pathgen.test.receipt.v1");
    assert_eq!(TestReceipt::ROUTE_ID, "test.receipt");
    assert_eq!(TestReceipt::KIND, "receipt");
    assert_eq!(TestReceipt::VERSION, 7);
    assert_eq!(TestReceipt::CANONICAL, ArtifactEndpoint::FileJson);
    assert_eq!(
        TestReceipt::EXPORTS,
        &[
            ArtifactEndpoint::FileJson,
            ArtifactEndpoint::FileJsonl,
            ArtifactEndpoint::ChainTarget { route_id: "chain.test.receipt.commit" },
        ]
    );
}

#[test]
fn string_endpoint_form_is_supported() {
    assert_eq!(StringEndpointArtifact::CANONICAL, ArtifactEndpoint::FileYaml);
    assert_eq!(StringEndpointArtifact::IMPORTS, &[ArtifactEndpoint::FileYaml]);
    assert_eq!(StringEndpointArtifact::EXPORTS, &[ArtifactEndpoint::FileYaml]);
}


#[test]
fn document_data_endpoints_are_supported_by_macros() {
    assert_eq!(DocumentDataArtifact::CANONICAL, ArtifactEndpoint::FileDocx);
    assert_eq!(
        DocumentDataArtifact::IMPORTS,
        &[ArtifactEndpoint::FileDocx, ArtifactEndpoint::FilePdf]
    );
    assert_eq!(
        DocumentDataArtifact::EXPORTS,
        &[
            ArtifactEndpoint::FileDocx,
            ArtifactEndpoint::FilePdf,
            ArtifactEndpoint::FileNpy,
            ArtifactEndpoint::FileNpz,
        ]
    );
}
