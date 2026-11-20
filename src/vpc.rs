use anyhow::{Context, Result};
use aws_sdk_ec2::{Client, types::Filter};

#[derive(Debug, Clone)]
pub struct VpcDetails {
    pub vpc_id: String,
    pub cidr_block: Option<String>,
    pub subnets: Vec<SubnetDetails>,
    pub security_groups: Vec<SecurityGroupDetails>,
}

#[derive(Debug, Clone)]
pub struct SubnetDetails {
    pub subnet_id: String,
    pub cidr_block: Option<String>,
    pub availability_zone: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SecurityGroupDetails {
    pub group_id: String,
    pub group_name: Option<String>,
    pub description: Option<String>,
}

pub struct VpcLookup {
    client: Client,
}

impl VpcLookup {
    pub async fn new(profile: Option<&str>, region: Option<&str>) -> Result<Self> {
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
        if let Some(profile) = profile {
            loader = loader.profile_name(profile);
        }
        if let Some(region) = region {
            loader = loader.region(aws_config::Region::new(region.to_owned()));
        }

        let config = loader.load().await;
        Self::new_with_aws_config(config)
    }

    pub fn new_with_aws_config(config: aws_config::SdkConfig) -> Result<Self> {
        config
            .region()
            .context("no AWS region configured; pass --region or configure a default region")?;

        Ok(Self {
            client: Client::new(&config),
        })
    }

    pub async fn list_vpcs(&self) -> Result<Vec<VpcDetails>> {
        let mut vpcs = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self.client.describe_vpcs();
            if let Some(token) = next_token.as_deref() {
                request = request.next_token(token);
            }
            let response = request.send().await.context("could not describe VPCs")?;
            vpcs.extend(response.vpcs().iter().cloned());
            next_token = response.next_token().map(str::to_owned);
            if next_token.is_none() {
                break;
            }
        }

        let mut details = Vec::with_capacity(vpcs.len());
        for vpc in vpcs {
            let Some(vpc_id) = vpc.vpc_id().map(str::to_owned) else {
                continue;
            };
            details.push(VpcDetails {
                subnets: self.list_subnets(&vpc_id).await?,
                security_groups: self.list_security_groups(&vpc_id).await?,
                vpc_id,
                cidr_block: vpc.cidr_block().map(str::to_owned),
            });
        }
        Ok(details)
    }

    async fn list_subnets(&self, vpc_id: &str) -> Result<Vec<SubnetDetails>> {
        let mut subnets = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self
                .client
                .describe_subnets()
                .filters(Filter::builder().name("vpc-id").values(vpc_id).build());
            if let Some(token) = next_token.as_deref() {
                request = request.next_token(token);
            }
            let response = request
                .send()
                .await
                .with_context(|| format!("could not describe subnets for VPC {vpc_id}"))?;
            subnets.extend(response.subnets().iter().map(|subnet| SubnetDetails {
                subnet_id: subnet.subnet_id().unwrap_or_default().to_owned(),
                cidr_block: subnet.cidr_block().map(str::to_owned),
                availability_zone: subnet.availability_zone().map(str::to_owned),
            }));
            next_token = response.next_token().map(str::to_owned);
            if next_token.is_none() {
                break;
            }
        }
        Ok(subnets)
    }

    async fn list_security_groups(&self, vpc_id: &str) -> Result<Vec<SecurityGroupDetails>> {
        let mut security_groups = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let mut request = self
                .client
                .describe_security_groups()
                .filters(Filter::builder().name("vpc-id").values(vpc_id).build());
            if let Some(token) = next_token.as_deref() {
                request = request.next_token(token);
            }
            let response = request
                .send()
                .await
                .with_context(|| format!("could not describe security groups for VPC {vpc_id}"))?;
            security_groups.extend(response.security_groups().iter().map(|group| {
                SecurityGroupDetails {
                    group_id: group.group_id().unwrap_or_default().to_owned(),
                    group_name: group.group_name().map(str::to_owned),
                    description: group.description().map(str::to_owned),
                }
            }));
            next_token = response.next_token().map(str::to_owned);
            if next_token.is_none() {
                break;
            }
        }
        Ok(security_groups)
    }
}
