use std::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::{AppPathRole, PathPolicy, PathgenError, ResolvedPath};

impl PathPolicy {
    /// Create the directory for a role if it does not already exist.
    pub fn ensure_dir(&self, role: AppPathRole) -> Result<ResolvedPath, PathgenError> {
        let resolved = self.dir(role)?;
        fs::create_dir_all(&resolved.path)?;
        Ok(resolved)
    }

    /// Resolve a file and create its parent directory if needed.
    pub fn ensure_parent_for_file(
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

    /// Validate that a role directory currently exists.
    pub fn validate_dir_exists(&self, role: AppPathRole) -> Result<ResolvedPath, PathgenError> {
        let resolved = self.dir(role)?;
        fs::metadata(&resolved.path)?;
        Ok(resolved)
    }

    /// Unix-only helper for creating a role directory with a requested mode.
    #[cfg(unix)]
    pub fn ensure_dir_mode(
        &self,
        role: AppPathRole,
        mode: u32,
    ) -> Result<ResolvedPath, PathgenError> {
        let resolved = self.ensure_dir(role)?;
        let permissions = fs::Permissions::from_mode(mode);
        fs::set_permissions(&resolved.path, permissions)?;
        Ok(resolved)
    }
}
