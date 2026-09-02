use std::path::{Path, PathBuf};

use crate::{
    safety::{checked_join, validate_app_name},
    AppPathRole, Env, PathMutability, PathOwner, PathSensitivity, PathgenError, ResolvedPath,
    StandardPath,
};

/// Stable policy version emitted in [`ResolvedPath`] receipts.
pub const POLICY_VERSION: &str = "pathgen.policy.v1";

/// The central path policy object.
///
/// Create one policy per app/service/env and pass it into subsystems that need
/// deterministic paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPolicy {
    app_name: String,
    env: Env,
    root: Option<PathBuf>,
    dev_base: PathBuf,
    test_base: PathBuf,
}

impl PathPolicy {
    pub fn new(app_name: impl Into<String>, env: Env) -> Result<Self, PathgenError> {
        let app_name = app_name.into();
        validate_app_name(&app_name)?;

        Ok(Self {
            app_name,
            env,
            root: None,
            dev_base: PathBuf::from("/tmp/pathgen-dev"),
            test_base: PathBuf::from("/tmp/pathgen-test"),
        })
    }

    pub fn prod(app_name: impl Into<String>) -> Result<Self, PathgenError> {
        Self::new(app_name, Env::Prod)
    }

    pub fn dev(app_name: impl Into<String>) -> Result<Self, PathgenError> {
        Self::new(app_name, Env::Dev)
    }

    pub fn test(app_name: impl Into<String>) -> Result<Self, PathgenError> {
        Self::new(app_name, Env::Test)
    }

    /// Remap absolute prod/dev/test paths under a different root.
    ///
    /// Useful for rootfs image generation, chroot layouts, container filesystem
    /// assembly, and integration tests.
    pub fn with_root(mut self, root: impl AsRef<Path>) -> Self {
        self.root = Some(root.as_ref().to_path_buf());
        self
    }

    /// Override the base directory used for development environment paths.
    pub fn with_dev_base(mut self, base: impl AsRef<Path>) -> Self {
        self.dev_base = base.as_ref().to_path_buf();
        self
    }

    /// Override the base directory used for test environment paths.
    pub fn with_test_base(mut self, base: impl AsRef<Path>) -> Self {
        self.test_base = base.as_ref().to_path_buf();
        self
    }

    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    pub fn env(&self) -> Env {
        self.env
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Resolve the directory for a semantic role.
    pub fn dir(&self, role: AppPathRole) -> Result<ResolvedPath, PathgenError> {
        let path = self.resolve_dir_path(role)?;
        Ok(self.receipt(role, path))
    }

    /// Resolve a file under a semantic role directory.
    pub fn file(
        &self,
        role: AppPathRole,
        name: impl AsRef<Path>,
    ) -> Result<ResolvedPath, PathgenError> {
        let dir = self.resolve_dir_path(role)?;
        let path = checked_join(&dir, name)?;
        Ok(self.receipt(role, path))
    }

    /// Resolve a runtime socket under the runtime directory.
    pub fn socket(&self, name: impl AsRef<Path>) -> Result<ResolvedPath, PathgenError> {
        self.file(AppPathRole::Runtime, name)
    }

    pub fn config_file(&self, name: impl AsRef<Path>) -> Result<ResolvedPath, PathgenError> {
        self.file(AppPathRole::Config, name)
    }

    pub fn log_file(&self, name: impl AsRef<Path>) -> Result<ResolvedPath, PathgenError> {
        self.file(AppPathRole::Logs, name)
    }

    pub fn state_file(&self, name: impl AsRef<Path>) -> Result<ResolvedPath, PathgenError> {
        self.file(AppPathRole::State, name)
    }

    pub fn data_file(&self, name: impl AsRef<Path>) -> Result<ResolvedPath, PathgenError> {
        self.file(AppPathRole::Data, name)
    }

    /// Creates a receipt for every known role without touching the filesystem.
    pub fn dry_run_manifest(&self) -> Result<Vec<ResolvedPath>, PathgenError> {
        let roles = [
            AppPathRole::Config,
            AppPathRole::Logs,
            AppPathRole::Cache,
            AppPathRole::Runtime,
            AppPathRole::Temp,
            AppPathRole::State,
            AppPathRole::Data,
            AppPathRole::ProjectRoot,
            AppPathRole::SystemdUnits,
            AppPathRole::Certs,
            AppPathRole::Secrets,
        ];

        roles.into_iter().map(|role| self.dir(role)).collect()
    }

    pub(crate) fn resolve_dir_path(&self, role: AppPathRole) -> Result<PathBuf, PathgenError> {
        let base = match self.env {
            Env::Prod => self.prod_dir(role)?,
            Env::Dev => self.dev_dir(role)?,
            Env::Test => self.test_dir(role)?,
        };

        Ok(match &self.root {
            Some(root) => remap_under_root(root, &base),
            None => base,
        })
    }

    fn prod_dir(&self, role: AppPathRole) -> Result<PathBuf, PathgenError> {
        let app = self.app_name.as_str();

        let path = match role {
            AppPathRole::Config => StandardPath::Etc.as_path().join(app),
            AppPathRole::Logs => StandardPath::VarLog.as_path().join(app),
            AppPathRole::Cache => StandardPath::VarCache.as_path().join(app),
            AppPathRole::Runtime => StandardPath::Run.as_path().join(app),
            AppPathRole::Temp => StandardPath::Tmp.as_path().join(app),
            AppPathRole::State => StandardPath::VarLib.as_path().join(app),
            AppPathRole::Data => StandardPath::VarLib.as_path().join(app).join("data"),
            AppPathRole::ProjectRoot => StandardPath::Opt.as_path().join(app),
            AppPathRole::SystemdUnits => PathBuf::from("/etc/systemd/system"),
            AppPathRole::Certs => StandardPath::Etc.as_path().join(app).join("certs"),
            AppPathRole::Secrets => StandardPath::Etc.as_path().join(app).join("secrets"),
        };

        Ok(path)
    }

    fn dev_dir(&self, role: AppPathRole) -> Result<PathBuf, PathgenError> {
        let root = checked_join(&self.dev_base, self.app_name.as_str())?;
        checked_join(&root, role.label())
    }

    fn test_dir(&self, role: AppPathRole) -> Result<PathBuf, PathgenError> {
        let root = checked_join(&self.test_base, self.app_name.as_str())?;
        checked_join(&root, role.label())
    }

    fn receipt(&self, role: AppPathRole, path: PathBuf) -> ResolvedPath {
        ResolvedPath {
            app_name: self.app_name.clone(),
            env: self.env,
            role,
            path,
            sensitivity: self.sensitivity_for(role),
            mutability: self.mutability_for(role),
            owner: self.owner_for(role),
            policy_version: POLICY_VERSION.to_string(),
        }
    }

    fn sensitivity_for(&self, role: AppPathRole) -> PathSensitivity {
        match role {
            AppPathRole::Config => PathSensitivity::Confidential,
            AppPathRole::Logs => PathSensitivity::Internal,
            AppPathRole::Cache => PathSensitivity::Internal,
            AppPathRole::Runtime => PathSensitivity::Internal,
            AppPathRole::Temp => PathSensitivity::Internal,
            AppPathRole::State => PathSensitivity::Confidential,
            AppPathRole::Data => PathSensitivity::Confidential,
            AppPathRole::ProjectRoot => PathSensitivity::Internal,
            AppPathRole::SystemdUnits => PathSensitivity::Confidential,
            AppPathRole::Certs => PathSensitivity::Secret,
            AppPathRole::Secrets => PathSensitivity::Secret,
        }
    }

    fn mutability_for(&self, role: AppPathRole) -> PathMutability {
        match role {
            AppPathRole::Config => PathMutability::ReadOnly,
            AppPathRole::Logs => PathMutability::AppendOnly,
            AppPathRole::Cache => PathMutability::Writable,
            AppPathRole::Runtime => PathMutability::Ephemeral,
            AppPathRole::Temp => PathMutability::Ephemeral,
            AppPathRole::State => PathMutability::Writable,
            AppPathRole::Data => PathMutability::Writable,
            AppPathRole::ProjectRoot => PathMutability::ReadOnly,
            AppPathRole::SystemdUnits => PathMutability::ReadOnly,
            AppPathRole::Certs => PathMutability::ReadOnly,
            AppPathRole::Secrets => PathMutability::ReadOnly,
        }
    }

    fn owner_for(&self, role: AppPathRole) -> PathOwner {
        match role {
            AppPathRole::Config
            | AppPathRole::ProjectRoot
            | AppPathRole::SystemdUnits
            | AppPathRole::Certs
            | AppPathRole::Secrets => PathOwner::Root,

            AppPathRole::Logs
            | AppPathRole::Cache
            | AppPathRole::Runtime
            | AppPathRole::Temp
            | AppPathRole::State
            | AppPathRole::Data => PathOwner::ServiceUser,
        }
    }
}

fn remap_under_root(root: &Path, absolute: &Path) -> PathBuf {
    let rel = absolute.strip_prefix("/").unwrap_or(absolute);
    root.join(rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prod_policy_resolves_primary_dirs() {
        let paths = PathPolicy::prod("archive-api").unwrap();

        assert_eq!(
            paths.dir(AppPathRole::Config).unwrap().path,
            PathBuf::from("/etc/archive-api")
        );
        assert_eq!(
            paths.dir(AppPathRole::Logs).unwrap().path,
            PathBuf::from("/var/log/archive-api")
        );
        assert_eq!(
            paths.dir(AppPathRole::Runtime).unwrap().path,
            PathBuf::from("/run/archive-api")
        );
        assert_eq!(
            paths.dir(AppPathRole::State).unwrap().path,
            PathBuf::from("/var/lib/archive-api")
        );
    }

    #[test]
    fn prod_policy_resolves_files() {
        let paths = PathPolicy::prod("archive-api").unwrap();

        assert_eq!(
            paths.config_file("app.toml").unwrap().path,
            PathBuf::from("/etc/archive-api/app.toml")
        );
        assert_eq!(
            paths.log_file("server.log").unwrap().path,
            PathBuf::from("/var/log/archive-api/server.log")
        );
        assert_eq!(
            paths.socket("api.sock").unwrap().path,
            PathBuf::from("/run/archive-api/api.sock")
        );
    }

    #[test]
    fn dev_policy_uses_dev_base() {
        let paths = PathPolicy::dev("archive-api").unwrap();
        assert_eq!(
            paths.dir(AppPathRole::Logs).unwrap().path,
            PathBuf::from("/tmp/pathgen-dev/archive-api/logs")
        );
    }

    #[test]
    fn test_policy_uses_test_base() {
        let paths = PathPolicy::test("archive-api").unwrap();
        assert_eq!(
            paths.dir(AppPathRole::Logs).unwrap().path,
            PathBuf::from("/tmp/pathgen-test/archive-api/logs")
        );
    }

    #[test]
    fn root_remap_preserves_absolute_layout_under_root() {
        let paths = PathPolicy::prod("archive-api")
            .unwrap()
            .with_root("/rootfs");
        assert_eq!(
            paths.log_file("server.log").unwrap().path,
            PathBuf::from("/rootfs/var/log/archive-api/server.log")
        );
        assert_eq!(
            paths.dir(AppPathRole::SystemdUnits).unwrap().path,
            PathBuf::from("/rootfs/etc/systemd/system")
        );
    }

    #[test]
    fn dry_run_manifest_contains_all_roles() {
        let paths = PathPolicy::prod("archive-api").unwrap();
        let manifest = paths.dry_run_manifest().unwrap();
        assert_eq!(manifest.len(), 11);
        assert!(manifest.iter().any(|p| p.role == AppPathRole::Config));
        assert!(manifest.iter().any(|p| p.role == AppPathRole::Secrets));
    }
}
