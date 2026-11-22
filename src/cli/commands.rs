use super::args::{Cli, Commands};
use super::config::{CliConfig, ProfileSettings};
use anyhow::{Context, Result, bail};
use clap::Parser;
use cloudshell::{CloudShellClient, CreateEnvironmentOptions, VpcConfig, VpcLookup};
use std::time::Duration;
use std::{env, path::PathBuf};

pub(crate) async fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = CliConfig::load()?;
    let profile = resolve_profile(cli.profile, &config);
    let region = resolve_region(cli.region, &config);
    let credentials_path = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".cloudshell/credentials"));
    let aws_config = cloudshell::load_aws_config_with_cache(
        profile.as_deref(),
        region.as_deref(),
        credentials_path.as_deref(),
    )
    .await?;

    match cli.command {
        Commands::List => {
            let client = CloudShellClient::new_with_aws_config(aws_config.clone())?;
            let environments = client.list_environments().await?;
            println!("ENVIRONMENT ID\tSTATUS");
            for environment in environments {
                println!(
                    "{}\t{}",
                    environment.environment_id,
                    environment.status.as_deref().unwrap_or("UNKNOWN")
                );
            }
        }
        Commands::Create {
            name,
            vpc_id,
            subnet_ids,
            security_group_ids,
        } => {
            let client = CloudShellClient::new_with_aws_config(aws_config.clone())?;
            let profile_settings = selected_profile_settings(&config, profile.as_deref());
            let vpc_id = vpc_id.or_else(|| profile_vpc_id(profile_settings));
            let subnet_ids = configured_ids(subnet_ids, profile_settings, |settings| {
                settings.subnet_ids.as_ref()
            });
            let security_group_ids =
                configured_ids(security_group_ids, profile_settings, |settings| {
                    settings.security_group_ids.as_ref()
                });
            let has_vpc_options =
                vpc_id.is_some() || !subnet_ids.is_empty() || !security_group_ids.is_empty();
            let vpc_config = if has_vpc_options {
                if subnet_ids.is_empty() || security_group_ids.is_empty() {
                    bail!(
                        "VPC creation requires --vpc-id, --subnet-ids, and --security-group-ids or matching profile config"
                    );
                }
                Some(VpcConfig {
                    vpc_id: vpc_id.context("VPC creation requires --vpc-id or profile config")?,
                    subnet_ids,
                    security_group_ids,
                })
            } else {
                None
            };
            if vpc_config.is_some() && name.is_none() {
                bail!("VPC creation requires --name");
            }
            let environment = client
                .create_environment(&CreateEnvironmentOptions {
                    environment_name: name,
                    vpc_config,
                })
                .await?;
            println!("{}", environment.environment_id);
        }
        Commands::Vpc => {
            let lookup = VpcLookup::new_with_aws_config(aws_config.clone())?;
            for vpc in lookup.list_vpcs().await? {
                println!(
                    "VPC\t{}\t{}",
                    vpc.vpc_id,
                    vpc.cidr_block.as_deref().unwrap_or("-")
                );
                for subnet in vpc.subnets {
                    println!(
                        "  SUBNET\t{}\t{}\t{}",
                        subnet.subnet_id,
                        subnet.cidr_block.as_deref().unwrap_or("-"),
                        subnet.availability_zone.as_deref().unwrap_or("-")
                    );
                }
                for group in vpc.security_groups {
                    println!(
                        "  SECURITY GROUP\t{}\t{}\t{}",
                        group.group_id,
                        group.group_name.as_deref().unwrap_or("-"),
                        group.description.as_deref().unwrap_or("-")
                    );
                }
            }
        }
        Commands::Status { environment_id } => {
            let client = CloudShellClient::new_with_aws_config(aws_config.clone())?;
            let status = client.get_environment_status(&environment_id).await?;
            println!("{}", status.status);
            if let Some(reason) = status.status_reason {
                println!("{reason}");
            }
        }
        Commands::Start { environment_id } => {
            let client = CloudShellClient::new_with_aws_config(aws_config.clone())?;
            client.start_environment(&environment_id).await?;
            println!("Started {environment_id}");
        }
        Commands::Stop { environment_id } => {
            let client = CloudShellClient::new_with_aws_config(aws_config.clone())?;
            client.stop_environment(&environment_id).await?;
            println!("Stopped {environment_id}");
        }
        Commands::Delete { environment_id } => {
            let client = CloudShellClient::new_with_aws_config(aws_config.clone())?;
            client.delete_environment(&environment_id).await?;
            println!("Deleted {environment_id}");
        }
        Commands::Connect {
            environment_id,
            timeout,
        } => {
            let client = CloudShellClient::new_with_aws_config(aws_config.clone())?;
            client
                .connect(&environment_id, Duration::from_secs(timeout))
                .await?;
        }
    }

    Ok(())
}

fn resolve_profile(cli_profile: Option<String>, config: &CliConfig) -> Option<String> {
    cli_profile
        .or_else(|| env::var("AWS_PROFILE").ok())
        .or_else(|| env::var("AWS_DEFAULT_PROFILE").ok())
        .or_else(|| config.default.profile.clone())
}

fn resolve_region(cli_region: Option<String>, config: &CliConfig) -> Option<String> {
    cli_region
        .or_else(|| env::var("AWS_REGION").ok())
        .or_else(|| env::var("AWS_DEFAULT_REGION").ok())
        .or_else(|| config.default.region.clone())
}

fn selected_profile_settings<'a>(
    config: &'a CliConfig,
    profile: Option<&str>,
) -> Option<&'a ProfileSettings> {
    config.profile_settings(profile.unwrap_or("default"))
}

fn profile_vpc_id(settings: Option<&ProfileSettings>) -> Option<String> {
    settings.and_then(|settings| settings.vpc_id.clone())
}

fn configured_ids<'a>(
    cli_ids: Vec<String>,
    settings: Option<&'a ProfileSettings>,
    configured: impl FnOnce(&'a ProfileSettings) -> Option<&'a Vec<String>>,
) -> Vec<String> {
    if cli_ids.is_empty() {
        settings.and_then(configured).cloned().unwrap_or_default()
    } else {
        cli_ids
    }
}
