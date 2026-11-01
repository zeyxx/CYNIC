use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Canonical build action vocabulary for the CYNIC build pipeline.
///
/// Serialized as the long string form (`cargo:check`, ...). Deserialization
/// also accepts the short aliases (`check`, `build`, `build-release`, `test`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum BuildAction {
    #[serde(rename = "cargo:check")]
    CargoCheck,
    #[serde(rename = "cargo:build")]
    CargoBuild,
    #[serde(rename = "cargo:build-release")]
    CargoBuildRelease,
    #[serde(rename = "cargo:test")]
    CargoTest,
    #[serde(rename = "audit:crate")]
    AuditCrate,
    #[serde(rename = "audit:all")]
    AuditAll,
}

impl BuildAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CargoCheck => "cargo:check",
            Self::CargoBuild => "cargo:build",
            Self::CargoBuildRelease => "cargo:build-release",
            Self::CargoTest => "cargo:test",
            Self::AuditCrate => "audit:crate",
            Self::AuditAll => "audit:all",
        }
    }

    pub fn profile(&self) -> &'static str {
        match self {
            Self::CargoBuildRelease => "release",
            _ => "debug",
        }
    }
}

impl Default for BuildAction {
    fn default() -> Self {
        Self::CargoCheck
    }
}

impl FromStr for BuildAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "cargo:check" | "check" => Ok(Self::CargoCheck),
            "cargo:build" | "build" => Ok(Self::CargoBuild),
            "cargo:build-release" | "build-release" | "build:release" | "release" => Ok(Self::CargoBuildRelease),
            "cargo:test" | "test" => Ok(Self::CargoTest),
            "audit:crate" | "audit" => Ok(Self::AuditCrate),
            "audit:all" | "audit-all" | "audit_all" => Ok(Self::AuditAll),
            other => Err(format!("unknown build action: {other}")),
        }
    }
}

fn deserialize_build_action<'de, D>(deserializer: D) -> Result<BuildAction, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    BuildAction::from_str(&s).map_err(serde::de::Error::custom)
}

// Re-attach custom deserializer via a wrapper derive is not possible on enum directly
// with rename attributes above, so implement Deserialize manually.
impl<'de> Deserialize<'de> for BuildAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_build_action(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_round_trips() {
        let cases = [
            (BuildAction::CargoCheck, "cargo:check"),
            (BuildAction::CargoBuild, "cargo:build"),
            (BuildAction::CargoBuildRelease, "cargo:build-release"),
            (BuildAction::CargoTest, "cargo:test"),
            (BuildAction::AuditCrate, "audit:crate"),
            (BuildAction::AuditAll, "audit:all"),
        ];
        for (action, s) in cases {
            assert_eq!(action.as_str(), s);
            let parsed: BuildAction = s.parse().unwrap();
            assert_eq!(parsed, action);
        }
    }

    #[test]
    fn short_aliases_parse() {
        assert_eq!("check".parse::<BuildAction>().unwrap(), BuildAction::CargoCheck);
        assert_eq!("build".parse::<BuildAction>().unwrap(), BuildAction::CargoBuild);
        assert_eq!(
            "build-release".parse::<BuildAction>().unwrap(),
            BuildAction::CargoBuildRelease
        );
        assert_eq!(
            "build:release".parse::<BuildAction>().unwrap(),
            BuildAction::CargoBuildRelease
        );
        assert_eq!(
            "release".parse::<BuildAction>().unwrap(),
            BuildAction::CargoBuildRelease
        );
        assert_eq!("test".parse::<BuildAction>().unwrap(), BuildAction::CargoTest);
    }

    #[test]
    fn default_is_cargo_check() {
        assert_eq!(BuildAction::default(), BuildAction::CargoCheck);
    }

    #[test]
    fn profile_mapping() {
        assert_eq!(BuildAction::CargoBuildRelease.profile(), "release");
        assert_eq!(BuildAction::CargoCheck.profile(), "debug");
        assert_eq!(BuildAction::CargoBuild.profile(), "debug");
        assert_eq!(BuildAction::CargoTest.profile(), "debug");
    }

    #[test]
    fn serde_round_trip() {
        for action in [
            BuildAction::CargoCheck,
            BuildAction::CargoBuild,
            BuildAction::CargoBuildRelease,
            BuildAction::CargoTest,
            BuildAction::AuditCrate,
            BuildAction::AuditAll,
        ] {
            let json = serde_json::to_string(&action).unwrap();
            let back: BuildAction = serde_json::from_str(&json).unwrap();
            assert_eq!(back, action);
        }
    }

    #[test]
    fn serde_accepts_short_alias() {
        let v: BuildAction = serde_json::from_str("\"check\"").unwrap();
        assert_eq!(v, BuildAction::CargoCheck);
    }
}
