use crate::core::{Action, CloudResource, Provider, ResourceState, ResourceType};
use chrono::{DateTime, TimeZone, Utc};
use google_cloud_storage::model::Bucket;
use std::collections::HashMap;

pub struct GcsBucket {
    name: String,
    location: String,
    storage_class: String,
    labels: HashMap<String, String>,
    created_at: Option<DateTime<Utc>>,
}

impl GcsBucket {
    /// VERIFY: field names here (`location`, `storage_class`, `labels`,
    /// `create_time`) are inferred from the Cloud Storage JSON API's
    /// resource shape -- check against `cargo doc -p google-cloud-storage`
    /// once this builds. `create_time` is expected to be a
    /// `google_cloud_wkt::Timestamp` (protobuf `seconds`/`nanos` fields,
    /// same shape as `prost_types::Timestamp`), converted to `chrono` below.
    pub fn from_bucket(bucket: &Bucket) -> Self {
        let name = bucket.name.clone().unwrap_or_default();
        let location = bucket.location.clone().unwrap_or_default();
        let storage_class = bucket
            .storage_class
            .clone()
            .unwrap_or_else(|| "STANDARD".to_string());

        let created_at = bucket
            .create_time
            .as_ref()
            .and_then(|ts| Utc.timestamp_opt(ts.seconds, ts.nanos as u32).single());

        Self {
            name,
            location,
            storage_class,
            labels: bucket.labels.clone(),
            created_at,
        }
    }

    pub fn storage_class(&self) -> &str {
        &self.storage_class
    }
}

impl CloudResource for GcsBucket {
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
        Provider::GCP
    }

    fn region(&self) -> &str {
        &self.location
    }

    fn state(&self) -> ResourceState {
        // Buckets don't have a running/stopped lifecycle -- if it's listed,
        // it exists and is usable. Same convention as S3Bucket on the AWS
        // side.
        ResourceState::Running
    }

    fn cost_per_month(&self) -> Option<f64> {
        // No size data from a plain bucket listing -- a real number needs a
        // separate per-bucket storage-usage query. Flat per-class estimate
        // for now, same placeholder-until-wired-up spirit as S3Bucket.
        Some(estimate_gcs_cost(&self.storage_class))
    }

    fn tags(&self) -> &HashMap<String, String> {
        &self.labels
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    fn supported_actions(&self) -> Vec<Action> {
        vec![Action::ViewDetails]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn estimate_gcs_cost(storage_class: &str) -> f64 {
    match storage_class {
        "STANDARD" => 5.0,
        "NEARLINE" => 2.0,
        "COLDLINE" => 1.0,
        "ARCHIVE" => 0.5,
        _ => 5.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_gcs_cost() {
        assert_eq!(estimate_gcs_cost("STANDARD"), 5.0);
        assert_eq!(estimate_gcs_cost("ARCHIVE"), 0.5);
        assert_eq!(estimate_gcs_cost("unknown"), 5.0);
    }

    #[test]
    fn test_gcs_bucket_is_always_running() {
        let bucket = GcsBucket {
            name: "my-bucket".to_string(),
            location: "US".to_string(),
            storage_class: "STANDARD".to_string(),
            labels: HashMap::new(),
            created_at: None,
        };

        assert_eq!(bucket.state(), ResourceState::Running);
        assert_eq!(bucket.resource_type(), ResourceType::Storage);
        assert_eq!(bucket.supported_actions(), vec![Action::ViewDetails]);
    }
}
