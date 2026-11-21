+++
title = "Rust library"
description = "Use CloudShellClient from an asynchronous Rust application."
weight = 4
+++

The crate exposes `CloudShellClient` for environment operations and connection management, plus `VpcLookup` for network discovery. The library uses the AWS SDK configuration and credential providers.

## Create a client

Construct a client from an optional AWS profile and region:

```rust,no_run
use cloudshell::CloudShellClient;

# async fn example() -> anyhow::Result<()> {
let client = CloudShellClient::new(Some("sandbox"), Some("eu-west-1")).await?;
for environment in client.list_environments().await? {
    println!("{}: {:?}", environment.environment_id, environment.status);
}
# Ok(())
# }
```

If the calling application already loaded AWS settings, use `CloudShellClient::new_with_aws_config` instead.

## Available operations

`CloudShellClient` supports listing, creating, and deleting environments; checking status; starting and stopping environments; waiting for an environment to reach `RUNNING`; and opening an interactive connection. Creation options can include a name and a `VpcConfig`.

`connect` requires an interactive terminal. It starts suspended environments, waits for readiness, creates a session, and cleans up the session when the interactive shell exits.

## VPC discovery

`VpcLookup::new` accepts an optional profile and region, and `VpcLookup::new_with_aws_config` accepts an existing SDK config. `list_vpcs` returns VPC CIDR blocks along with each VPC's subnet and security group details. A region must be configured.

See [VPC networking](@/docs/networking.md) for CLI usage and the relevant EC2 describe permissions.
