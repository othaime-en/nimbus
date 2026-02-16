use crate::core::{Action, CloudResource, Provider, ResourceState, ResourceType};
use aws_sdk_rds::types::DbInstance;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct RDSInstance {
    db_instance_id: String,
    name: String,
    engine: String,
    engine_version: String,
    instance_class: String,
    state: String,
    region: String,
    tags: HashMap<String, String>,
    created_at: Option<DateTime<Utc>>,
    endpoint: Option<String>,
    port: Option<i32>,
    storage_gb: Option<i32>,
    multi_az: bool,
}

impl RDSInstance {
    pub fn from_aws_instance(instance: &DbInstance, region: &str) -> Self {
        let tags: HashMap<String, String> = instance
            .tag_list()
            .iter()
            .filter_map(|tag| {
                if let (Some(key), Some(value)) = (tag.key(), tag.value()) {
                    Some((key.to_string(), value.to_string()))
                } else {
                    None
                }
            })
            .collect();

        let name = tags
            .get("Name")
            .cloned()
            .or_else(|| instance.db_instance_identifier().map(|s| s.to_string()))
            .unwrap_or_else(|| "Unknown".to_string());

        let db_instance_id = instance
            .db_instance_identifier()
            .unwrap_or("")
            .to_string();

        let engine = instance.engine().unwrap_or("unknown").to_string();
        let engine_version = instance.engine_version().unwrap_or("").to_string();
        let instance_class = instance.db_instance_class().unwrap_or("").to_string();
        let state = instance.db_instance_status().unwrap_or("unknown").to_string();

        let created_at = instance.instance_create_time().and_then(|dt| {
            DateTime::parse_from_rfc3339(&dt.to_string())
                .ok()
                .map(|parsed| parsed.with_timezone(&Utc))
        });

        let endpoint = instance
            .endpoint()
            .and_then(|e| e.address())
            .map(|a| a.to_string());

        let port = instance.endpoint().and_then(|e| e.port());

        let storage_gb = instance.allocated_storage();
        let multi_az = instance.multi_az().unwrap_or(false);

        Self {
            db_instance_id,
            name,
            engine,
            engine_version,
            instance_class,
            state,
            region: region.to_string(),
            tags,
            created_at,
            endpoint,
            port,
            storage_gb,
            multi_az,
        }
    }

    pub fn engine(&self) -> &str {
        &self.engine
    }

    pub fn engine_version(&self) -> &str {
        &self.engine_version
    }

    pub fn instance_class(&self) -> &str {
        &self.instance_class
    }

    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    pub fn port(&self) -> Option<i32> {
        self.port
    }

    pub fn storage_gb(&self) -> Option<i32> {
        self.storage_gb
    }

    pub fn is_multi_az(&self) -> bool {
        self.multi_az
    }
}

impl CloudResource for RDSInstance {
    fn id(&self) -> &str {
        &self.db_instance_id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn resource_type(&self) -> ResourceType {
        ResourceType::Database
    }

    fn provider(&self) -> Provider {
        Provider::AWS
    }

    fn region(&self) -> &str {
        &self.region
    }

    fn state(&self) -> ResourceState {
        match self.state.as_str() {
            "available" => ResourceState::Running,
            "stopped" => ResourceState::Stopped,
            "stopping" => ResourceState::Stopping,
            "starting" => ResourceState::Starting,
            "creating" => ResourceState::Pending,
            "deleting" => ResourceState::Stopping,
            "failed" | "inaccessible-encryption-credentials" => ResourceState::Error,
            _ => ResourceState::Unknown,
        }
    }

    fn cost_per_month(&self) -> Option<f64> {
        Some(estimate_rds_cost(&self.instance_class, self.storage_gb, self.multi_az))
    }

    fn tags(&self) -> &HashMap<String, String> {
        &self.tags
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
            ResourceState::Pending | ResourceState::Stopping | ResourceState::Starting => {
                vec![Action::ViewDetails]
            }
            _ => vec![Action::ViewDetails],
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn estimate_rds_cost(instance_class: &str, storage_gb: Option<i32>, multi_az: bool) -> f64 {
    let base_cost = match instance_class {
        c if c.contains("db.t3.micro") => 14.60,
        c if c.contains("db.t3.small") => 29.20,
        c if c.contains("db.t3.medium") => 58.40,
        c if c.contains("db.t2.micro") => 16.79,
        c if c.contains("db.t2.small") => 33.58,
        c if c.contains("db.t2.medium") => 67.16,
        c if c.contains("db.m5.large") => 131.40,
        c if c.contains("db.m5.xlarge") => 262.80,
        c if c.contains("db.r5.large") => 175.20,
        c if c.contains("db.r5.xlarge") => 350.40,
        _ => 100.0,
    };

    let storage_cost = storage_gb.map(|gb| gb as f64 * 0.115).unwrap_or(0.0);

    let total = base_cost + storage_cost;

    if multi_az {
        total * 2.0
    } else {
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_rds_cost() {
        assert_eq!(estimate_rds_cost("db.t3.micro", None, false), 14.60);
        assert_eq!(estimate_rds_cost("db.t3.micro", Some(100), false), 14.60 + 11.5);
        assert_eq!(
            estimate_rds_cost("db.t3.micro", Some(100), true),
            (14.60 + 11.5) * 2.0
        );
    }

    #[test]
    fn test_rds_state_mapping() {
        let instance = RDSInstance {
            db_instance_id: "db-123".to_string(),
            name: "test-db".to_string(),
            engine: "postgres".to_string(),
            engine_version: "14.7".to_string(),
            instance_class: "db.t3.micro".to_string(),
            state: "available".to_string(),
            region: "us-east-1".to_string(),
            tags: HashMap::new(),
            created_at: None,
            endpoint: None,
            port: None,
            storage_gb: None,
            multi_az: false,
        };

        assert_eq!(instance.state(), ResourceState::Running);
        assert_eq!(instance.resource_type(), ResourceType::Database);
    }

    #[test]
    fn test_rds_supported_actions() {
        let mut instance = RDSInstance {
            db_instance_id: "db-123".to_string(),
            name: "test-db".to_string(),
            engine: "postgres".to_string(),
            engine_version: "14.7".to_string(),
            instance_class: "db.t3.micro".to_string(),
            state: "available".to_string(),
            region: "us-east-1".to_string(),
            tags: HashMap::new(),
            created_at: None,
            endpoint: None,
            port: None,
            storage_gb: None,
            multi_az: false,
        };

        let actions = instance.supported_actions();
        assert!(actions.contains(&Action::Stop));
        assert!(actions.contains(&Action::Restart));

        instance.state = "stopped".to_string();
        let actions = instance.supported_actions();
        assert!(actions.contains(&Action::Start));
        assert!(!actions.contains(&Action::Stop));
    }
}