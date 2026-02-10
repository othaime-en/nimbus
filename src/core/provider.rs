use async_trait::async_trait;
use crate::core::{
    action::Action,
    cost::{CostBreakdown, CostPeriod},
    resource::{CloudResource, Provider, ResourceType},
};
use crate::error::Result;

#[async_trait]
pub trait CloudProvider: Send + Sync {
    fn name(&self) -> &str;
    fn provider_type(&self) -> Provider;
    
    async fn authenticate(&mut self) -> Result<()>;
    async fn test_connection(&self) -> Result<bool>;
    
    async fn list_all_resources(&self) -> Result<Vec<Box<dyn CloudResource>>>;
    async fn list_resources_by_type(&self, resource_type: ResourceType) -> Result<Vec<Box<dyn CloudResource>>>;
    
    async fn get_resource(&self, id: &str) -> Result<Box<dyn CloudResource>>;
    async fn execute_action(&self, resource_id: &str, action: Action) -> Result<()>;
    
    async fn get_total_cost(&self, period: CostPeriod) -> Result<f64>;
    async fn get_cost_breakdown(&self) -> Result<CostBreakdown>;
    
    fn regions(&self) -> Vec<String>;
    fn current_region(&self) -> &str;
    async fn set_region(&mut self, region: &str) -> Result<()>;
}