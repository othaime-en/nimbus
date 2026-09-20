# Nimbus

A terminal-based interface for managing cloud resources. Currently supports AWS; GCP and Azure are planned.

## Overview

Nimbus provides a unified view of your cloud infrastructure directly in the terminal. View, filter, and manage EC2 instances, RDS databases, S3 buckets, load balancers, and Route 53 zones without switching to the AWS console. Multi-cloud support (GCP, Azure) is on the way.

## Installation

```bash
git clone https://github.com/othaime-en/nimbus
cd nimbus
cargo build --release
```

## Configuration

Create `~/.nimbus/config.toml`:

```toml
[providers.aws]
profile = "default"
region = "us-east-1"

[ui]
auto_refresh = true
confirm_destructive_actions = true

[cache]
enabled = true
max_age_hours = 24
```

Alternatively, set environment variables:

```bash
export NIMBUS_AWS_PROFILE=production
export NIMBUS_AWS_REGION=us-west-2
```

## Usage

```bash
nimbus
```

Navigate between cloud providers using Tab or number keys (1-4). Press `/` to filter resources by name, ID, type, state, or region. Press `r` to refresh the resource list.

Press `q` to quit.

## Requirements

- Rust 1.75 or later
- Valid AWS credentials (via AWS CLI configuration or environment variables)

## License

MIT
