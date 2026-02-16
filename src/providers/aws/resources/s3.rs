use crate::core::{Action, CloudResource, Provider, ResourceState, ResourceType};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct S3Bucket {
    name: String,
    region: String,
    created_at: Option<DateTime<Utc>>,
    tags: HashMap<String, String>,
    size_bytes: Option<u64>,
    object_count: Option<u64>,
}

impl S3Bucket {
    pub fn new(
        name: String,
        region: String,
        created_at: Option<DateTime<Utc>>,
        tags: HashMap<String, String>,
    ) -> Self {
        Self {
            name,
            region,
            created_at,
            tags,
            size_bytes: None,
            object_count: None,
        }
    }

    pub fn with_size_info(mut self, size_bytes: u64, object_count: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self.object_count = Some(object_count);
        self
    }

    pub fn size_bytes(&self) -> Option<u64> {
        self.size_bytes
    }

    pub fn object_count(&self) -> Option<u64> {
        self.object_count
    }

    pub fn size_gb(&self) -> Option<f64> {
        self.size_bytes.map(|bytes| bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

impl CloudResource for S3Bucket {
    fn id(&self) -> &str {
        &self.name
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn resource_type(&self) -> ResourceType {
        ResourceType::Storage
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
        self.size_gb().map(|gb| estimate_s3_cost(gb))
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

fn estimate_s3_cost(size_gb: f64) -> f64 {
    if size_gb <= 50.0 * 1024.0 {
        size_gb * 0.023
    } else if size_gb <= 450.0 * 1024.0 {
        50.0 * 1024.0 * 0.023 + (size_gb - 50.0 * 1024.0) * 0.022
    } else {
        50.0 * 1024.0 * 0.023 + 400.0 * 1024.0 * 0.022 + (size_gb - 450.0 * 1024.0) * 0.021
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_s3_cost() {
        assert_eq!(estimate_s3_cost(100.0), 100.0 * 0.023);
        
        let large_size = 60.0 * 1024.0;
        let expected = 50.0 * 1024.0 * 0.023 + (large_size - 50.0 * 1024.0) * 0.022;
        assert_eq!(estimate_s3_cost(large_size), expected);
    }

    #[test]
    fn test_s3_bucket_basic() {
        let bucket = S3Bucket::new(
            "my-bucket".to_string(),
            "us-east-1".to_string(),
            None,
            HashMap::new(),
        );

        assert_eq!(bucket.name(), "my-bucket");
        assert_eq!(bucket.id(), "my-bucket");
        assert_eq!(bucket.resource_type(), ResourceType::Storage);
        assert_eq!(bucket.state(), ResourceState::Running);
    }

    #[test]
    fn test_s3_bucket_with_size() {
        let bucket = S3Bucket::new(
            "my-bucket".to_string(),
            "us-east-1".to_string(),
            None,
            HashMap::new(),
        )
        .with_size_info(5 * 1024 * 1024 * 1024, 1000);

        assert_eq!(bucket.size_bytes(), Some(5 * 1024 * 1024 * 1024));
        assert_eq!(bucket.object_count(), Some(1000));
        assert!(bucket.size_gb().unwrap() > 4.9);
        assert!(bucket.cost_per_month().is_some());
    }

    #[test]
    fn test_s3_supported_actions() {
        let bucket = S3Bucket::new(
            "test".to_string(),
            "us-east-1".to_string(),
            None,
            HashMap::new(),
        );

        let actions = bucket.supported_actions();
        assert!(actions.contains(&Action::ViewDetails));
        assert!(actions.contains(&Action::Terminate));
        assert!(!actions.contains(&Action::Start));
    }
}