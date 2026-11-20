# cloudshell

An unofficial Rust library and CLI for AWS CloudShell. 


CloudShell has no public AWS SDK API so may change this API without notice. Please use this library and CLI with caution, don't rely on it for real workloads. The code is quite messy, probably has a bunch of bugs and I'm not actively managing versioning at the moment.

You have been warned!

## Requirements

- Rust 1.94.1 or later
- AWS credentials configured through the standard AWS CLI

The Rust Session Manager bridge handles the interactive data channel directly;
`session-manager-plugin` is not required.

## Library

```rust,no_run
use cloudshell::CloudShellClient;

# async fn example() -> anyhow::Result<()> {
let client = CloudShellClient::new(Some("my-profile"), Some("eu-west-1")).await?;
for environment in client.list_environments().await? {
    println!("{}: {:?}", environment.environment_id, environment.status);
}
# Ok(())
# }
```

The library provides environment listing, creation (including VPC configuration), VPC/subnet/security-group discovery, status, start, stop, deletion, readiness polling, and interactive connection. IAM permissions for these operations must be granted to the selected AWS identity.

## How to build

Make sure to have the rust toolchain installed. You can install it using [rustup](https://rustup.rs/).

To build the project, clone the repository and run:

```bash
cargo build --release
```

You will find the 'cloudshell' binary in the `target/release` directory.

## Example CLI usage

```bash
# List available environments
cloudshell list

# Create a new environment with VPC configuration
cloudshell create --vpc-id vpc-123 --subnet-ids subnet-123 subnet-456 --security-group-ids sg-123

# Connect to an environment
cloudshell connect --environment-id env-123

# Stop an environment
cloudshell stop --environment-id env-123

# Delete an environment
cloudshell delete --environment-id env-123
```

## Random notes that I'll have to clean up and whip into shape


## CLI

The global `--region` and `--profile` options can appear before or after the
subcommand. `--profile` selects an AWS shared config profile for any command.
`create` also accepts `--name`;
VPC creation requires `--name`, `--vpc-id`, `--subnet-ids`, and
`--security-group-ids`. The ID lists accept comma-separated values. Environment
lifecycle changes are exposed as `start` and `stop`; the API has no separate
update-environment operation.

`create-and-connect` creates a public environment, waits for it to be ready,
then opens an interactive session without leaving the process or its AWS
credential provider. Use `--timeout` to change the readiness wait limit. The
optional `--vpc-id`, `--subnet-id`, and `--security-group-id` must be supplied
together; omit all three to create a public environment without VPC
configuration. CloudShell associates an environment with the identity that
created its session, so creating and connecting must use the same credentials.
The selected profile follows CLI flag, `AWS_PROFILE`/`AWS_DEFAULT_PROFILE`, then
`[default].profile` precedence. Region selection follows the same order using
`--region`, `AWS_REGION`/`AWS_DEFAULT_REGION`, and `[default].region`.

The CLI optionally reads `~/.cloudshell/config.toml`:

```toml
[default]
profile = "sandbox"
region = "eu-west-1"

[profile-sandbox]
vpc_id = "vpc-123"
subnets_ids = ["subnet-123", "subnet-456"]
security-group-ids = ["sg-123"]
```

`[default]` settings apply across commands. A matching `[profile-<name>]` section supplies VPC settings for `create` and `create-and-connect`; command-line VPC values override configured values. CLI flags take precedence over AWS environment variables, which take precedence over `[default]` settings.

When a selected AWS profile resolves temporary assume-role credentials, the CLI caches them in `~/.cloudshell/credentials` as TOML, keyed by profile name. The AWS profile provider is used when no valid cached session exists; valid cached credentials are reused, refreshed credentials replace them, and expired entries are removed whenever the file is read. The CLI creates the credentials file with owner-only permissions on Unix-like systems. Treat it as a secret and do not commit or share it. The cache file is managed by the CLI and normally does not need to be edited manually.

`vpc` lists VPC IDs and CIDR blocks, plus each VPC's subnet IDs, CIDR blocks, availability zones, and security group IDs, names, and descriptions. It uses the EC2 API and requires permission to describe VPCs, subnets, and security groups.

`connect` starts a suspended environment, waits up to 120 seconds for it to become available, then opens an interactive shell. Use `--timeout` to change
that wait limit.
