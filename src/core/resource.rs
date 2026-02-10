use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub trait CloudResource: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn resource_type(&self) -> ResourceType;
    fn provider(&self) -> Provider;
    fn region(&self) -> &str;
    fn state(&self) -> ResourceState;
    fn cost_per_month(&self) -> Option<f64>;
    fn tags(&self) -> &HashMap<String, String>;
    fn created_at(&self) -> Option<DateTime<Utc>>;
    fn supported_actions(&self) -> Vec<crate::core::action::Action>;
    
    fn as_any(&self) -> &dyn std::any::Any;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Compute,
    Database,
    Storage,
    LoadBalancer,
    DNS,
    Container,
    Serverless,
    Network,
    Cache,
    Queue,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Compute => "Compute",
            ResourceType::Database => "Database",
            ResourceType::Storage => "Storage",
            ResourceType::LoadBalancer => "Load Balancer",
            ResourceType::DNS => "DNS",
            ResourceType::Container => "Container",
            ResourceType::Serverless => "Serverless",
            ResourceType::Network => "Network",
            ResourceType::Cache => "Cache",
            ResourceType::Queue => "Queue",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    AWS,
    GCP,
    Azure,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::AWS => "AWS",
            Provider::GCP => "GCP",
            Provider::Azure => "Azure",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    Running,
    Stopped,
    Terminated,
    Pending,
    Stopping,
    Starting,
    Error,
    Unknown,
}

impl ResourceState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceState::Running => "Running",
            ResourceState::Stopped => "Stopped",
            ResourceState::Terminated => "Terminated",
            ResourceState::Pending => "Pending",
            ResourceState::Stopping => "Stopping",
            ResourceState::Starting => "Starting",
            ResourceState::Error => "Error",
            ResourceState::Unknown => "Unknown",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, ResourceState::Running | ResourceState::Pending | ResourceState::Starting)
    }
}