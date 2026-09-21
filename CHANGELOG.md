# Changelog

All notable changes to Nimbus are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Each release's section heading (`## [x.y.z] - date`) must match the git tag
(`vx.y.z`) exactly — the release workflow extracts release notes from this
file by matching that heading.

## [Unreleased]

## [0.1.0] - 2026-09-21

### Added

- Unified TUI for AWS: EC2, RDS, S3, ELB, Route 53
- Dashboard with cost summary, resource counts, and regional breakdown
- Resource detail view with lifecycle actions (start/stop/restart/terminate)
- Confirmation dialogs for destructive actions
- SQLite-backed offline cache with manual refresh (`r`) and clear (`c`)
- Configuration via `~/.nimbus/config.toml` or environment variables

### Known limitations

- No GCP/Azure support yet
- No setup wizard — config file must be created manually
- No cost alerts/thresholds yet
