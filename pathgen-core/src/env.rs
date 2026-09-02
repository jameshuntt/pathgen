use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use crate::PathgenError;

/// Runtime environment used to choose path layout rules.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Env {
    Prod,
    Dev,
    Test,
}

impl Display for Env {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let value = match self {
            Env::Prod => "prod",
            Env::Dev => "dev",
            Env::Test => "test",
        };

        write!(f, "{}", value)
    }
}

impl FromStr for Env {
    type Err = PathgenError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim().to_lowercase().as_str() {
            "prod" | "production" => Ok(Env::Prod),
            "dev" | "development" => Ok(Env::Dev),
            "test" | "testing" => Ok(Env::Test),
            _ => Err(PathgenError::InvalidAppName {
                input: input.to_string(),
                reason: "unknown environment; expected prod, dev, or test",
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_lowercase() {
        assert_eq!(Env::Prod.to_string(), "prod");
        assert_eq!(Env::Dev.to_string(), "dev");
        assert_eq!(Env::Test.to_string(), "test");
    }

    #[test]
    fn parses_common_names() {
        assert_eq!("production".parse::<Env>().unwrap(), Env::Prod);
        assert_eq!("development".parse::<Env>().unwrap(), Env::Dev);
        assert_eq!("testing".parse::<Env>().unwrap(), Env::Test);
    }
}
