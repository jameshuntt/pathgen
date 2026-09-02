#![cfg(feature = "macros")]

use pathgen::{ArtifactContract as ArtifactContractTrait, ArtifactEndpoint};

#[derive(pathgen::ArtifactContract)]
#[artifact(
    core = pathgen,
    schema = "pathgen.facade.artifact.v1",
    route = "facade.artifact",
    kind = "facade_artifact",
    version = 3,
    canonical(file::json),
    exports(file::json, file::txt)
)]
struct FacadeArtifact;

#[test]
fn facade_reexports_artifact_contract_derive_when_macros_feature_is_enabled() {
    assert_eq!(FacadeArtifact::SCHEMA, "pathgen.facade.artifact.v1");
    assert_eq!(FacadeArtifact::ROUTE_ID, "facade.artifact");
    assert_eq!(FacadeArtifact::CANONICAL, ArtifactEndpoint::FileJson);
    assert_eq!(FacadeArtifact::EXPORTS, &[ArtifactEndpoint::FileJson, ArtifactEndpoint::FileTxt]);
}
