use crate::core::{Action, CloudResource, Provider, ResourceState, ResourceType};
use crate::error::{NimbusError, Result};
use crate::providers::gcp::client::GcpClient;
use chrono::{DateTime, Utc};
use google_cloud_compute_v1::model::Instance as GceInstanceModel;
use std::collections::HashMap;

pub struct GceInstance {
    id: String,
    name: String,
    machine_type: String,
    status: String,
    zone: String,
    labels: HashMap<String, String>,
    created_at: Option<DateTime<Utc>>,
}

impl GceInstance {
    /// VERIFY: `instance.status` is a generated proto enum (`Status`) here;
    /// this assumes its `Display` impl prints the same upper-snake-case
    /// strings the REST API uses ("RUNNING", "TERMINATED", etc.) -- check
    /// against `cargo doc -p google-cloud-compute-v1` once this builds and
    /// adjust the match in `state()` if not. `machine_type` and `zone` come
    /// back as full resource URLs (e.g. `.../zones/us-central1-a/machineTypes/e2-medium`)
    /// rather than bare names, so we take the last path segment.
    pub fn from_gce_instance(instance: &GceInstanceModel, zone_hint: &str) -> Self {
        let id = instance
            .id
            .map(|id| id.to_string())
            .unwrap_or_else(|| instance.name.clone().unwrap_or_default());

        let name = instance.name.clone().unwrap_or_else(|| id.clone());

        let machine_type = url_tail(instance.machine_type.as_deref()).unwrap_or_else(|| "unknown".to_string());
        let zone = url_tail(instance.zone.as_deref()).unwrap_or_else(|| zone_hint.to_string());

        let status = instance
            .status
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "UNKNOWN".to_string());

        let created_at = instance
            .creation_timestamp
            .as_deref()
            .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Self {
            id,
            name,
            machine_type,
            status,
            zone,
            labels: instance.labels.clone(),
            created_at,
        }
    }

    pub fn machine_type(&self) -> &str {
        &self.machine_type
    }
}

fn url_tail(url: Option<&str>) -> Option<String> {
    url.and_then(|u| u.rsplit('/').next()).map(|s| s.to_string())
}

impl CloudResource for GceInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn resource_type(&self) -> ResourceType {
        ResourceType::Compute
    }

    fn provider(&self) -> Provider {
        Provider::GCP
    }

    fn region(&self) -> &str {
        &self.zone
    }

    fn state(&self) -> ResourceState {
        match self.status.as_str() {
            "RUNNING" => ResourceState::Running,
            "TERMINATED" | "SUSPENDED" => ResourceState::Stopped,
            "STOPPING" | "SUSPENDING" => ResourceState::Stopping,
            "PROVISIONING" | "STAGING" => ResourceState::Pending,
            "REPAIRING" => ResourceState::Error,
            _ => ResourceState::Unknown,
        }
    }

    fn cost_per_month(&self) -> Option<f64> {
        Some(estimate_gce_cost(&self.machine_type))
    }

    fn tags(&self) -> &HashMap<String, String> {
        &self.labels
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    fn supported_actions(&self) -> Vec<Action> {
        match self.state() {
            ResourceState::Running => vec![
                Action::Stop,
                Action::Restart,
                Action::Terminate,
                Action::ViewDetails,
            ],
            ResourceState::Stopped => vec![Action::Start, Action::Terminate, Action::ViewDetails],
            ResourceState::Pending | ResourceState::Stopping => vec![Action::ViewDetails],
            _ => vec![Action::ViewDetails],
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Compute Engine's list/get/insert/etc. APIs are all scoped to a single
/// zone, but `GcpConfig` exposes one `region` value (e.g. "us-central1").
/// Until a follow-up adds a project-wide `aggregated_list` across zones,
/// GCE resources are read from one representative zone per region --
/// picked here by appending a default zone suffix. A value that already
/// looks like a zone (has the "-a"/"-b"/... suffix, i.e. two dashes
/// instead of one) is passed through unchanged.
pub fn default_zone(region: &str) -> String {
    if region.matches('-').count() >= 2 {
        region.to_string()
    } else {
        format!("{}-a", region)
    }
}

/// Static, approximate on-demand monthly pricing (us-central1, no
/// committed-use or sustained-use discounts) for common GCE machine types.
/// Same static-table approach as `estimate_ec2_cost` in the AWS EC2 module --
/// see the Phase 3.1 cost-tracking discussion for why this isn't live
/// billing data.
fn estimate_gce_cost(machine_type: &str) -> f64 {
    match machine_type {
        "e2-micro" => 6.12,
        "e2-small" => 12.23,
        "e2-medium" => 24.46,
        "e2-standard-2" => 48.92,
        "e2-standard-4" => 97.83,
        "n2-standard-2" => 69.87,
        "n2-standard-4" => 139.75,
        "n1-standard-1" => 24.27,
        "n1-standard-2" => 48.55,
        _ => 50.0,
    }
}

/// Executes a lifecycle action against a GCE instance. `project_id` and
/// `zone` come from the provider's own config/zone derivation (see
/// `GCPProvider::zone`) rather than being encoded into `resource_id`,
/// matching how the AWS side leaves region resolution to the provider.
pub async fn execute_action(
    client: &GcpClient,
    project_id: &str,
    zone: &str,
    resource_id: &str,
    action: Action,
) -> Result<()> {
    match action {
        Action::Start => {
            client
                .instances
                .start()
                .set_project(project_id)
                .set_zone(zone)
                .set_instance(resource_id)
                .send()
                .await
                .map_err(|e| {
                    NimbusError::provider(
                        "GCP",
                        format!("Failed to start instance {}: {}", resource_id, e),
                    )
                })?;
            Ok(())
        }
        Action::Stop => {
            client
                .instances
                .stop()
                .set_project(project_id)
                .set_zone(zone)
                .set_instance(resource_id)
                .send()
                .await
                .map_err(|e| {
                    NimbusError::provider(
                        "GCP",
                        format!("Failed to stop instance {}: {}", resource_id, e),
                    )
                })?;
            Ok(())
        }
        Action::Restart => {
            client
                .instances
                .reset()
                .set_project(project_id)
                .set_zone(zone)
                .set_instance(resource_id)
                .send()
                .await
                .map_err(|e| {
                    NimbusError::provider(
                        "GCP",
                        format!("Failed to restart instance {}: {}", resource_id, e),
                    )
                })?;
            Ok(())
        }
        Action::Terminate => {
            client
                .instances
                .delete()
                .set_project(project_id)
                .set_zone(zone)
                .set_instance(resource_id)
                .send()
                .await
                .map_err(|e| {
                    NimbusError::provider(
                        "GCP",
                        format!("Failed to delete instance {}: {}", resource_id, e),
                    )
                })?;
            Ok(())
        }
        _ => Err(NimbusError::UnsupportedAction(action, ResourceType::Compute)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_zone_appends_suffix_for_region() {
        assert_eq!(default_zone("us-central1"), "us-central1-a");
        assert_eq!(default_zone("europe-west4"), "europe-west4-a");
    }

    #[test]
    fn test_default_zone_passes_through_existing_zone() {
        assert_eq!(default_zone("us-central1-b"), "us-central1-b");
    }

    #[test]
    fn test_estimate_gce_cost() {
        assert_eq!(estimate_gce_cost("e2-micro"), 6.12);
        assert_eq!(estimate_gce_cost("unknown-type"), 50.0);
    }

    #[test]
    fn test_url_tail() {
        assert_eq!(
            url_tail(Some(
                "https://www.googleapis.com/compute/v1/projects/p/zones/us-central1-a/machineTypes/e2-medium"
            )),
            Some("e2-medium".to_string())
        );
        assert_eq!(url_tail(None), None);
    }

    #[test]
    fn test_gce_instance_state_mapping() {
        let instance = GceInstance {
            id: "123".to_string(),
            name: "web-1".to_string(),
            machine_type: "e2-medium".to_string(),
            status: "RUNNING".to_string(),
            zone: "us-central1-a".to_string(),
            labels: HashMap::new(),
            created_at: None,
        };

        assert_eq!(instance.state(), ResourceState::Running);
        assert_eq!(instance.resource_type(), ResourceType::Compute);
        assert_eq!(instance.provider(), Provider::GCP);
    }

    #[test]
    fn test_gce_instance_supported_actions() {
        let mut instance = GceInstance {
            id: "123".to_string(),
            name: "web-1".to_string(),
            machine_type: "e2-medium".to_string(),
            status: "RUNNING".to_string(),
            zone: "us-central1-a".to_string(),
            labels: HashMap::new(),
            created_at: None,
        };

        let actions = instance.supported_actions();
        assert!(actions.contains(&Action::Stop));
        assert!(!actions.contains(&Action::Start));

        instance.status = "TERMINATED".to_string();
        let actions = instance.supported_actions();
        assert!(actions.contains(&Action::Start));
        assert!(!actions.contains(&Action::Stop));
    }
}
