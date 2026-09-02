use std::{
    fmt::{self, Display, Formatter},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::PathgenError;

/// OS/system-level path anchors used by path policy rules.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum StandardPath {
    Root,
    Bin,
    Sbin,
    Usr,
    UsrBin,
    UsrSbin,
    Lib,
    Lib64,
    Etc,
    Var,
    VarLog,
    VarCache,
    VarLib,
    Opt,
    Tmp,
    Home,
    Mnt,
    Media,
    Proc,
    Sys,
    Dev,
    Boot,
    Run,
}

impl StandardPath {
    /// Returns the concrete absolute path for a standard path anchor.
    pub fn as_path(self) -> &'static Path {
        match self {
            StandardPath::Root => Path::new("/"),
            StandardPath::Bin => Path::new("/bin"),
            StandardPath::Sbin => Path::new("/sbin"),
            StandardPath::Usr => Path::new("/usr"),
            StandardPath::UsrBin => Path::new("/usr/bin"),
            StandardPath::UsrSbin => Path::new("/usr/sbin"),
            StandardPath::Lib => Path::new("/lib"),
            StandardPath::Lib64 => Path::new("/lib64"),
            StandardPath::Etc => Path::new("/etc"),
            StandardPath::Var => Path::new("/var"),
            StandardPath::VarLog => Path::new("/var/log"),
            StandardPath::VarCache => Path::new("/var/cache"),
            StandardPath::VarLib => Path::new("/var/lib"),
            StandardPath::Opt => Path::new("/opt"),
            StandardPath::Tmp => Path::new("/tmp"),
            StandardPath::Home => Path::new("/home"),
            StandardPath::Mnt => Path::new("/mnt"),
            StandardPath::Media => Path::new("/media"),
            StandardPath::Proc => Path::new("/proc"),
            StandardPath::Sys => Path::new("/sys"),
            StandardPath::Dev => Path::new("/dev"),
            StandardPath::Boot => Path::new("/boot"),
            StandardPath::Run => Path::new("/run"),
        }
    }

    /// Remaps this absolute anchor under an alternate root.
    ///
    /// Example: `/var/log` under `/rootfs` becomes `/rootfs/var/log`.
    pub fn under_root(self, root: &Path) -> PathBuf {
        let raw = self.as_path();
        let rel = raw.strip_prefix("/").unwrap_or(raw);
        root.join(rel)
    }
}

impl Display for StandardPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_path().display())
    }
}

impl FromStr for StandardPath {
    type Err = PathgenError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let normalized = input
            .trim()
            .trim_start_matches('/')
            .trim_end_matches('/')
            .to_lowercase();

        match normalized.as_str() {
            "" | "root" => Ok(StandardPath::Root),
            "bin" => Ok(StandardPath::Bin),
            "sbin" => Ok(StandardPath::Sbin),
            "usr" => Ok(StandardPath::Usr),
            "usr/bin" | "usrbin" | "usr_bin" => Ok(StandardPath::UsrBin),
            "usr/sbin" | "usrsbin" | "usr_sbin" => Ok(StandardPath::UsrSbin),
            "lib" => Ok(StandardPath::Lib),
            "lib64" => Ok(StandardPath::Lib64),
            "etc" => Ok(StandardPath::Etc),
            "var" => Ok(StandardPath::Var),
            "var/log" | "varlog" | "var_log" => Ok(StandardPath::VarLog),
            "var/cache" | "varcache" | "var_cache" => Ok(StandardPath::VarCache),
            "var/lib" | "varlib" | "var_lib" => Ok(StandardPath::VarLib),
            "opt" => Ok(StandardPath::Opt),
            "tmp" | "temp" => Ok(StandardPath::Tmp),
            "home" => Ok(StandardPath::Home),
            "mnt" => Ok(StandardPath::Mnt),
            "media" => Ok(StandardPath::Media),
            "proc" => Ok(StandardPath::Proc),
            "sys" => Ok(StandardPath::Sys),
            "dev" => Ok(StandardPath::Dev),
            "boot" => Ok(StandardPath::Boot),
            "run" => Ok(StandardPath::Run),
            _ => Err(PathgenError::UnknownStandardPath {
                input: input.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_paths_display_as_absolute_paths() {
        assert_eq!(StandardPath::Etc.to_string(), "/etc");
        assert_eq!(StandardPath::VarLog.to_string(), "/var/log");
        assert_eq!(StandardPath::Run.to_string(), "/run");
    }

    #[test]
    fn standard_paths_parse_from_absolute_or_relative_names() {
        assert_eq!("/etc".parse::<StandardPath>().unwrap(), StandardPath::Etc);
        assert_eq!(
            "var/log".parse::<StandardPath>().unwrap(),
            StandardPath::VarLog
        );
        assert_eq!(
            "/usr/bin/".parse::<StandardPath>().unwrap(),
            StandardPath::UsrBin
        );
    }

    #[test]
    fn standard_path_under_root_remaps_absolute_anchor() {
        let remapped = StandardPath::VarLog.under_root(Path::new("/rootfs"));
        assert_eq!(remapped, PathBuf::from("/rootfs/var/log"));
    }
}
