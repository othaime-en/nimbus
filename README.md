# Nimbus

A terminal-based interface for managing cloud resources across AWS, GCP, and Azure.

## Overview

Nimbus provides a unified view of your cloud infrastructure directly in the terminal. View, filter, and manage compute instances, databases, storage, and other cloud resources without switching between provider consoles.

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

## Current Status

Currently supports AWS EC2 instances with filtering and navigation. Additional AWS resources (RDS, S3, ELB, Route53) and GCP/Azure support coming soon.

## License

MIT
