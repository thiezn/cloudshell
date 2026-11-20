use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::PathBuf};

#[derive(Debug, Default, Deserialize)]
pub(crate) struct CliConfig {
    #[serde(default)]
    pub(crate) default: DefaultSettings,
    #[serde(flatten)]
    profiles: HashMap<String, ProfileSettings>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct DefaultSettings {
    pub(crate) profile: Option<String>,
    pub(crate) region: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProfileSettings {
    #[serde(alias = "vpc-id")]
    pub(crate) vpc_id: Option<String>,
    #[serde(
        rename = "subnets_ids",
        alias = "subnet_ids",
        alias = "subnet-ids",
        alias = "subnets-ids"
    )]
    pub(crate) subnet_ids: Option<Vec<String>>,
    #[serde(rename = "security-group-ids", alias = "security_group_ids")]
    pub(crate) security_group_ids: Option<Vec<String>>,
}

impl CliConfig {
    pub(crate) fn load() -> Result<Self> {
        let Some(home) = env::var_os("HOME") else {
            return Ok(Self::default());
        };
        let config_path = PathBuf::from(home).join(".cloudshell/config.toml");
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_path)
            .with_context(|| format!("could not read {}", config_path.display()))?;
        toml::from_str(&contents)
            .with_context(|| format!("could not parse {}", config_path.display()))
    }

    pub(crate) fn profile_settings(&self, profile: &str) -> Option<&ProfileSettings> {
        self.profiles.get(&format!("profile-{profile}"))
    }
}

#[cfg(test)]
mod tests {
    use super::CliConfig;

    #[test]
    fn parses_default_and_profile_settings() -> Result<(), toml::de::Error> {
        let config: CliConfig = toml::from_str(
            r#"
[default]
profile = "sandbox"
region = "eu-west-1"

[profile-sandbox]
vpc_id = "vpc-123"
subnets_ids = ["subnet-123", "subnet-456"]
security-group-ids = ["sg-123"]
"#,
        )?;

        assert_eq!(config.default.profile.as_deref(), Some("sandbox"));
        assert_eq!(config.default.region.as_deref(), Some("eu-west-1"));
        let profile = config.profile_settings("sandbox");
        assert!(profile.is_some());
        if let Some(profile) = profile {
            assert_eq!(profile.vpc_id.as_deref(), Some("vpc-123"));
            assert_eq!(
                profile
                    .subnet_ids
                    .as_ref()
                    .map(|ids| ids.iter().map(String::as_str).collect::<Vec<_>>()),
                Some(vec!["subnet-123", "subnet-456"])
            );
            assert_eq!(
                profile
                    .security_group_ids
                    .as_ref()
                    .map(|ids| ids.iter().map(String::as_str).collect::<Vec<_>>()),
                Some(vec!["sg-123"])
            );
        }
        Ok(())
    }
}
