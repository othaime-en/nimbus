use crate::core::{Action, CloudResource, Provider, ResourceState, ResourceType};
use aws_sdk_route53::types::HostedZone;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct Route53Zone {
    zone_id: String,
    name: String,
    is_private: bool,
    resource_record_set_count: Option<i64>,
    region: String,
    tags: HashMap<String, String>,
}

impl Route53Zone {
    pub fn from_aws_zone(zone: &HostedZone, region: &str) -> Self {
        let zone_id = zone.id().to_string();
        let name = zone.name().to_string();
        let is_private = zone.config()
            .map(|c| c.private_zone())
            .unwrap_or(false);
        let resource_record_set_count = zone.resource_record_set_count();

        Self {
            zone_id,
            name,
            is_private,
            resource_record_set_count,
            region: region.to_string(),
            tags: HashMap::new(),
        }
    }

    pub fn with_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn is_private(&self) -> bool {
        self.is_private
    }

    pub fn record_count(&self) -> Option<i64> {
        self.resource_record_set_count
    }
}

impl CloudResource for Route53Zone {
    fn id(&self) -> &str {
        &self.zone_id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn resource_type(&self) -> ResourceType {
        ResourceType::DNS
    }

    fn provider(&self) -> Provider {
        Provider::AWS
    }

    fn region(&self) -> &str {
        &self.region
    }

    fn state(&self) -> ResourceState {
        ResourceState::Running
    }

    fn cost_per_month(&self) -> Option<f64> {
        Some(estimate_route53_cost(self.resource_record_set_count))
    }

    fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        None
    }

    fn supported_actions(&self) -> Vec<Action> {
        vec![Action::ViewDetails, Action::Terminate]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn estimate_route53_cost(record_count: Option<i64>) -> f64 {
    let base_cost = 0.50;
    
    let record_cost = record_count
        .map(|count| {
            let billable_records = (count - 25).max(0) as f64;
            billable_records * 0.40
        })
        .unwrap_or(0.0);

    base_cost + record_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_route53_cost() {
        assert_eq!(estimate_route53_cost(None), 0.50);
        assert_eq!(estimate_route53_cost(Some(10)), 0.50);
        assert_eq!(estimate_route53_cost(Some(25)), 0.50);
        assert_eq!(estimate_route53_cost(Some(30)), 0.50 + 5.0 * 0.40);
    }

    #[test]
    fn test_route53_zone_basic() {
        let zone = Route53Zone {
            zone_id: "Z1234567890ABC".to_string(),
            name: "example.com.".to_string(),
            is_private: false,
            resource_record_set_count: Some(10),
            region: "global".to_string(),
            tags: HashMap::new(),
        };

        assert_eq!(zone.id(), "Z1234567890ABC");
        assert_eq!(zone.name(), "example.com.");
        assert_eq!(zone.resource_type(), ResourceType::DNS);
        assert_eq!(zone.state(), ResourceState::Running);
        assert!(!zone.is_private());
        assert_eq!(zone.record_count(), Some(10));
    }

    #[test]
    fn test_route53_private_zone() {
        let zone = Route53Zone {
            zone_id: "Z1234567890ABC".to_string(),
            name: "internal.example.com.".to_string(),
            is_private: true,
            resource_record_set_count: Some(5),
            region: "us-east-1".to_string(),
            tags: HashMap::new(),
        };

        assert!(zone.is_private());
    }

    #[test]
    fn test_route53_supported_actions() {
        let zone = Route53Zone {
            zone_id: "Z1234567890ABC".to_string(),
            name: "example.com.".to_string(),
            is_private: false,
            resource_record_set_count: Some(10),
            region: "global".to_string(),
            tags: HashMap::new(),
        };

        let actions = zone.supported_actions();
        assert!(actions.contains(&Action::ViewDetails));
        assert!(actions.contains(&Action::Terminate));
        assert!(!actions.contains(&Action::Start));
    }
}