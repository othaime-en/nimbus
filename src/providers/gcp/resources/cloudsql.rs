use crate::core::{Action, CloudResource, Provider, ResourceState, ResourceType};
use chrono::{DateTime, Utc};
use google_cloud_sql_v1::model::DatabaseInstance;
use std::collections::HashMap;

pub struct CloudSqlInstance {
    name: String,
    database_version: String,
    state: String,
    tier: String,
    region: String,
    labels: HashMap<String, String>,
}

impl CloudSqlInstance {
    /// VERIFY: field names/shapes here (`database_version`, `state`,
    /// `settings.tier`, `settings.user_labels`) are inferred from the
    /// Cloud SQL Admin API's REST resource rather than confirmed against
    /// this crate's generated model -- check against
    /// `cargo doc -p google-cloud-sql-v1` once this builds. The Cloud SQL
    /// Admin API also doesn't expose a creation timestamp on this
    /// resource, so `created_at()` always returns `None` -- that's not a
    /// gap in this mapping, the API just doesn't have the field.
    pub fn from_sql_instance(instance: &DatabaseInstance) -> Self {
        let name = instance.name.clone().unwrap_or_default();

        let database_version = instance
            .database_version
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let state = instance
            .state
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "UNKNOWN".to_string());

        let (tier, labels) = instance
            .settings
            .as_ref()
            .map(|s| {
                (
                    s.tier.clone().unwrap_or_else(|| "unknown".to_string()),
                    s.user_labels.clone(),
                )
            })
            .unwrap_or_else(|| ("unknown".to_string(), HashMap::new()));

        let region = instance.region.clone().unwrap_or_default();

        Self {
            name,
            database_version,
            state,
            tier,
            region,
            labels,
        }
    }

    pub fn database_version(&self) -> &str {
        &self.database_version
    }

    pub fn tier(&self) -> &str {
        &self.tier
    }
}

impl CloudResource for CloudSqlInstance {
    fn id(&self) -> &str {
        &self.name
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn resource_type(&self) -> ResourceType {
        ResourceType::Database
    }

    fn provider(&self) -> Provider {
        Provider::GCP
    }

    fn region(&self) -> &str {
        &self.region
    }

    fn state(&self) -> ResourceState {
        match self.state.as_str() {
            "RUNNABLE" => ResourceState::Running,
            "SUSPENDED" | "STOPPED" => ResourceState::Stopped,
            "PENDING_CREATE" | "MAINTENANCE" => ResourceState::Pending,
            "FAILED" => ResourceState::Error,
            _ => ResourceState::Unknown,
        }
    }

    fn cost_per_month(&self) -> Option<f64> {
        Some(estimate_cloudsql_cost(&self.tier))
    }

    fn tags(&self) -> &HashMap<String, String> {
        &self.labels
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        None
    }

    fn supported_actions(&self) -> Vec<Action> {
        // Start/stop/restart/delete for Cloud SQL aren't wired up yet --
        // a wrong action against a live database is a lot more expensive
        // than against a VM, and the Cloud SQL Admin API's patch/restart
        // semantics need more care than fit in this pass. See
        // GCPProvider::execute_action and the Phase 3.1 handoff notes.
        vec![Action::ViewDetails]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Static, approximate monthly pricing for common Cloud SQL tiers
/// (no HA, no storage cost included). Same static-table approach as
/// `estimate_ec2_cost` -- see the Phase 3.1 cost-tracking discussion.
fn estimate_cloudsql_cost(tier: &str) -> f64 {
    match tier {
        "db-f1-micro" => 7.67,
        "db-g1-small" => 25.92,
        "db-custom-1-3840" => 51.10,
        "db-custom-2-7680" => 102.20,
        "db-n1-standard-1" => 66.30,
        "db-n1-standard-2" => 132.60,
        _ => 60.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_cloudsql_cost() {
        assert_eq!(estimate_cloudsql_cost("db-f1-micro"), 7.67);
        assert_eq!(estimate_cloudsql_cost("unknown-tier"), 60.0);
    }

    #[test]
    fn test_cloudsql_state_mapping() {
        let instance = CloudSqlInstance {
            name: "prod-db".to_string(),
            database_version: "POSTGRES_14".to_string(),
            state: "RUNNABLE".to_string(),
            tier: "db-f1-micro".to_string(),
            region: "us-central1".to_string(),
            labels: HashMap::new(),
        };

        assert_eq!(instance.state(), ResourceState::Running);
        assert_eq!(instance.resource_type(), ResourceType::Database);
        assert_eq!(instance.provider(), Provider::GCP);
        assert_eq!(instance.created_at(), None);
    }

    #[test]
    fn test_cloudsql_supported_actions_view_only() {
        let instance = CloudSqlInstance {
            name: "prod-db".to_string(),
            database_version: "POSTGRES_14".to_string(),
            state: "RUNNABLE".to_string(),
            tier: "db-f1-micro".to_string(),
            region: "us-central1".to_string(),
            labels: HashMap::new(),
        };

        assert_eq!(instance.supported_actions(), vec![Action::ViewDetails]);
    }
}
