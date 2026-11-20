use anyhow::{Context, Result, bail};
use aws_credential_types::{Credentials, provider::ProvideCredentials};
use aws_sigv4::http_request::{
    SignableBody, SignableRequest, SigningParams, SigningSettings, sign,
};
use aws_smithy_runtime_api::client::identity::Identity;
use serde::{Serialize, de::DeserializeOwned};
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;

const SIGNING_NAME: &str = "cloudshell";
const CREDENTIAL_REFRESH_WINDOW: Duration = Duration::from_secs(60);

pub(crate) struct CloudShellTransport {
    aws_config: aws_config::SdkConfig,
    credentials: Mutex<Option<Credentials>>,
    http: reqwest::Client,
    region: String,
}

impl CloudShellTransport {
    pub(crate) async fn new(profile: Option<&str>, region: Option<&str>) -> Result<Self> {
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
        if let Some(profile) = profile {
            loader = loader.profile_name(profile);
        }
        if let Some(region) = region {
            loader = loader.region(aws_config::Region::new(region.to_owned()));
        }

        let aws_config = loader.load().await;
        Self::from_aws_config(aws_config)
    }

    pub(crate) fn from_aws_config(aws_config: aws_config::SdkConfig) -> Result<Self> {
        let region = aws_config
            .region()
            .context("no AWS region configured; pass --region or configure a default region")?
            .to_string();

        Ok(Self {
            aws_config,
            credentials: Mutex::new(None),
            http: reqwest::Client::new(),
            region,
        })
    }

    pub(crate) fn region(&self) -> &str {
        &self.region
    }

    pub(crate) async fn request_json<T: DeserializeOwned>(
        &self,
        operation: &str,
        payload: &impl Serialize,
    ) -> Result<T> {
        let body = self.request(operation, payload).await?;
        serde_json::from_slice(&body)
            .with_context(|| format!("could not parse CloudShell {operation} response"))
    }

    pub(crate) async fn request_empty(
        &self,
        operation: &str,
        payload: &impl Serialize,
    ) -> Result<()> {
        self.request(operation, payload).await?;
        Ok(())
    }

    async fn request(&self, operation: &str, payload: &impl Serialize) -> Result<Vec<u8>> {
        let body = serde_json::to_vec(payload)?;
        let url = self.endpoint(operation)?;
        let credentials = self.credentials().await?;
        let identity: Identity = credentials.into();

        let signable_request = SignableRequest::new(
            "POST",
            url.as_str(),
            [("content-type", "application/json")].into_iter(),
            SignableBody::Bytes(&body),
        )?;
        let signing_params: SigningParams<'_> = aws_sigv4::sign::v4::SigningParams::builder()
            .identity(&identity)
            .region(&self.region)
            .name(SIGNING_NAME)
            .time(SystemTime::now())
            .settings(SigningSettings::default())
            .build()?
            .into();

        let (instructions, _) = sign(signable_request, &signing_params)?.into_parts();
        let mut signed_request = http::Request::builder()
            .method("POST")
            .uri(url.as_str())
            .header("content-type", "application/json")
            .body(body.clone())?;
        instructions.apply_to_request_http1x(&mut signed_request);

        let response = self
            .http
            .post(url)
            .headers(signed_request.headers().clone())
            .body(body)
            .send()
            .await?;
        let status = response.status();
        let response_body = response.bytes().await?;
        if !status.is_success() {
            let details = String::from_utf8_lossy(&response_body);
            bail!("CloudShell {operation} failed with {status}: {details}");
        }
        Ok(response_body.to_vec())
    }

    async fn credentials(&self) -> Result<Credentials> {
        let mut cached = self.credentials.lock().await;
        if let Some(credentials) = cached.as_ref() {
            match credentials.expiry() {
                None => return Ok(credentials.clone()),
                Some(expiry) if expiry > SystemTime::now() + CREDENTIAL_REFRESH_WINDOW => {
                    return Ok(credentials.clone());
                }
                Some(_) => {}
            }
        }

        let provider = self
            .aws_config
            .credentials_provider()
            .context("no AWS credentials provider is configured")?;
        let credentials = provider.provide_credentials().await?;
        *cached = Some(credentials.clone());
        Ok(credentials)
    }

    fn endpoint(&self, operation: &str) -> Result<reqwest::Url> {
        let suffix = if self.region.starts_with("cn-") {
            "amazonaws.com.cn"
        } else {
            "amazonaws.com"
        };
        Ok(reqwest::Url::parse(&format!(
            "https://cloudshell.{}.{}/{operation}",
            self.region, suffix
        ))?)
    }
}
