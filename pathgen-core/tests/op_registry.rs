use pathgen_core::{
    AppPathRole, ArtifactContract, ArtifactEndpoint, ArtifactOpDescriptor, ArtifactOpDirection,
    ArtifactOpRegistry, ArtifactOpTag, ArtifactOutputDescriptor, PathMutability, PathOwner,
    PathSensitivity, PathgenError,
};

const PROD_JSON: ArtifactOutputDescriptor = ArtifactOutputDescriptor {
    route_id: "receipt.package.signing.production",
    schema: "example.package.production_signing_receipt.v1",
    kind: "production_signing",
    version: 1,
    endpoint: ArtifactEndpoint::FileJson,
    base_role: AppPathRole::State,
    relative_template: "receipts/package/signing/production/{receipt_id}.json",
    sensitivity: PathSensitivity::Confidential,
    mutability: PathMutability::AppendOnly,
    owner: PathOwner::ServiceUser,
};

const PROD_TXT: ArtifactOutputDescriptor = ArtifactOutputDescriptor {
    endpoint: ArtifactEndpoint::FileTxt,
    relative_template: "receipts/package/signing/production/{receipt_id}.txt",
    ..PROD_JSON
};

const PROD_YAML: ArtifactOutputDescriptor = ArtifactOutputDescriptor {
    endpoint: ArtifactEndpoint::FileYaml,
    relative_template: "receipts/package/signing/production/{receipt_id}.yaml",
    ..PROD_JSON
};

struct ProductionReceipt;

impl ArtifactContract for ProductionReceipt {
    const SCHEMA: &'static str = "example.package.production_signing_receipt.v1";
    const ROUTE_ID: &'static str = "receipt.package.signing.production";
    const KIND: &'static str = "production_signing";
    const VERSION: u32 = 1;
    const CANONICAL: ArtifactEndpoint = ArtifactEndpoint::FileJson;
    const IMPORTS: &'static [ArtifactEndpoint] = &[ArtifactEndpoint::FileJson];
    const EXPORTS: &'static [ArtifactEndpoint] = &[ArtifactEndpoint::FileJson, ArtifactEndpoint::FileTxt];
}

static REGISTRY_DESCRIPTORS: [ArtifactOpDescriptor; 2] = [
    ArtifactOpDescriptor::new(PROD_JSON),
    ArtifactOpDescriptor::new(PROD_TXT),
];
static REGISTRY: ArtifactOpRegistry = ArtifactOpRegistry::new(&REGISTRY_DESCRIPTORS);

#[test]
fn registry_compiles_declared_export_handle() {
    REGISTRY.validate_unique().unwrap();

    let handle = REGISTRY
        .compile_export::<ProductionReceipt>(ArtifactEndpoint::FileJson)
        .unwrap();

    assert_eq!(handle.tag(), ArtifactOpTag::FileJson);
    assert_eq!(handle.endpoint(), ArtifactEndpoint::FileJson);
    assert_eq!(
        handle.descriptor().output.route_id,
        "receipt.package.signing.production"
    );
}

#[test]
fn registry_compiles_canonical_export_handle() {
    let handle = REGISTRY
        .compile_canonical_export::<ProductionReceipt>()
        .unwrap();

    assert_eq!(handle.endpoint(), ProductionReceipt::CANONICAL);
    assert_eq!(handle.word() & ArtifactOpTag::MASK, ArtifactOpTag::FileJson.as_usize());
}

#[test]
fn registry_compile_direction_selects_import_or_export() {
    let import = REGISTRY
        .compile::<ProductionReceipt>(ArtifactOpDirection::Import, ArtifactEndpoint::FileJson)
        .unwrap();
    let export = REGISTRY
        .compile::<ProductionReceipt>(ArtifactOpDirection::Export, ArtifactEndpoint::FileTxt)
        .unwrap();

    assert_eq!(import.endpoint(), ArtifactEndpoint::FileJson);
    assert_eq!(export.endpoint(), ArtifactEndpoint::FileTxt);
}

#[test]
fn registry_rejects_endpoint_not_declared_by_contract_before_lookup() {
    let err = REGISTRY
        .compile_export::<ProductionReceipt>(ArtifactEndpoint::FileYaml)
        .unwrap_err();

    assert!(matches!(err, PathgenError::ArtifactEndpointNotDeclared { .. }));
}

#[test]
fn registry_rejects_declared_but_unregistered_endpoint() {
    static JSON_ONLY_DESCRIPTORS: [ArtifactOpDescriptor; 1] = [ArtifactOpDescriptor::new(PROD_JSON)];
    static JSON_ONLY: ArtifactOpRegistry = ArtifactOpRegistry::new(&JSON_ONLY_DESCRIPTORS);

    let err = JSON_ONLY
        .compile_export::<ProductionReceipt>(ArtifactEndpoint::FileTxt)
        .unwrap_err();

    assert!(matches!(err, PathgenError::ArtifactOpDescriptorNotRegistered { .. }));
}

#[test]
fn registry_rejects_duplicate_route_endpoint_bindings() {
    static DUPLICATE_DESCRIPTORS: [ArtifactOpDescriptor; 2] = [
        ArtifactOpDescriptor::new(PROD_JSON),
        ArtifactOpDescriptor::new(PROD_JSON),
    ];
    static DUPLICATE: ArtifactOpRegistry = ArtifactOpRegistry::new(&DUPLICATE_DESCRIPTORS);

    let err = DUPLICATE.validate_unique().unwrap_err();
    assert!(matches!(err, PathgenError::DuplicateArtifactOpDescriptor { .. }));

    let err = DUPLICATE
        .find("receipt.package.signing.production", ArtifactEndpoint::FileJson)
        .unwrap_err();
    assert!(matches!(err, PathgenError::DuplicateArtifactOpDescriptor { .. }));
}

#[test]
fn registry_finds_registered_descriptor_by_route_and_endpoint() {
    let descriptor = REGISTRY
        .find("receipt.package.signing.production", ArtifactEndpoint::FileTxt)
        .unwrap();

    assert_eq!(descriptor.output.endpoint, ArtifactEndpoint::FileTxt);
    assert!(REGISTRY.contains("receipt.package.signing.production", ArtifactEndpoint::FileJson));
    assert!(!REGISTRY.contains("receipt.package.signing.production", ArtifactEndpoint::FileYaml));
}

#[test]
fn registry_still_rejects_wrong_descriptor_contract_fields() {
    const WRONG_SCHEMA: ArtifactOutputDescriptor = ArtifactOutputDescriptor {
        schema: "wrong.schema.v1",
        ..PROD_YAML
    };

    static WRONG_DESCRIPTORS: [ArtifactOpDescriptor; 1] = [ArtifactOpDescriptor::new(WRONG_SCHEMA)];
    static WRONG_REGISTRY: ArtifactOpRegistry = ArtifactOpRegistry::new(&WRONG_DESCRIPTORS);

    struct YamlExportReceipt;

    impl ArtifactContract for YamlExportReceipt {
        const SCHEMA: &'static str = "example.package.production_signing_receipt.v1";
        const ROUTE_ID: &'static str = "receipt.package.signing.production";
        const KIND: &'static str = "production_signing";
        const VERSION: u32 = 1;
        const CANONICAL: ArtifactEndpoint = ArtifactEndpoint::FileYaml;
        const IMPORTS: &'static [ArtifactEndpoint] = &[ArtifactEndpoint::FileYaml];
        const EXPORTS: &'static [ArtifactEndpoint] = &[ArtifactEndpoint::FileYaml];
    }

    let err = WRONG_REGISTRY
        .compile_export::<YamlExportReceipt>(ArtifactEndpoint::FileYaml)
        .unwrap_err();

    assert!(matches!(err, PathgenError::ArtifactContractMismatch { field: "schema", .. }));
}
