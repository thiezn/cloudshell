+++
title = "VPC networking"
description = "Discover AWS network resources and create CloudShell environments in a VPC."
weight = 5
+++

## Discover network resources

Use the `vpc` command to list VPCs in the selected region, including subnet IDs, CIDR blocks, availability zones, and security group IDs and names:

```bash
cloudshell vpc --region eu-west-1
```

This command uses the EC2 API to describe VPCs, subnets, and security groups. The selected AWS identity needs permission to perform those describe operations.

## Create in a VPC

Choose a VPC, a subnet in that VPC, and a security group, then create the environment and connect to its ID:

```bash
environment_id=$(cloudshell create --name dev-shell \
  --vpc-id vpc-123 \
  --subnet-ids subnet-123 \
  --security-group-ids sg-123)
cloudshell connect "$environment_id"
```

All three VPC settings must be supplied together. See the [CLI reference](@/docs/cli.md) for details. Values can also be set in the matching profile section of `~/.cloudshell/config.toml`.

## Public environments

Omit VPC options to create a public CloudShell environment. Public environments do not require VPC discovery permissions.
