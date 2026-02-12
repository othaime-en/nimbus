use async_trait::async_trait;
use crate::core::{
    action::Action,
    cost::{CostBreakdown, CostPeriod},
    resource::{CloudResource, Provider, ResourceType},
};
use crate::error::Result;

/// Core trait for cloud provider implementations.
/// 
/// Each cloud provider (AWS, GCP, Azure) implements this trait to provide
/// unified access to cloud resources. Implementations handle authentication,
/// resource discovery, cost tracking, and lifecycle operations.
/// 
/// # Example Implementation
/// 
/// ```no_run
/// use async_trait::async_trait;
/// use nimbus::core::{CloudProvider, Provider};
/// # use nimbus::error::Result;
/// 
/// struct MyProvider {
///     name: String,
/// }
/// 
/// #[async_trait]
/// impl CloudProvider for MyProvider {
///     fn name(&self) -> &str {
///         &self.name
///     }
///     
///     fn provider_type(&self) -> Provider {
///         Provider::AWS
///     }
///     
///     // ... implement other methods
/// #   async fn authenticate(&mut self) -> Result<()> { Ok(()) }
/// #   async fn test_connection(&self) -> Result<bool> { Ok(true) }
/// #   async fn list_all_resources(&self) -> Result<Vec<Box<dyn nimbus::core::CloudResource>>> { Ok(vec![]) }
/// #   async fn list_resources_by_type(&self, _: nimbus::core::ResourceType) -> Result<Vec<Box<dyn nimbus::core::CloudResource>>> { Ok(vec![]) }
/// #   async fn get_resource(&self, _: &str) -> Result<Box<dyn nimbus::core::CloudResource>> { unimplemented!() }
/// #   async fn execute_action(&self, _: &str, _: nimbus::core::Action) -> Result<()> { Ok(()) }
/// #   async fn get_total_cost(&self, _: nimbus::core::CostPeriod) -> Result<f64> { Ok(0.0) }
/// #   async fn get_cost_breakdown(&self) -> Result<nimbus::core::CostBreakdown> { Ok(nimbus::core::CostBreakdown::new()) }
/// #   fn regions(&self) -> Vec<String> { vec![] }
/// #   fn current_region(&self) -> &str { "us-east-1" }
/// #   async fn set_region(&mut self, _: &str) -> Result<()> { Ok(()) }
/// }
/// ```
#[async_trait]
pub trait CloudProvider: Send + Sync {
    /// Returns the provider's display name.
    fn name(&self) -> &str;
    
    /// Returns which cloud provider this is (AWS, GCP, Azure).
    fn provider_type(&self) -> Provider;
    
    /// Authenticates with the cloud provider using configured credentials.
    /// 
    /// This should validate credentials and establish a session. Called once
    /// during initialization before any other operations.
    async fn authenticate(&mut self) -> Result<()>;
    
    /// Tests the connection to the cloud provider.
    async fn test_connection(&self) -> Result<bool>;
    
    /// Lists all resources across all types in the current region.
    /// 
    /// This may be slow for providers with many resources. Consider using
    /// `list_resources_by_type` to fetch specific types.
    async fn list_all_resources(&self) -> Result<Vec<Box<dyn CloudResource>>>;
    
    /// Lists resources of a specific type in the current region.
    /// 
    /// More efficient than `list_all_resources` when you only need
    /// specific resource types.
    async fn list_resources_by_type(
        &self, 
        resource_type: ResourceType
    ) -> Result<Vec<Box<dyn CloudResource>>>;
    
    /// Retrieves a specific resource by its unique ID.
    async fn get_resource(&self, id: &str) -> Result<Box<dyn CloudResource>>;
    
    /// Executes an action on a resource.
    async fn execute_action(&self, resource_id: &str, action: Action) -> Result<()>;
    
    /// Gets the total cost for a given time period.
    async fn get_total_cost(&self, period: CostPeriod) -> Result<f64>;
    
    /// Gets detailed cost breakdown by service and region.
    /// 
    /// Provides comprehensive cost information including trends and
    /// categorization by service type and geographic region.
    async fn get_cost_breakdown(&self) -> Result<CostBreakdown>;
    
    /// Returns all available regions for this provider.
    fn regions(&self) -> Vec<String>;
    
    /// Returns the currently selected region.
    fn current_region(&self) -> &str;
    
    /// Changes the active region for resource queries.
    async fn set_region(&mut self, region: &str) -> Result<()>;
}