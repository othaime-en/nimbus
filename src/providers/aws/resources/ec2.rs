use crate::core::{Action, CloudResource, Provider, ResourceState, ResourceType};
use aws_sdk_ec2::types::Instance as Ec2Instance;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct EC2Instance {
    instance_id: String,
    name: String,
    instance_type: String,
    state: String,
    region: String,
    tags: HashMap<String, String>,
    launch_time: Option<DateTime<Utc>>,
    public_ip: Option<String>,
    private_ip: Option<String>,
}

impl EC2Instance {
    pub fn from_aws_instance(instance: &Ec2Instance, region: &str) -> Self {
        let tags: HashMap<String, String> = instance
            .tags()
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
            .unwrap_or_else(|| instance.instance_id().unwrap_or("Unknown").to_string());

        let instance_id = instance.instance_id().unwrap_or("").to_string();
        let instance_type = instance
            .instance_type()
            .map(|t| t.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let state = instance
            .state()
            .and_then(|s| s.name())
            .map(|n| n.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let launch_time = instance.launch_time().and_then(|lt| {
            DateTime::parse_from_rfc3339(&lt.to_string())
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        let public_ip = instance
            .public_ip_address()
            .map(|ip| ip.to_string());

        let private_ip = instance
            .private_ip_address()
            .map(|ip| ip.to_string());

        Self {
            instance_id,
            name,
            instance_type,
            state,
            region: region.to_string(),
            tags,
            launch_time,
            public_ip,
            private_ip,
        }
    }

    pub fn instance_type(&self) -> &str {
        &self.instance_type
    }

    pub fn public_ip(&self) -> Option<&str> {
        self.public_ip.as_deref()
    }

    pub fn private_ip(&self) -> Option<&str> {
        self.private_ip.as_deref()
    }
}

impl CloudResource for EC2Instance {
    fn id(&self) -> &str {
        &self.instance_id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn resource_type(&self) -> ResourceType {
        ResourceType::Compute
    }

    fn provider(&self) -> Provider {
        Provider::AWS
    }

    fn region(&self) -> &str {
        &self.region
    }

    fn state(&self) -> ResourceState {
        match self.state.as_str() {
            "running" => ResourceState::Running,
            "stopped" => ResourceState::Stopped,
            "terminated" => ResourceState::Terminated,
            "pending" => ResourceState::Pending,
            "stopping" => ResourceState::Stopping,
            "shutting-down" => ResourceState::Stopping,
            _ => ResourceState::Unknown,
        }
    }

    fn cost_per_month(&self) -> Option<f64> {
        Some(estimate_ec2_cost(&self.instance_type))
    }

    fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        self.launch_time
    }

    fn supported_actions(&self) -> Vec<Action> {
        match self.state() {
            ResourceState::Running => vec![
                Action::Stop,
                Action::Restart,
                Action::Terminate,
                Action::ViewDetails,
            ],
            ResourceState::Stopped => vec![
                Action::Start,
                Action::Terminate,
                Action::ViewDetails,
            ],
            ResourceState::Pending | ResourceState::Stopping => vec![Action::ViewDetails],
            _ => vec![Action::ViewDetails],
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn estimate_ec2_cost(instance_type: &str) -> f64 {
    match instance_type {
        t if t.starts_with("t2.micro") => 8.47,
        t if t.starts_with("t2.small") => 16.79,
        t if t.starts_with("t2.medium") => 33.58,
        t if t.starts_with("t3.micro") => 7.59,
        t if t.starts_with("t3.small") => 15.18,
        t if t.starts_with("t3.medium") => 30.37,
        t if t.starts_with("m5.large") => 69.35,
        t if t.starts_with("m5.xlarge") => 138.70,
        t if t.starts_with("c5.large") => 61.06,
        t if t.starts_with("c5.xlarge") => 122.11,
        _ => 50.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_ec2_cost() {
        assert_eq!(estimate_ec2_cost("t2.micro"), 8.47);
        assert_eq!(estimate_ec2_cost("t3.medium"), 30.37);
        assert_eq!(estimate_ec2_cost("unknown"), 50.0);
    }

    #[test]
    fn test_ec2_instance_state_mapping() {
        let instance = EC2Instance {
            instance_id: "i-123".to_string(),
            name: "test".to_string(),
            instance_type: "t2.micro".to_string(),
            state: "running".to_string(),
            region: "us-east-1".to_string(),
            tags: HashMap::new(),
            launch_time: None,
            public_ip: None,
            private_ip: None,
        };

        assert_eq!(instance.state(), ResourceState::Running);
        assert_eq!(instance.resource_type(), ResourceType::Compute);
        assert_eq!(instance.provider(), Provider::AWS);
    }

    #[test]
    fn test_ec2_instance_supported_actions() {
        let mut instance = EC2Instance {
            instance_id: "i-123".to_string(),
            name: "test".to_string(),
            instance_type: "t2.micro".to_string(),
            state: "running".to_string(),
            region: "us-east-1".to_string(),
            tags: HashMap::new(),
            launch_time: None,
            public_ip: None,
            private_ip: None,
        };

        let actions = instance.supported_actions();
        assert!(actions.contains(&Action::Stop));
        assert!(actions.contains(&Action::Terminate));
        assert!(!actions.contains(&Action::Start));

        instance.state = "stopped".to_string();
        let actions = instance.supported_actions();
        assert!(actions.contains(&Action::Start));
        assert!(!actions.contains(&Action::Stop));
    }
}