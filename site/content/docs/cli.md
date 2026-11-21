+++
title = "CLI reference"
description = "Commands and options for the cloudshell executable."
weight = 2
+++

## Global options

| Option | Description |
| --- | --- |
| `--profile <PROFILE>` | AWS shared config profile. Temporary role credentials may be cached locally. |
| `--region <REGION>` | AWS region. Defaults to the selected profile or environment configuration. |
| `--help` | Show help for the command or a subcommand. |

Global options can appear before or after the subcommand. For example, `cloudshell --region eu-west-1 list` and `cloudshell list --region eu-west-1` are both valid.

## Commands

| Command | Purpose |
| --- | --- |
| `list` | List environments and their statuses. `ls` is an alias. |
| `create` | Create an environment. Accepts `--name` and optional VPC settings. |
| `vpc` | List VPCs, subnets, and security groups in the selected region. |
| `status <ENVIRONMENT_ID>` | Show an environment's status and any status reason. |
| `start <ENVIRONMENT_ID>` | Start an environment. |
| `stop <ENVIRONMENT_ID>` | Stop an environment. |
| `delete <ENVIRONMENT_ID>` | Delete an environment. |
| `connect <ENVIRONMENT_ID>` | Start if suspended, wait until ready, and open an interactive shell. |

## Create an environment

A public environment can be created without extra options:

```bash
cloudshell create
```

To create an environment in a VPC, include a name, VPC ID, one or more subnets, and one or more security groups. List options accept comma-separated values:

```bash
cloudshell create \
  --name dev-shell \
  --vpc-id vpc-123 \
  --subnet-ids subnet-123,subnet-456 \
  --security-group-ids sg-123
```

The same values can be stored in a matching profile section; see [Configuration](@/docs/configuration.md).

## Connection timeout

`connect` waits up to 120 seconds by default. Change the limit with `--timeout`:

```bash
cloudshell connect env-123 --timeout 240
```

`connect` requires an interactive terminal on standard input and output.
