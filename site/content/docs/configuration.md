+++
title = "Configuration"
description = "AWS profile and region selection, VPC defaults, and cached credentials."
weight = 3
+++

## Profile and region precedence

For both profile and region, CloudShell uses the first configured value in this order:

1. Command-line option (`--profile` or `--region`)
2. AWS environment variable (`AWS_PROFILE` / `AWS_DEFAULT_PROFILE` or `AWS_REGION` / `AWS_DEFAULT_REGION`)
3. The `[default]` section in `~/.cloudshell/config.toml`
4. The AWS SDK's normal configuration and credential chain

## Optional CLI configuration

The CLI reads `~/.cloudshell/config.toml` when it exists:

```toml
[default]
profile = "sandbox"
region = "eu-west-1"

[profile-sandbox]
vpc_id = "vpc-123"
subnets_ids = ["subnet-123", "subnet-456"]
security-group-ids = ["sg-123"]
```

`[default]` supplies the profile and region defaults. A `[profile-<name>]` section supplies VPC settings to `create`. Command-line VPC values override configured values. VPC creation requires all of the VPC ID, subnet IDs, and security group IDs; `create` also requires `--name` for VPC environments.

The CLI accepts `subnets_ids`, `subnet_ids`, `subnet-ids`, or `subnets-ids` for subnet lists, and `security-group-ids` or `security_group_ids` for security groups.

## Temporary credential cache

When a named AWS profile resolves temporary assume-role credentials, the CLI caches them in `~/.cloudshell/credentials`, keyed by profile name. The file is TOML with one table per profile. For example:

```toml
[sandbox]
aws_access_key_id = "ASIAEXAMPLEACCESSKEY"
aws_secret_access_key = "example-secret-key"
aws_session_token = "example-session-token"
aws_expiration = "2030-01-01T12:00:00Z"

[production]
aws_access_key_id = "ASIAANOTHEREXAMPLEKEY"
aws_secret_access_key = "another-example-secret"
aws_session_token = "another-example-session-token"
aws_expiration = "2030-01-01T13:00:00Z"
```

These values are temporary credentials; never share or commit the file. The CLI manages it, so you normally should not edit it manually. Cached credentials retain the expiration timestamp returned by AWS and are reused while more than 60 seconds remain. When they are closer to expiration, the CLI refreshes them through the AWS profile provider and updates the file. Expired entries are removed when the cache is read.

On Unix-like systems, the CLI restricts the cache file to owner read/write permissions. Treat it as a secret.
