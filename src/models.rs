use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Environment {
    pub environment_id: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnvironmentStatus {
    pub environment_id: Option<String>,
    pub status: String,
    pub status_reason: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateEnvironmentOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpc_config: Option<VpcConfig>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct VpcConfig {
    pub vpc_id: String,
    pub subnet_ids: Vec<String>,
    pub security_group_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct EnvironmentsResponse {
    #[serde(default)]
    pub(crate) environments: Vec<Environment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct SessionResponse {
    pub(crate) session_id: String,
    pub(crate) token_value: String,
    pub(crate) stream_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_environment_options_serialize_to_cloudshell_names()
    -> std::result::Result<(), serde_json::Error> {
        let options = CreateEnvironmentOptions {
            environment_name: Some("dev".to_owned()),
            vpc_config: Some(VpcConfig {
                vpc_id: "vpc-123".to_owned(),
                subnet_ids: vec!["subnet-123".to_owned()],
                security_group_ids: vec!["sg-123".to_owned()],
            }),
        };

        let value = serde_json::to_value(options)?;
        assert_eq!(value["EnvironmentName"], "dev");
        assert_eq!(value["VpcConfig"]["VpcId"], "vpc-123");
        assert_eq!(value["VpcConfig"]["SubnetIds"][0], "subnet-123");
        assert_eq!(value["VpcConfig"]["SecurityGroupIds"][0], "sg-123");
        Ok(())
    }

    #[test]
    fn environment_responses_deserialize_to_rust_fields()
    -> std::result::Result<(), serde_json::Error> {
        let environment: Environment = serde_json::from_value(json!({
            "EnvironmentId": "env-123",
            "Status": "RUNNING"
        }))?;

        assert_eq!(environment.environment_id, "env-123");
        assert_eq!(environment.status.as_deref(), Some("RUNNING"));
        Ok(())
    }
}
