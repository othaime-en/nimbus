## v0.1.0 — Initial Release

First release of Nimbus. AWS support only - GCP and Azure are planned for v0.2.0.

### Features

- Unified TUI for AWS: EC2, RDS, S3, ELB, Route 53
- Dashboard with cost summary, resource counts, regional breakdown
- Resource detail view with lifecycle actions (start/stop/restart/terminate)
- Confirmation dialogs for destructive actions
- SQLite-backed offline cache with manual refresh (`r`) and clear (`c`)
- Config via `~/.nimbus/config.toml` or environment variables

### Known limitations

- No GCP/Azure support yet
- No setup wizard - config file must be created manually
- No cost alerts/thresholds yet
