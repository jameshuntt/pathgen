use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use crate::PathgenError;

/// Semantic app/service path roles.
///
/// Application crates should depend on these roles instead of manually writing
/// concrete paths such as `/var/log/myapp` or `/run/myapp`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum AppPathRole {
    Config,
    Logs,
    Cache,
    Runtime,
    Temp,
    State,
    Data,
    ProjectRoot,
    SystemdUnits,
    Certs,
    Secrets,
}

impl AppPathRole {
    /// Stable lower-case role label used in dev/test layouts and receipts.
    pub fn label(self) -> &'static str {
        match self {
            AppPathRole::Config => "config",
            AppPathRole::Logs => "logs",
            AppPathRole::Cache => "cache",
            AppPathRole::Runtime => "runtime",
            AppPathRole::Temp => "temp",
            AppPathRole::State => "state",
            AppPathRole::Data => "data",
            AppPathRole::ProjectRoot => "project_root",
            AppPathRole::SystemdUnits => "systemd_units",
            AppPathRole::Certs => "certs",
            AppPathRole::Secrets => "secrets",
        }
    }
}

impl Display for AppPathRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl FromStr for AppPathRole {
    type Err = PathgenError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim().to_lowercase().as_str() {
            "config" | "conf" | "configuration" => Ok(AppPathRole::Config),
            "logs" | "log" => Ok(AppPathRole::Logs),
            "cache" => Ok(AppPathRole::Cache),
            "runtime" | "run" => Ok(AppPathRole::Runtime),
            "temp" | "tmp" => Ok(AppPathRole::Temp),
            "state" | "varlib" | "var_lib" => Ok(AppPathRole::State),
            "data" => Ok(AppPathRole::Data),
            "project_root" | "project-root" | "projectroot" | "root" => {
                Ok(AppPathRole::ProjectRoot)
            }
            "systemd_units" | "systemd-units" | "systemd" | "units" => {
                Ok(AppPathRole::SystemdUnits)
            }
            "certs" | "certificates" | "tls" => Ok(AppPathRole::Certs),
            "secrets" | "secret" => Ok(AppPathRole::Secrets),
            _ => Err(PathgenError::UnknownAppPathRole {
                input: input.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_stable() {
        assert_eq!(AppPathRole::Config.label(), "config");
        assert_eq!(AppPathRole::ProjectRoot.label(), "project_root");
        assert_eq!(AppPathRole::SystemdUnits.label(), "systemd_units");
    }

    #[test]
    fn display_uses_label() {
        assert_eq!(AppPathRole::Runtime.to_string(), "runtime");
    }

    #[test]
    fn parses_aliases() {
        assert_eq!("conf".parse::<AppPathRole>().unwrap(), AppPathRole::Config);
        assert_eq!("tmp".parse::<AppPathRole>().unwrap(), AppPathRole::Temp);
        assert_eq!("tls".parse::<AppPathRole>().unwrap(), AppPathRole::Certs);
    }
}
