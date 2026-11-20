use crate::{
    models::{
        CreateEnvironmentOptions, Environment, EnvironmentStatus, EnvironmentsResponse,
        SessionResponse,
    },
    transport::CloudShellTransport,
};
use anyhow::{Context, Result, bail};
use aws_ssm_bridge::{InteractiveConfig, InteractiveShell, Session, SessionConfig};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;
use uuid::Uuid;

const POLL_INTERVAL: Duration = Duration::from_secs(3);

pub struct CloudShellClient {
    transport: CloudShellTransport,
}

impl CloudShellClient {
    pub async fn new(profile: Option<&str>, region: Option<&str>) -> Result<Self> {
        Ok(Self {
            transport: CloudShellTransport::new(profile, region).await?,
        })
    }

    pub fn new_with_aws_config(aws_config: aws_config::SdkConfig) -> Result<Self> {
        Ok(Self {
            transport: CloudShellTransport::from_aws_config(aws_config)?,
        })
    }

    pub fn region(&self) -> &str {
        self.transport.region()
    }

    pub async fn list_environments(&self) -> Result<Vec<Environment>> {
        let response: EnvironmentsResponse = self
            .transport
            .request_json("describeEnvironments", &json!({}))
            .await?;
        Ok(response.environments)
    }

    pub async fn create_environment(
        &self,
        options: &CreateEnvironmentOptions,
    ) -> Result<Environment> {
        self.transport
            .request_json("createEnvironment", options)
            .await
    }

    pub async fn create_environment_and_wait_until_running(
        &self,
        timeout: Duration,
    ) -> Result<Environment> {
        self.create_environment_with_options_and_wait_until_running(
            &CreateEnvironmentOptions::default(),
            timeout,
        )
        .await
    }

    pub async fn create_environment_with_options_and_wait_until_running(
        &self,
        options: &CreateEnvironmentOptions,
        timeout: Duration,
    ) -> Result<Environment> {
        let environment = self.create_environment(options).await?;
        self.wait_until_running(&environment.environment_id, timeout)
            .await
            .with_context(|| {
                format!(
                    "environment {} was created but did not become ready",
                    environment.environment_id
                )
            })?;
        Ok(environment)
    }

    pub async fn get_environment_status(&self, environment_id: &str) -> Result<EnvironmentStatus> {
        self.transport
            .request_json(
                "getEnvironmentStatus",
                &json!({ "EnvironmentId": environment_id }),
            )
            .await
    }

    pub async fn start_environment(&self, environment_id: &str) -> Result<()> {
        self.transport
            .request_empty(
                "startEnvironment",
                &json!({ "EnvironmentId": environment_id }),
            )
            .await
    }

    pub async fn stop_environment(&self, environment_id: &str) -> Result<()> {
        self.transport
            .request_empty(
                "stopEnvironment",
                &json!({ "EnvironmentId": environment_id }),
            )
            .await
    }

    pub async fn delete_environment(&self, environment_id: &str) -> Result<()> {
        self.transport
            .request_empty(
                "deleteEnvironment",
                &json!({ "EnvironmentId": environment_id }),
            )
            .await
    }

    pub async fn wait_until_running(&self, environment_id: &str, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        loop {
            let status = self.get_environment_status(environment_id).await?;
            if status.status.eq_ignore_ascii_case("RUNNING") {
                return Ok(());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                bail!(
                    "environment {environment_id} did not reach RUNNING within {timeout:?}; last status: {}",
                    status.status
                );
            }
            tokio::time::sleep(POLL_INTERVAL.min(remaining)).await;
        }
    }

    pub async fn connect(&self, environment_id: &str, timeout: Duration) -> Result<Option<i32>> {
        if !aws_ssm_bridge::terminal::is_terminal() {
            bail!("connect requires an interactive terminal on stdin and stdout");
        }

        let status = self.get_environment_status(environment_id).await?;
        if status.status.eq_ignore_ascii_case("SUSPENDED") {
            self.start_environment(environment_id).await?;
        }
        if !status.status.eq_ignore_ascii_case("RUNNING") {
            self.wait_until_running(environment_id, timeout).await?;
        }

        let session: SessionResponse = self
            .transport
            .request_json(
                "createSession",
                &json!({
                    "EnvironmentId": environment_id,
                    "SessionType": "TMUX",
                    "TabId": Uuid::new_v4().to_string(),
                    "QCliDisabled": true,
                }),
            )
            .await?;

        let run_result = match Session::attach(
            session.session_id.clone(),
            session.stream_url,
            session.token_value,
            SessionConfig::new(environment_id),
        )
        .await
        {
            Ok(session) => {
                let session = Arc::new(session);
                let shell = InteractiveShell::new(InteractiveConfig {
                    region: Some(self.region().to_owned()),
                    ..InteractiveConfig::default()
                });
                let result = shell.attach(Arc::clone(&session)).await;
                let _ = session.terminate().await;
                result.context("interactive CloudShell session failed")
            }
            Err(error) => {
                Err(anyhow::Error::new(error).context("could not attach to CloudShell session"))
            }
        };

        let cleanup_result = self
            .transport
            .request_empty(
                "deleteSession",
                &json!({
                    "EnvironmentId": environment_id,
                    "SessionId": session.session_id,
                }),
            )
            .await;

        match (run_result, cleanup_result) {
            (Ok(exit_code), Ok(())) => Ok(exit_code),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(error.context("could not delete CloudShell session")),
            (Err(error), Err(cleanup_error)) => Err(error.context(format!(
                "CloudShell session cleanup also failed: {cleanup_error}"
            ))),
        }
    }
}
