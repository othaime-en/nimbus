use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize}; // CHANGES: Added serde imports for serialization support

/// Core trait representing any cloud resource across providers.
/// 
/// This trait provides a unified interface for working with resources from different
/// cloud providers (AWS, GCP, Azure). Implementations should provide resource-specific
/// details while maintaining this common interface.
/// 
/// # Examples
/// 
/// ```no_run
/// use nimbus::core::{CloudResource, ResourceType, Provider, ResourceState};
/// 
/// fn print_resource_info(resource: &dyn CloudResource) {
///     println!("{} - {} ({})", 
///         resource.name(), 
///         resource.resource_type().as_str(),
///         resource.state().as_str()
///     );
/// }
/// ```
pub trait CloudResource: Send + Sync {
    /// Returns the unique identifier for this resource.
    /// Format is provider-specific (e.g., AWS instance ID, GCP resource name).
    fn id(&self) -> &str;
    
    /// Returns the human-readable name of the resource.
    /// May be derived from tags or resource properties.
    fn name(&self) -> &str;
    
    /// Returns the category/type of this resource.
    fn resource_type(&self) -> ResourceType;
    
    /// Returns which cloud provider this resource belongs to.
    fn provider(&self) -> Provider;
    
    /// Returns the region/zone where this resource is deployed.
    fn region(&self) -> &str;
    
    /// Returns the current operational state of the resource.
    fn state(&self) -> ResourceState;
    
    /// Returns estimated monthly cost in USD, if available.
    /// Returns None if cost data is unavailable or not applicable.
    fn cost_per_month(&self) -> Option<f64>;
    
    /// Returns all tags/labels associated with this resource.
    fn tags(&self) -> &HashMap<String, String>;
    
    /// Returns when this resource was created, if known.
    fn created_at(&self) -> Option<DateTime<Utc>>;
    
    /// Returns the list of actions that can be performed on this resource.
    /// Actions depend on resource type and current state.
    fn supported_actions(&self) -> Vec<crate::core::action::Action>;
    
    /// Returns a reference to the concrete type for downcasting.
    /// Used when resource-specific fields need to be accessed.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Categories of cloud resources supported by Nimbus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] // CHANGES: Added Serialize, Deserialize
pub enum ResourceType {
    /// Virtual machines, instances, compute engines
    Compute,
    /// Relational and NoSQL databases
    Database,
    /// Object storage, block storage, file storage
    Storage,
    /// Application and network load balancers
    LoadBalancer,
    /// DNS zones and records
    DNS,
    /// Container orchestration (ECS, GKE, AKS)
    Container,
    /// Functions as a Service (Lambda, Cloud Functions)
    Serverless,
    /// VPCs, subnets, security groups
    Network,
    /// In-memory caches (Redis, Memcached)
    Cache,
    /// Message queues and pub/sub
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

    /// Returns all available resource types.
    pub fn all() -> Vec<ResourceType> {
        vec![
            ResourceType::Compute,
            ResourceType::Database,
            ResourceType::Storage,
            ResourceType::LoadBalancer,
            ResourceType::DNS,
            ResourceType::Container,
            ResourceType::Serverless,
            ResourceType::Network,
            ResourceType::Cache,
            ResourceType::Queue,
        ]
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Supported cloud providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] // CHANGES: Added Serialize, Deserialize
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

    /// Returns all available providers.
    pub fn all() -> Vec<Provider> {
        vec![Provider::AWS, Provider::GCP, Provider::Azure]
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Operational state of a cloud resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    /// Resource is active and operational
    Running,
    /// Resource is stopped but can be started
    Stopped,
    /// Resource has been terminated/deleted
    Terminated,
    /// Resource is being created or initialized
    Pending,
    /// Resource is in the process of stopping
    Stopping,
    /// Resource is in the process of starting
    Starting,
    /// Resource is in an error state
    Error,
    /// State could not be determined
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

    /// Returns true if the resource is in an active/running state.
    pub fn is_active(&self) -> bool {
        matches!(self, ResourceState::Running | ResourceState::Pending | ResourceState::Starting)
    }

    /// Returns true if the resource is in a transitional state.
    pub fn is_transitioning(&self) -> bool {
        matches!(self, ResourceState::Pending | ResourceState::Starting | ResourceState::Stopping)
    }

    /// Returns true if the resource can be started.
    pub fn can_start(&self) -> bool {
        matches!(self, ResourceState::Stopped)
    }

    /// Returns true if the resource can be stopped.
    pub fn can_stop(&self) -> bool {
        matches!(self, ResourceState::Running)
    }
}

impl fmt::Display for ResourceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_as_str() {
        assert_eq!(ResourceType::Compute.as_str(), "Compute");
        assert_eq!(ResourceType::Database.as_str(), "Database");
        assert_eq!(ResourceType::Storage.as_str(), "Storage");
    }

    #[test]
    fn test_resource_type_display() {
        assert_eq!(format!("{}", ResourceType::Compute), "Compute");
        assert_eq!(format!("{}", ResourceType::LoadBalancer), "Load Balancer");
    }

    #[test]
    fn test_resource_type_all() {
        let all = ResourceType::all();
        assert_eq!(all.len(), 10);
        assert!(all.contains(&ResourceType::Compute));
        assert!(all.contains(&ResourceType::Queue));
    }

    #[test]
    fn test_provider_as_str() {
        assert_eq!(Provider::AWS.as_str(), "AWS");
        assert_eq!(Provider::GCP.as_str(), "GCP");
        assert_eq!(Provider::Azure.as_str(), "Azure");
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(format!("{}", Provider::AWS), "AWS");
        assert_eq!(format!("{}", Provider::GCP), "GCP");
    }

    #[test]
    fn test_provider_all() {
        let all = Provider::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&Provider::AWS));
        assert!(all.contains(&Provider::Azure));
    }

    #[test]
    fn test_resource_state_as_str() {
        assert_eq!(ResourceState::Running.as_str(), "Running");
        assert_eq!(ResourceState::Stopped.as_str(), "Stopped");
        assert_eq!(ResourceState::Error.as_str(), "Error");
    }

    #[test]
    fn test_resource_state_display() {
        assert_eq!(format!("{}", ResourceState::Running), "Running");
        assert_eq!(format!("{}", ResourceState::Stopping), "Stopping");
    }

    #[test]
    fn test_resource_state_is_active() {
        assert!(ResourceState::Running.is_active());
        assert!(ResourceState::Pending.is_active());
        assert!(ResourceState::Starting.is_active());
        assert!(!ResourceState::Stopped.is_active());
        assert!(!ResourceState::Terminated.is_active());
        assert!(!ResourceState::Error.is_active());
    }

    #[test]
    fn test_resource_state_is_transitioning() {
        assert!(ResourceState::Pending.is_transitioning());
        assert!(ResourceState::Starting.is_transitioning());
        assert!(ResourceState::Stopping.is_transitioning());
        assert!(!ResourceState::Running.is_transitioning());
        assert!(!ResourceState::Stopped.is_transitioning());
    }

    #[test]
    fn test_resource_state_can_start() {
        assert!(ResourceState::Stopped.can_start());
        assert!(!ResourceState::Running.can_start());
        assert!(!ResourceState::Terminated.can_start());
        assert!(!ResourceState::Starting.can_start());
    }

    #[test]
    fn test_resource_state_can_stop() {
        assert!(ResourceState::Running.can_stop());
        assert!(!ResourceState::Stopped.can_stop());
        assert!(!ResourceState::Stopping.can_stop());
        assert!(!ResourceState::Terminated.can_stop());
    }

    #[test]
    fn test_provider_equality() {
        assert_eq!(Provider::AWS, Provider::AWS);
        assert_ne!(Provider::AWS, Provider::GCP);
    }

    #[test]
    fn test_resource_type_equality() {
        assert_eq!(ResourceType::Compute, ResourceType::Compute);
        assert_ne!(ResourceType::Compute, ResourceType::Database);
    }

    #[test]
    fn test_resource_state_equality() {
        assert_eq!(ResourceState::Running, ResourceState::Running);
        assert_ne!(ResourceState::Running, ResourceState::Stopped);
    }
}