use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use crate::{AppPathRole, Env};

/// Broad sensitivity label for audit/evidence output.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PathSensitivity {
    Public,
    Internal,
    Confidential,
    Secret,
}

impl Display for PathSensitivity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let value = match self {
            PathSensitivity::Public => "public",
            PathSensitivity::Internal => "internal",
            PathSensitivity::Confidential => "confidential",
            PathSensitivity::Secret => "secret",
        };
        write!(f, "{}", value)
    }
}

/// Expected mutability behavior for a path role.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PathMutability {
    ReadOnly,
    Writable,
    AppendOnly,
    Ephemeral,
}

impl Display for PathMutability {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let value = match self {
            PathMutability::ReadOnly => "read_only",
            PathMutability::Writable => "writable",
            PathMutability::AppendOnly => "append_only",
            PathMutability::Ephemeral => "ephemeral",
        };
        write!(f, "{}", value)
    }
}

/// Expected ownership class for a path role.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PathOwner {
    Root,
    ServiceUser,
    CurrentUser,
}

impl Display for PathOwner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let value = match self {
            PathOwner::Root => "root",
            PathOwner::ServiceUser => "service_user",
            PathOwner::CurrentUser => "current_user",
        };
        write!(f, "{}", value)
    }
}

/// Evidence object returned whenever a path is resolved.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPath {
    pub app_name: String,
    pub env: Env,
    pub role: AppPathRole,
    pub path: PathBuf,
    pub sensitivity: PathSensitivity,
    pub mutability: PathMutability,
    pub owner: PathOwner,
    pub policy_version: String,
}

impl ResolvedPath {
    /// Human-readable one-line receipt suitable for logs or debug output.
    pub fn audit_line(&self) -> String {
        format!(
            "policy={} app={} env={} role={} path={} sensitivity={} mutability={} owner={}",
            self.policy_version,
            self.app_name,
            self.env,
            self.role,
            self.path.display(),
            self.sensitivity,
            self.mutability,
            self.owner,
        )
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::POLICY_VERSION;

    #[test]
    fn receipt_audit_line_contains_core_evidence() {
        let receipt = ResolvedPath {
            app_name: "archive-api".to_string(),
            env: Env::Prod,
            role: AppPathRole::Logs,
            path: PathBuf::from("/var/log/archive-api/server.log"),
            sensitivity: PathSensitivity::Internal,
            mutability: PathMutability::AppendOnly,
            owner: PathOwner::ServiceUser,
            policy_version: POLICY_VERSION.to_string(),
        };

        let line = receipt.audit_line();
        assert!(line.contains("policy=pathgen.policy.v1"));
        assert!(line.contains("app=archive-api"));
        assert!(line.contains("role=logs"));
        assert!(line.contains("path=/var/log/archive-api/server.log"));
    }
}
