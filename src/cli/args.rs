use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "cloudshell",
    version,
    about = "Manage AWS CloudShell environments"
)]
pub(super) struct Cli {
    #[arg(
        long,
        global = true,
        help = "AWS region; defaults to the profile or environment configuration"
    )]
    pub(super) region: Option<String>,

    #[arg(
        long,
        global = true,
        help = "AWS shared config profile; temporary role credentials are cached"
    )]
    pub(super) profile: Option<String>,

    #[command(subcommand)]
    pub(super) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(super) enum Commands {
    #[command(alias = "ls")]
    List,
    Create {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        vpc_id: Option<String>,
        #[arg(long, value_delimiter = ',')]
        subnet_ids: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        security_group_ids: Vec<String>,
    },
    CreateAndConnect {
        #[arg(long, help = "VPC to create the CloudShell environment in")]
        vpc_id: Option<String>,
        #[arg(long, help = "Subnet to use within the VPC")]
        subnet_id: Option<String>,
        #[arg(long, help = "Security group to attach to the environment")]
        security_group_id: Option<String>,
        #[arg(
            long,
            default_value_t = 120,
            help = "Seconds to wait for the environment to become ready"
        )]
        timeout: u64,
    },
    Vpc,
    Status {
        environment_id: String,
    },
    Start {
        environment_id: String,
    },
    Stop {
        environment_id: String,
    },
    Delete {
        environment_id: String,
    },
    Connect {
        environment_id: String,
        #[arg(
            long,
            default_value_t = 120,
            help = "Seconds to wait for the environment to become ready"
        )]
        timeout: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::Parser;

    #[test]
    fn profile_is_accepted_by_all_commands() -> Result<(), clap::Error> {
        for arguments in [
            &["cloudshell", "list", "--profile", "assumed-role"][..],
            &["cloudshell", "create", "--profile", "assumed-role"][..],
            &[
                "cloudshell",
                "create-and-connect",
                "--profile",
                "assumed-role",
            ][..],
            &["cloudshell", "vpc", "--profile", "assumed-role"][..],
            &[
                "cloudshell",
                "status",
                "env-123",
                "--profile",
                "assumed-role",
            ][..],
            &[
                "cloudshell",
                "start",
                "env-123",
                "--profile",
                "assumed-role",
            ][..],
            &["cloudshell", "stop", "env-123", "--profile", "assumed-role"][..],
            &[
                "cloudshell",
                "delete",
                "env-123",
                "--profile",
                "assumed-role",
            ][..],
            &[
                "cloudshell",
                "connect",
                "env-123",
                "--profile",
                "assumed-role",
            ][..],
            &["cloudshell", "--profile", "assumed-role", "list"][..],
        ] {
            assert!(Cli::try_parse_from(arguments).is_ok(), "{arguments:?}");
        }
        assert!(
            Cli::try_parse_from(["cloudshell", "start", "env-123", "--create-and-connect"])
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn vpc_accepts_a_profile() -> Result<(), clap::Error> {
        let cli = Cli::try_parse_from(["cloudshell", "vpc", "--profile", "network"])?;
        assert!(matches!(cli.command, Commands::Vpc));
        Ok(())
    }

    #[test]
    fn create_and_connect_accepts_options_for_config_merge() {
        for arguments in [
            &["cloudshell", "create-and-connect", "--vpc-id", "vpc-123"][..],
            &[
                "cloudshell",
                "create-and-connect",
                "--subnet-id",
                "subnet-123",
            ][..],
            &[
                "cloudshell",
                "create-and-connect",
                "--security-group-id",
                "sg-123",
            ][..],
            &[
                "cloudshell",
                "create-and-connect",
                "--vpc-id",
                "vpc-123",
                "--subnet-id",
                "subnet-123",
            ][..],
            &[
                "cloudshell",
                "create-and-connect",
                "--vpc-id",
                "vpc-123",
                "--security-group-id",
                "sg-123",
            ][..],
            &[
                "cloudshell",
                "create-and-connect",
                "--subnet-id",
                "subnet-123",
                "--security-group-id",
                "sg-123",
            ][..],
        ] {
            assert!(Cli::try_parse_from(arguments).is_ok(), "{arguments:?}");
        }

        assert!(Cli::try_parse_from(["cloudshell", "create-and-connect"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "cloudshell",
                "create-and-connect",
                "--vpc-id",
                "vpc-123",
                "--subnet-id",
                "subnet-123",
                "--security-group-id",
                "sg-123",
            ])
            .is_ok()
        );
    }
}
