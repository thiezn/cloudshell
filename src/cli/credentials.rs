use anyhow::{Context, Result};
use aws_credential_types::{
    Credentials,
    provider::{ProvideCredentials, SharedCredentialsProvider, error::CredentialsError, future},
};
use aws_smithy_types::{DateTime, date_time::Format};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt, fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use tokio::sync::Mutex;

const CREDENTIAL_REFRESH_WINDOW: Duration = Duration::from_secs(60);

#[derive(Default, Deserialize, Serialize)]
struct CredentialsFile {
    #[serde(flatten)]
    profiles: HashMap<String, StoredCredentials>,
}

#[derive(Deserialize, Serialize)]
struct StoredCredentials {
    aws_access_key_id: String,
    aws_secret_access_key: String,
    aws_session_token: String,
    aws_expiration: String,
}

#[derive(Clone)]
struct CredentialStore {
    path: PathBuf,
}

impl CredentialStore {
    fn load_profile(&self, profile: &str) -> Result<Option<Credentials>> {
        let file = self.read_and_prune()?;
        let Some(stored) = file.profiles.get(profile) else {
            return Ok(None);
        };
        let expiration = parse_expiration(&stored.aws_expiration)?;
        Ok(Some(Credentials::new(
            stored.aws_access_key_id.clone(),
            stored.aws_secret_access_key.clone(),
            Some(stored.aws_session_token.clone()),
            Some(expiration),
            "cloudshell-credentials-file",
        )))
    }

    fn save_profile(&self, profile: &str, credentials: &Credentials) -> Result<()> {
        let (Some(expiration), Some(session_token)) =
            (credentials.expiry(), credentials.session_token())
        else {
            return Ok(());
        };
        if expiration <= SystemTime::now() {
            return Ok(());
        }

        let mut file = self.read_and_prune()?;
        file.profiles.insert(
            profile.to_owned(),
            StoredCredentials {
                aws_access_key_id: credentials.access_key_id().to_owned(),
                aws_secret_access_key: credentials.secret_access_key().to_owned(),
                aws_session_token: session_token.to_owned(),
                aws_expiration: format_expiration(expiration)?,
            },
        );
        self.write(&file)
    }

    fn read_and_prune(&self) -> Result<CredentialsFile> {
        if !self.path.exists() {
            return Ok(CredentialsFile::default());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&self.path)?.permissions();
            if permissions.mode() & 0o777 != 0o600 {
                permissions.set_mode(0o600);
                fs::set_permissions(&self.path, permissions).with_context(|| {
                    format!("could not restrict permissions on {}", self.path.display())
                })?;
            }
        }

        let contents = fs::read_to_string(&self.path)
            .with_context(|| format!("could not read {}", self.path.display()))?;
        let mut file: CredentialsFile = toml::from_str(&contents)
            .with_context(|| format!("could not parse {}", self.path.display()))?;
        let now = SystemTime::now();
        let mut expired_profiles = Vec::new();
        for (profile, credentials) in &file.profiles {
            let expiration = parse_expiration(&credentials.aws_expiration)
                .with_context(|| format!("invalid expiration for cached profile '{profile}'"))?;
            if expiration <= now {
                expired_profiles.push(profile.clone());
            }
        }
        if !expired_profiles.is_empty() {
            for profile in expired_profiles {
                file.profiles.remove(&profile);
            }
            self.write(&file)?;
        }
        Ok(file)
    }

    fn write(&self, file: &CredentialsFile) -> Result<()> {
        let directory = self
            .path
            .parent()
            .context("credential cache path has no parent directory")?;
        fs::create_dir_all(directory)
            .with_context(|| format!("could not create {}", directory.display()))?;
        let temporary_path = directory.join(format!(".credentials-{}.tmp", uuid::Uuid::new_v4()));
        let contents = toml::to_string(file).context("could not serialize credential cache")?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut temporary_file = options
            .open(&temporary_path)
            .with_context(|| format!("could not create {}", temporary_path.display()))?;
        temporary_file
            .write_all(contents.as_bytes())
            .with_context(|| format!("could not write {}", temporary_path.display()))?;
        temporary_file
            .sync_all()
            .with_context(|| format!("could not sync {}", temporary_path.display()))?;
        drop(temporary_file);
        fs::rename(&temporary_path, &self.path).with_context(|| {
            format!(
                "could not replace credential cache at {}",
                self.path.display()
            )
        })
    }
}

pub async fn load_aws_config_with_cache(
    profile: Option<&str>,
    region: Option<&str>,
    credentials_path: Option<&Path>,
) -> Result<aws_config::SdkConfig> {
    let store = credentials_path.map(|path| CredentialStore {
        path: path.to_path_buf(),
    });
    let cached = match (profile, store.as_ref()) {
        (Some(profile), Some(store)) => store.load_profile(profile)?,
        _ => None,
    };
    let build_loader = || {
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
        if let Some(profile) = profile {
            loader = loader.profile_name(profile);
        }
        if let Some(region) = region {
            loader = loader.region(aws_config::Region::new(region.to_owned()));
        }
        loader
    };

    let base_config = build_loader().load().await;
    if let (Some(profile), Some(store), Some(cached)) = (profile, store.as_ref(), cached) {
        let provider = CachedSessionCredentialsProvider {
            credentials: Mutex::new(cached),
            fallback: base_config.credentials_provider(),
            profile: profile.to_owned(),
            store: store.clone(),
        };
        return Ok(build_loader().credentials_provider(provider).load().await);
    }

    if let Some(profile) = profile
        && let Some(provider) = base_config.credentials_provider()
    {
        let credentials = provider.provide_credentials().await.with_context(|| {
            format!("could not resolve AWS credentials for profile '{profile}'")
        })?;
        if let Some(store) = store {
            store.save_profile(profile, &credentials)?;
        }
    }
    Ok(base_config)
}

struct CachedSessionCredentialsProvider {
    credentials: Mutex<Credentials>,
    fallback: Option<SharedCredentialsProvider>,
    profile: String,
    store: CredentialStore,
}

impl fmt::Debug for CachedSessionCredentialsProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CachedSessionCredentialsProvider")
            .field("profile", &self.profile)
            .finish_non_exhaustive()
    }
}

impl CachedSessionCredentialsProvider {
    async fn load(&self) -> aws_credential_types::provider::Result {
        let mut credentials = self.credentials.lock().await;
        if credentials
            .expiry()
            .is_none_or(|expiration| expiration > SystemTime::now() + CREDENTIAL_REFRESH_WINDOW)
        {
            return Ok(credentials.clone());
        }

        let fallback = self.fallback.as_ref().ok_or_else(|| {
            CredentialsError::provider_error(std::io::Error::other(format!(
                "cached credentials for profile '{}' expired and no AWS profile provider is available",
                self.profile
            )))
        })?;
        let refreshed = fallback.provide_credentials().await?;
        self.store
            .save_profile(&self.profile, &refreshed)
            .map_err(|error| {
                CredentialsError::provider_error(std::io::Error::other(format!(
                    "could not update cached credentials for profile '{}': {error:#}",
                    self.profile
                )))
            })?;
        *credentials = refreshed.clone();
        Ok(refreshed)
    }
}

impl ProvideCredentials for CachedSessionCredentialsProvider {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        future::ProvideCredentials::new(self.load())
    }
}

fn parse_expiration(value: &str) -> Result<SystemTime> {
    let date_time = DateTime::from_str(value, Format::DateTimeWithOffset)
        .context("invalid RFC 3339 expiration timestamp")?;
    SystemTime::try_from(date_time).context("expiration timestamp is outside the system time range")
}

fn format_expiration(value: SystemTime) -> Result<String> {
    DateTime::from(value)
        .fmt(Format::DateTimeWithOffset)
        .context("could not format credential expiration timestamp")
}

#[cfg(test)]
mod tests {
    use super::{CredentialStore, CredentialsFile, StoredCredentials};
    use std::{collections::HashMap, env, fs};

    #[test]
    fn removes_expired_profiles_when_reading_credentials() -> anyhow::Result<()> {
        let directory = env::temp_dir().join(format!(
            "cloudshell-credentials-test-{}",
            uuid::Uuid::new_v4()
        ));
        let store = CredentialStore {
            path: directory.join("credentials"),
        };
        store.write(&CredentialsFile {
            profiles: HashMap::from([
                (
                    "expired".to_owned(),
                    StoredCredentials {
                        aws_access_key_id: "old-key".to_owned(),
                        aws_secret_access_key: "old-secret".to_owned(),
                        aws_session_token: "old-token".to_owned(),
                        aws_expiration: "2000-01-01T00:00:00.000Z".to_owned(),
                    },
                ),
                (
                    "valid".to_owned(),
                    StoredCredentials {
                        aws_access_key_id: "valid-key".to_owned(),
                        aws_secret_access_key: "valid-secret".to_owned(),
                        aws_session_token: "valid-token".to_owned(),
                        aws_expiration: "2099-01-01T00:00:00.000Z".to_owned(),
                    },
                ),
            ]),
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&store.path)?.permissions();
            permissions.set_mode(0o644);
            fs::set_permissions(&store.path, permissions)?;
        }
        let credentials = store.load_profile("valid")?;
        assert!(credentials.is_some());
        let file = store.read_and_prune()?;
        assert!(!file.profiles.contains_key("expired"));
        assert!(file.profiles.contains_key("valid"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&store.path)?.permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_dir_all(directory)?;
        Ok(())
    }
}
