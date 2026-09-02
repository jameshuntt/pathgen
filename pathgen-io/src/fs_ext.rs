use std::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use pathgen_core::{AppPathRole, PathPolicy, PathgenError, ResolvedPath};

/// Filesystem side-effect helpers for [`PathPolicy`].
///
/// These used to be inherent methods on `PathPolicy`. They now live as an
/// extension trait so `pathgen-core` can remain dependency-light and focused on
/// declaration/resolution.
pub trait PathPolicyFsExt {
    fn ensure_dir(&self, role: AppPathRole) -> Result<ResolvedPath, PathgenError>;

    fn ensure_parent_for_file(
        &self,
        role: AppPathRole,
        file_name: &str,
    ) -> Result<ResolvedPath, PathgenError>;

    fn validate_dir_exists(&self, role: AppPathRole) -> Result<ResolvedPath, PathgenError>;

    #[cfg(unix)]
    fn ensure_dir_mode(&self, role: AppPathRole, mode: u32) -> Result<ResolvedPath, PathgenError>;
}

impl PathPolicyFsExt for PathPolicy {
    fn ensure_dir(&self, role: AppPathRole) -> Result<ResolvedPath, PathgenError> {
        let resolved = self.dir(role)?;
        fs::create_dir_all(&resolved.path)?;
        Ok(resolved)
    }

    fn ensure_parent_for_file(
        &self,
        role: AppPathRole,
        file_name: &str,
    ) -> Result<ResolvedPath, PathgenError> {
        let resolved = self.file(role, file_name)?;

        let parent = resolved
            .path
            .parent()
            .ok_or_else(|| PathgenError::MissingParent {
                path: resolved.path.clone(),
            })?;

        fs::create_dir_all(parent)?;
        Ok(resolved)
    }

    fn validate_dir_exists(&self, role: AppPathRole) -> Result<ResolvedPath, PathgenError> {
        let resolved = self.dir(role)?;
        fs::metadata(&resolved.path)?;
        Ok(resolved)
    }

    #[cfg(unix)]
    fn ensure_dir_mode(&self, role: AppPathRole, mode: u32) -> Result<ResolvedPath, PathgenError> {
        let resolved = self.ensure_dir(role)?;
        let permissions = fs::Permissions::from_mode(mode);
        fs::set_permissions(&resolved.path, permissions)?;
        Ok(resolved)
    }
}
