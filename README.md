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

## Caution

When `--profile` selects an assume-role profile, the CLI assumes that role.
CloudShell environments are tied to the identity that created them, so reconnect
using the same profile. Temporary credentials are cached in
`~/.cloudshell/credentials` and refreshed shortly before AWS's expiration time.
