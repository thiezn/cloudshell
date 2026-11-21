+++
title = "Getting started"
description = "Build CloudShell, configure AWS credentials, and run your first command."
weight = 1
+++

CloudShell is an unofficial Rust library and command-line tool for managing AWS CloudShell environments. It talks to an AWS API that is not publicly documented, so behavior may change without notice. Do not rely on it for production workloads.

## Requirements

- Rust 1.94.1 or later
- AWS credentials configured through the standard AWS CLI credential chain
- An AWS region, supplied with `--region`, in your AWS configuration, or through an environment variable

The built-in Session Manager bridge handles interactive data channels; `session-manager-plugin` is not required.

## Build

Clone the repository and build the release binary:

```bash
git clone https://github.com/thiezn/cloudshell.git
cd cloudshell
cargo build --release
```

The binary is at `target/release/cloudshell`. Confirm the available commands with:

```bash
target/release/cloudshell --help
```

## Configure AWS access

CloudShell uses the AWS SDK credential chain. Configure credentials as you would for the AWS CLI, then select a profile and region when needed:

```bash
cloudshell --profile sandbox --region eu-west-1 list
```

The global options can appear before or after a subcommand. See [Configuration](@/docs/configuration.md) for precedence and the optional `~/.cloudshell/config.toml` file.

## List and connect

List environments, then connect to one by its environment ID:

```bash
cloudshell list
cloudshell connect env-123
```

`connect` starts a suspended environment and waits for it to become available before opening an interactive shell. Use `--timeout` to change the default 120-second wait.

## Create and connect

Create an environment, then connect using the ID printed by `create`:

```bash
environment_id=$(cloudshell create)
cloudshell connect "$environment_id"
```

Refer to the [CLI reference](@/docs/cli.md) for lifecycle commands and [VPC networking](@/docs/networking.md) for creating environments in a VPC.
