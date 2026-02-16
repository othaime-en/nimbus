use crate::core::{Action, CloudResource, Provider, ResourceState, ResourceType};
use aws_sdk_elasticloadbalancingv2::types::LoadBalancer;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct ELBLoadBalancer {
    arn: String,
    name: String,
    lb_type: String,
    scheme: String,
    state: String,
    region: String,
    tags: HashMap<String, String>,
    created_at: Option<DateTime<Utc>>,
    dns_name: Option<String>,
    availability_zones: Vec<String>,
}

impl ELBLoadBalancer {
    pub fn from_aws_lb(lb: &LoadBalancer, region: &str) -> Self {
        let arn = lb.load_balancer_arn().unwrap_or("").to_string();
        let name = lb.load_balancer_name().unwrap_or("Unknown").to_string();
        let lb_type = lb.r#type().map(|t| t.as_str().to_string()).unwrap_or_else(|| "application".to_string());
        let scheme = lb.scheme().map(|s| s.as_str().to_string()).unwrap_or_else(|| "internet-facing".to_string());
        let state = lb.state()
            .and_then(|s| s.code())
            .map(|c| c.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let created_at = lb.created_time().and_then(|dt| {
            DateTime::parse_from_rfc3339(&dt.to_string())
                .ok()
                .map(|parsed| parsed.with_timezone(&Utc))
        });

        let dns_name = lb.dns_name().map(|d| d.to_string());

        let availability_zones: Vec<String> = lb
            .availability_zones()
            .iter()
            .filter_map(|az| az.zone_name().map(|z| z.to_string()))
            .collect();

        Self {
            arn,
            name,
            lb_type,
            scheme,
            state,
            region: region.to_string(),
            tags: HashMap::new(),
            created_at,
            dns_name,
            availability_zones,
        }
    }

    pub fn with_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn load_balancer_type(&self) -> &str {
        &self.lb_type
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn dns_name(&self) -> Option<&str> {
        self.dns_name.as_deref()
    }

    pub fn availability_zones(&self) -> &[String] {
        &self.availability_zones
    }
}

impl CloudResource for ELBLoadBalancer {
    fn id(&self) -> &str {
        &self.arn
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn resource_type(&self) -> ResourceType {
        ResourceType::LoadBalancer
    }

    fn provider(&self) -> Provider {
        Provider::AWS
    }

    fn region(&self) -> &str {
        &self.region
    }

    fn state(&self) -> ResourceState {
        match self.state.as_str() {
            "active" => ResourceState::Running,
            "provisioning" => ResourceState::Pending,
            "failed" => ResourceState::Error,
            _ => ResourceState::Unknown,
        }
    }

    fn cost_per_month(&self) -> Option<f64> {
        Some(estimate_elb_cost(&self.lb_type))
    }

    fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    fn supported_actions(&self) -> Vec<Action> {
        vec![Action::ViewDetails, Action::Terminate]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn estimate_elb_cost(lb_type: &str) -> f64 {
    match lb_type {
        "application" => 18.40,
        "network" => 18.40,
        "gateway" => 27.80,
        _ => 18.40,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_elb_cost() {
        assert_eq!(estimate_elb_cost("application"), 18.40);
        assert_eq!(estimate_elb_cost("network"), 18.40);
        assert_eq!(estimate_elb_cost("gateway"), 27.80);
    }

    #[test]
    fn test_elb_basic() {
        let lb = ELBLoadBalancer {
            arn: "arn:aws:elasticloadbalancing:us-east-1:123456789012:loadbalancer/app/my-lb/50dc6c495c0c9188".to_string(),
            name: "my-lb".to_string(),
            lb_type: "application".to_string(),
            scheme: "internet-facing".to_string(),
            state: "active".to_string(),
            region: "us-east-1".to_string(),
            tags: HashMap::new(),
            created_at: None,
            dns_name: Some("my-lb-1234567890.us-east-1.elb.amazonaws.com".to_string()),
            availability_zones: vec!["us-east-1a".to_string(), "us-east-1b".to_string()],
        };

        assert_eq!(lb.name(), "my-lb");
        assert_eq!(lb.resource_type(), ResourceType::LoadBalancer);
        assert_eq!(lb.state(), ResourceState::Running);
        assert_eq!(lb.load_balancer_type(), "application");
    }

    #[test]
    fn test_elb_state_mapping() {
        let mut lb = ELBLoadBalancer {
            arn: "test-arn".to_string(),
            name: "test-lb".to_string(),
            lb_type: "application".to_string(),
            scheme: "internal".to_string(),
            state: "active".to_string(),
            region: "us-east-1".to_string(),
            tags: HashMap::new(),
            created_at: None,
            dns_name: None,
            availability_zones: vec![],
        };

        assert_eq!(lb.state(), ResourceState::Running);

        lb.state = "provisioning".to_string();
        assert_eq!(lb.state(), ResourceState::Pending);

        lb.state = "failed".to_string();
        assert_eq!(lb.state(), ResourceState::Error);
    }

    #[test]
    fn test_elb_supported_actions() {
        let lb = ELBLoadBalancer {
            arn: "test-arn".to_string(),
            name: "test-lb".to_string(),
            lb_type: "application".to_string(),
            scheme: "internal".to_string(),
            state: "active".to_string(),
            region: "us-east-1".to_string(),
            tags: HashMap::new(),
            created_at: None,
            dns_name: None,
            availability_zones: vec![],
        };

        let actions = lb.supported_actions();
        assert!(actions.contains(&Action::ViewDetails));
        assert!(actions.contains(&Action::Terminate));
        assert!(!actions.contains(&Action::Start));
    }
}