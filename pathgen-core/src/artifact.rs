use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use crate::{
    checked_join, AppPathRole, ArtifactEndpoint, Env, PathMutability, PathOwner, PathPolicy,
    PathSensitivity, PathgenError, POLICY_VERSION,
};

/// Durable semantic contract for a receipt/config/manifest/status artifact.
///
/// This is the declaration layer. It does not import JSON/YAML/TOML/SQLite
/// crates and it does not perform I/O. Backends/codecs can be feature-backed
/// adapters above this contract.
pub trait ArtifactContract {
    const SCHEMA: &'static str;
    const ROUTE_ID: &'static str;
    const KIND: &'static str;
    const VERSION: u32;

    const CANONICAL: ArtifactEndpoint;
    const IMPORTS: &'static [ArtifactEndpoint];
    const EXPORTS: &'static [ArtifactEndpoint];
}

/// Output descriptor registered with pathgen/routegen.
///
/// It says: for this semantic route + endpoint, which base role and relative
/// template may be used? The template is resolved through [`PathPolicy`], so it
/// inherits the same root remapping and safe-join rules as ordinary app paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArtifactOutputDescriptor {
    pub route_id: &'static str,
    pub schema: &'static str,
    pub kind: &'static str,
    pub version: u32,
    pub endpoint: ArtifactEndpoint,
    pub base_role: AppPathRole,
    pub relative_template: &'static str,
    pub sensitivity: PathSensitivity,
    pub mutability: PathMutability,
    pub owner: PathOwner,
}

impl ArtifactOutputDescriptor {
    pub const fn endpoint_tag(self) -> crate::ArtifactOpTag {
        crate::ArtifactOpTag::from_endpoint(self.endpoint)
    }

    pub const fn is_file_output(self) -> bool {
        self.endpoint.is_file()
    }
}

/// Key/value variables used to expand a descriptor template such as
/// `receipts/package/signing/{receipt_id}.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PathTemplateVars<'a> {
    pairs: Vec<(&'a str, &'a str)>,
}

impl<'a> PathTemplateVars<'a> {
    pub fn new() -> Self {
        Self { pairs: Vec::new() }
    }

    pub fn one(key: &'a str, value: &'a str) -> Self {
        Self::new().with(key, value)
    }

    pub fn with(mut self, key: &'a str, value: &'a str) -> Self {
        self.pairs.push((key, value));
        self
    }

    pub fn get(&self, key: &str) -> Option<&'a str> {
        self.pairs
            .iter()
            .rev()
            .find_map(|(candidate, value)| (*candidate == key).then_some(*value))
    }
}

/// Evidence object returned when an artifact output descriptor is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedArtifactOutput {
    pub app_name: String,
    pub env: Env,
    pub route_id: String,
    pub schema: String,
    pub kind: String,
    pub version: u32,
    pub endpoint: ArtifactEndpoint,
    pub role: AppPathRole,
    pub relative_path: String,
    pub path: PathBuf,
    pub sensitivity: PathSensitivity,
    pub mutability: PathMutability,
    pub owner: PathOwner,
    pub policy_version: String,
}

impl ResolvedArtifactOutput {
    pub fn audit_line(&self) -> String {
        format!(
            "policy={} app={} env={} route={} schema={} kind={} version={} endpoint={} role={} path={} sensitivity={} mutability={} owner={}",
            self.policy_version,
            self.app_name,
            self.env,
            self.route_id,
            self.schema,
            self.kind,
            self.version,
            self.endpoint,
            self.role,
            self.path.display(),
            self.sensitivity,
            self.mutability,
            self.owner,
        )
    }
}

impl PathPolicy {
    /// Resolve a declared artifact file output into a concrete path/evidence
    /// object. Non-file endpoints are declarations only at this layer and must
    /// be handled by their own adapters.
    pub fn artifact_output(
        &self,
        descriptor: &ArtifactOutputDescriptor,
        vars: &PathTemplateVars<'_>,
    ) -> Result<ResolvedArtifactOutput, PathgenError> {
        if !descriptor.endpoint.is_file() {
            return Err(PathgenError::UnsupportedArtifactEndpoint {
                endpoint: descriptor.endpoint.label(),
                reason: "PathPolicy::artifact_output resolves only file::* endpoints",
            });
        }

        let relative_path = expand_template(descriptor.relative_template, vars)?;
        let base = self.resolve_dir_path(descriptor.base_role)?;
        let path = checked_join(&base, &relative_path)?;

        Ok(ResolvedArtifactOutput {
            app_name: self.app_name().to_string(),
            env: self.env(),
            route_id: descriptor.route_id.to_string(),
            schema: descriptor.schema.to_string(),
            kind: descriptor.kind.to_string(),
            version: descriptor.version,
            endpoint: descriptor.endpoint,
            role: descriptor.base_role,
            relative_path,
            path,
            sensitivity: descriptor.sensitivity,
            mutability: descriptor.mutability,
            owner: descriptor.owner,
            policy_version: POLICY_VERSION.to_string(),
        })
    }
}

fn expand_template(template: &str, vars: &PathTemplateVars<'_>) -> Result<String, PathgenError> {
    if template.is_empty() {
        return Err(PathgenError::InvalidArtifactTemplate {
            template: template.to_string(),
            reason: "template cannot be empty",
        });
    }

    let mut out = String::new();
    let mut rest = template;

    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('}') else {
            return Err(PathgenError::InvalidArtifactTemplate {
                template: template.to_string(),
                reason: "unclosed template variable",
            });
        };

        let key = &after_start[..end];
        validate_template_key(template, key)?;
        let value = vars.get(key).ok_or_else(|| PathgenError::MissingArtifactTemplateVar {
            template: template.to_string(),
            key: key.to_string(),
        })?;
        out.push_str(value);
        rest = &after_start[end + 1..];
    }

    if rest.contains('}') {
        return Err(PathgenError::InvalidArtifactTemplate {
            template: template.to_string(),
            reason: "unopened template variable close",
        });
    }

    out.push_str(rest);
    Ok(out)
}

fn validate_template_key(template: &str, key: &str) -> Result<(), PathgenError> {
    if key.is_empty() {
        return Err(PathgenError::InvalidArtifactTemplate {
            template: template.to_string(),
            reason: "template variable cannot be empty",
        });
    }

    let ok = key
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'));

    if !ok {
        return Err(PathgenError::InvalidArtifactTemplate {
            template: template.to_string(),
            reason: "template variable names may contain only ASCII letters, digits, '_' and '-'",
        });
    }

    Ok(())
}

impl Display for ArtifactOutputDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "route={} schema={} endpoint={} role={} template={}",
            self.route_id, self.schema, self.endpoint, self.base_role, self.relative_template
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArtifactOpDescriptor, ArtifactOpHandle};

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

    #[repr(align(8))]
    struct AlignedTest;

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

    #[test]
    fn artifact_output_resolves_through_path_policy() {
        let paths = PathPolicy::prod("example-app").unwrap().with_root("/tmp/rootfs");
        let vars = PathTemplateVars::one("receipt_id", "abc123");

        let resolved = paths.artifact_output(&PROD_JSON, &vars).unwrap();

        assert_eq!(resolved.endpoint, ArtifactEndpoint::FileJson);
        assert_eq!(resolved.route_id, "receipt.package.signing.production");
        assert_eq!(resolved.schema, "example.package.production_signing_receipt.v1");
        assert_eq!(
            resolved.path,
            PathBuf::from("/tmp/rootfs/var/lib/example-app/receipts/package/signing/production/abc123.json")
        );
        assert!(resolved.audit_line().contains("endpoint=file::json"));
    }

    #[test]
    fn artifact_output_rejects_template_parent_traversal_after_expansion() {
        let paths = PathPolicy::test("example-app").unwrap();
        let vars = PathTemplateVars::one("receipt_id", "../evil");
        assert!(matches!(
            paths.artifact_output(&PROD_JSON, &vars),
            Err(PathgenError::UnsafeSubpath { .. })
        ));
    }

    #[test]
    fn artifact_output_rejects_missing_template_vars() {
        let paths = PathPolicy::test("example-app").unwrap();
        let vars = PathTemplateVars::new();
        assert!(matches!(
            paths.artifact_output(&PROD_JSON, &vars),
            Err(PathgenError::MissingArtifactTemplateVar { .. })
        ));
    }

    #[test]
    fn endpoint_tag_uses_three_bit_families() {
        assert_eq!(ArtifactEndpoint::FileJson.label(), "file::json");
        assert_eq!(ArtifactEndpoint::SqliteTable { table: "receipts" }.label(), "sql::sqlite");
        assert_eq!(std::mem::align_of::<AlignedTest>() & crate::ArtifactOpTag::MASK, 0);
    }

    #[test]
    fn compiled_op_handle_validates_contract_and_recovers_descriptor() {
        static OP: ArtifactOpDescriptor = ArtifactOpDescriptor::new(PROD_JSON);

        let handle = ArtifactOpHandle::compile_export::<ProductionReceipt>(&OP).unwrap();

        assert_eq!(handle.tag(), crate::ArtifactOpTag::FileJson);
        assert_eq!(handle.descriptor().output.route_id, "receipt.package.signing.production");
        assert_eq!(handle.word() & crate::ArtifactOpTag::MASK, crate::ArtifactOpTag::FileJson.as_usize());
    }

    #[test]
    fn compiled_op_handle_rejects_endpoint_not_declared_by_contract() {
        static YAML: ArtifactOpDescriptor = ArtifactOpDescriptor::new(ArtifactOutputDescriptor {
            endpoint: ArtifactEndpoint::FileYaml,
            relative_template: "receipts/package/signing/production/{receipt_id}.yaml",
            ..PROD_JSON
        });

        assert!(matches!(
            ArtifactOpHandle::compile_export::<ProductionReceipt>(&YAML),
            Err(PathgenError::ArtifactEndpointNotDeclared { .. })
        ));
    }
}
