use std::{fmt::{self, Display, Formatter}, path::{Path, PathBuf}};

use pathgen_core::{
    ArtifactEndpoint, ArtifactOpHandle, Env, PathgenError, ResolvedArtifactOutput,
};
use sha2::{Digest, Sha256};

/// Cryptographic digest of the exact payload bytes planned for an artifact output.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PayloadDigest {
    pub algorithm: &'static str,
    pub value: String,
}

impl PayloadDigest {
    pub fn sha256(payload: &[u8]) -> Self {
        let digest = Sha256::digest(payload);
        Self {
            algorithm: "sha256",
            value: hex_lower(&digest),
        }
    }

    pub fn prefixed(&self) -> String {
        format!("{}:{}", self.algorithm, self.value)
    }
}

impl Display for PayloadDigest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.value)
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Resolved target for a planned artifact output.
///
/// Pass 03 implements file targets first. SQL and chain targets should get their
/// own variants when their adapters are introduced, without changing the core
/// `ArtifactContract` declaration model.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResolvedArtifactTarget {
    File { path: PathBuf },
}

impl ResolvedArtifactTarget {
    pub fn path(&self) -> &Path {
        match self {
            ResolvedArtifactTarget::File { path } => path.as_path(),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ResolvedArtifactTarget::File { .. } => "file",
        }
    }
}

/// Auditable side-effect intent created before writing/sending an artifact.
///
/// This is the pass-03 bridge between path resolution and execution:
///
/// ```text
/// declared artifact + endpoint + resolved target + payload digest
///   -> ArtifactOutputPlan
///   -> writer consumes plan
///   -> ArtifactWriteReceipt
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactOutputPlan {
    pub app_name: String,
    pub env: Env,
    pub route_id: String,
    pub schema: String,
    pub kind: String,
    pub version: u32,
    pub endpoint: ArtifactEndpoint,
    pub target: ResolvedArtifactTarget,
    pub bytes_len: usize,
    pub payload_digest: PayloadDigest,
    pub policy_version: String,
    pub op_word: Option<usize>,
    pub op_clean_addr: Option<usize>,
}

impl ArtifactOutputPlan {
    /// Build a file output plan from pathgen's resolved artifact evidence and
    /// the exact bytes that would be written.
    pub fn file(resolved: &ResolvedArtifactOutput, payload: &[u8]) -> Result<Self, PathgenError> {
        if !resolved.endpoint.is_file() {
            return Err(PathgenError::UnsupportedArtifactEndpoint {
                endpoint: resolved.endpoint.label(),
                reason: "ArtifactOutputPlan::file requires a file::* endpoint",
            });
        }

        Ok(Self::from_parts(resolved, payload, None))
    }

    /// Build a file output plan and bind it to a compiled operation handle.
    ///
    /// The handle is not the authority; this function proves that the handle's
    /// descriptor and the resolved target describe the same route/schema/kind/
    /// version/endpoint before recording the packed word as dispatch evidence.
    pub fn file_with_op(
        handle: ArtifactOpHandle,
        resolved: &ResolvedArtifactOutput,
        payload: &[u8],
    ) -> Result<Self, PathgenError> {
        validate_handle_matches_resolved(handle, resolved)?;
        Ok(Self::from_parts(resolved, payload, Some(handle)))
    }

    fn from_parts(
        resolved: &ResolvedArtifactOutput,
        payload: &[u8],
        handle: Option<ArtifactOpHandle>,
    ) -> Self {
        Self {
            app_name: resolved.app_name.clone(),
            env: resolved.env,
            route_id: resolved.route_id.clone(),
            schema: resolved.schema.clone(),
            kind: resolved.kind.clone(),
            version: resolved.version,
            endpoint: resolved.endpoint,
            target: ResolvedArtifactTarget::File {
                path: resolved.path.clone(),
            },
            bytes_len: payload.len(),
            payload_digest: PayloadDigest::sha256(payload),
            policy_version: resolved.policy_version.clone(),
            op_word: handle.map(ArtifactOpHandle::word),
            op_clean_addr: handle.map(ArtifactOpHandle::clean_addr),
        }
    }

    pub fn target_path(&self) -> &Path {
        self.target.path()
    }

    pub fn audit_line(&self) -> String {
        format!(
            "policy={} app={} env={} route={} schema={} kind={} version={} endpoint={} target={} path={} bytes_len={} payload_digest={} op_word={}",
            self.policy_version,
            self.app_name,
            self.env,
            self.route_id,
            self.schema,
            self.kind,
            self.version,
            self.endpoint,
            self.target.label(),
            self.target_path().display(),
            self.bytes_len,
            self.payload_digest,
            self.op_word
                .map(|word| format!("{word:#x}"))
                .unwrap_or_else(|| "none".to_string()),
        )
    }
}

fn validate_handle_matches_resolved(
    handle: ArtifactOpHandle,
    resolved: &ResolvedArtifactOutput,
) -> Result<(), PathgenError> {
    let output = handle.descriptor().output;

    if output.route_id != resolved.route_id.as_str() {
        return Err(plan_mismatch("route_id", output.route_id, &resolved.route_id));
    }
    if output.schema != resolved.schema.as_str() {
        return Err(plan_mismatch("schema", output.schema, &resolved.schema));
    }
    if output.kind != resolved.kind.as_str() {
        return Err(plan_mismatch("kind", output.kind, &resolved.kind));
    }
    if output.version != resolved.version {
        return Err(PathgenError::ArtifactVersionMismatch {
            expected: output.version,
            actual: resolved.version,
        });
    }
    if output.endpoint != resolved.endpoint {
        return Err(PathgenError::ArtifactOutputPlanMismatch {
            field: "endpoint",
            expected: output.endpoint.label().to_string(),
            actual: resolved.endpoint.label().to_string(),
        });
    }

    Ok(())
}

fn plan_mismatch(field: &'static str, expected: &'static str, actual: &str) -> PathgenError {
    PathgenError::ArtifactOutputPlanMismatch {
        field,
        expected: expected.to_string(),
        actual: actual.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_digest_is_stable() {
        let digest = PayloadDigest::sha256(b"hello");
        assert_eq!(digest.algorithm, "sha256");
        assert_eq!(
            digest.value,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(
            digest.prefixed(),
            "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
