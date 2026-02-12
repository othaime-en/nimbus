use nimbus::core::{
    Action, CloudProvider, CloudResource, CostBreakdown, CostPeriod, Provider, ResourceState,
    ResourceType,
};
use nimbus::error::{NimbusError, Result};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;

struct MockResource {
    id: String,
    name: String,
    resource_type: ResourceType,
    provider: Provider,
    region: String,
    state: ResourceState,
    tags: HashMap<String, String>,
}

impl CloudResource for MockResource {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn resource_type(&self) -> ResourceType {
        self.resource_type
    }

    fn provider(&self) -> Provider {
        self.provider
    }

    fn region(&self) -> &str {
        &self.region
    }

    fn state(&self) -> ResourceState {
        self.state
    }

    fn cost_per_month(&self) -> Option<f64> {
        Some(100.0)
    }

    fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }

    fn created_at(&self) -> Option<chrono::DateTime<Utc>> {
        Some(Utc::now())
    }

    fn supported_actions(&self) -> Vec<Action> {
        vec![Action::Start, Action::Stop, Action::ViewDetails]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct MockProvider {
    name: String,
    provider_type: Provider,
    region: String,
    resources: Vec<MockResource>,
}

#[async_trait]
impl CloudProvider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn provider_type(&self) -> Provider {
        self.provider_type
    }

    async fn authenticate(&mut self) -> Result<()> {
        Ok(())
    }

    async fn test_connection(&self) -> Result<bool> {
        Ok(true)
    }

    async fn list_all_resources(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        let resources: Vec<Box<dyn CloudResource>> = self
            .resources
            .iter()
            .map(|r| {
                Box::new(MockResource {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    resource_type: r.resource_type,
                    provider: r.provider,
                    region: r.region.clone(),
                    state: r.state,
                    tags: r.tags.clone(),
                }) as Box<dyn CloudResource>
            })
            .collect();
        Ok(resources)
    }

    async fn list_resources_by_type(
        &self,
        resource_type: ResourceType,
    ) -> Result<Vec<Box<dyn CloudResource>>> {
        let resources: Vec<Box<dyn CloudResource>> = self
            .resources
            .iter()
            .filter(|r| r.resource_type == resource_type)
            .map(|r| {
                Box::new(MockResource {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    resource_type: r.resource_type,
                    provider: r.provider,
                    region: r.region.clone(),
                    state: r.state,
                    tags: r.tags.clone(),
                }) as Box<dyn CloudResource>
            })
            .collect();
        Ok(resources)
    }

    async fn get_resource(&self, id: &str) -> Result<Box<dyn CloudResource>> {
        self.resources
            .iter()
            .find(|r| r.id == id)
            .map(|r| {
                Box::new(MockResource {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    resource_type: r.resource_type,
                    provider: r.provider,
                    region: r.region.clone(),
                    state: r.state,
                    tags: r.tags.clone(),
                }) as Box<dyn CloudResource>
            })
            .ok_or_else(|| NimbusError::ResourceNotFound(id.to_string()))
    }

    async fn execute_action(&self, _resource_id: &str, _action: Action) -> Result<()> {
        Ok(())
    }

    async fn get_total_cost(&self, _period: CostPeriod) -> Result<f64> {
        Ok(500.0)
    }

    async fn get_cost_breakdown(&self) -> Result<CostBreakdown> {
        let mut breakdown = CostBreakdown::new();
        breakdown.total = 500.0;
        breakdown.add_service_cost("Compute".to_string(), 300.0);
        breakdown.add_service_cost("Storage".to_string(), 200.0);
        breakdown.add_region_cost("us-east-1".to_string(), 500.0);
        Ok(breakdown)
    }

    fn regions(&self) -> Vec<String> {
        vec!["us-east-1".to_string(), "us-west-2".to_string()]
    }

    fn current_region(&self) -> &str {
        &self.region
    }

    async fn set_region(&mut self, region: &str) -> Result<()> {
        self.region = region.to_string();
        Ok(())
    }
}

fn create_mock_provider() -> MockProvider {
    MockProvider {
        name: "Test Provider".to_string(),
        provider_type: Provider::AWS,
        region: "us-east-1".to_string(),
        resources: vec![
            MockResource {
                id: "i-1234".to_string(),
                name: "web-server".to_string(),
                resource_type: ResourceType::Compute,
                provider: Provider::AWS,
                region: "us-east-1".to_string(),
                state: ResourceState::Running,
                tags: HashMap::new(),
            },
            MockResource {
                id: "db-5678".to_string(),
                name: "prod-db".to_string(),
                resource_type: ResourceType::Database,
                provider: Provider::AWS,
                region: "us-east-1".to_string(),
                state: ResourceState::Running,
                tags: HashMap::new(),
            },
        ],
    }
}

#[tokio::test]
async fn test_provider_authentication() {
    let mut provider = create_mock_provider();
    let result = provider.authenticate().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_provider_test_connection() {
    let provider = create_mock_provider();
    let result = provider.test_connection().await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_list_all_resources() {
    let provider = create_mock_provider();
    let resources = provider.list_all_resources().await.unwrap();
    assert_eq!(resources.len(), 2);
}

#[tokio::test]
async fn test_list_resources_by_type() {
    let provider = create_mock_provider();
    let compute = provider
        .list_resources_by_type(ResourceType::Compute)
        .await
        .unwrap();
    assert_eq!(compute.len(), 1);
    assert_eq!(compute[0].id(), "i-1234");

    let database = provider
        .list_resources_by_type(ResourceType::Database)
        .await
        .unwrap();
    assert_eq!(database.len(), 1);
    assert_eq!(database[0].id(), "db-5678");

    let storage = provider
        .list_resources_by_type(ResourceType::Storage)
        .await
        .unwrap();
    assert_eq!(storage.len(), 0);
}

#[tokio::test]
async fn test_get_resource() {
    let provider = create_mock_provider();
    let resource = provider.get_resource("i-1234").await.unwrap();
    assert_eq!(resource.name(), "web-server");
    assert_eq!(resource.resource_type(), ResourceType::Compute);
}

#[tokio::test]
async fn test_get_resource_not_found() {
    let provider = create_mock_provider();
    let result = provider.get_resource("not-exists").await;
    assert!(result.is_err());
    match result {
        Err(NimbusError::ResourceNotFound(id)) => assert_eq!(id, "not-exists"),
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[tokio::test]
async fn test_get_total_cost() {
    let provider = create_mock_provider();
    let cost = provider.get_total_cost(CostPeriod::ThisMonth).await.unwrap();
    assert_eq!(cost, 500.0);
}

#[tokio::test]
async fn test_get_cost_breakdown() {
    let provider = create_mock_provider();
    let breakdown = provider.get_cost_breakdown().await.unwrap();
    assert_eq!(breakdown.total, 500.0);
    assert_eq!(breakdown.by_service.len(), 2);
    assert_eq!(*breakdown.by_service.get("Compute").unwrap(), 300.0);
}

#[tokio::test]
async fn test_provider_regions() {
    let provider = create_mock_provider();
    let regions = provider.regions();
    assert_eq!(regions.len(), 2);
    assert!(regions.contains(&"us-east-1".to_string()));
}

#[tokio::test]
async fn test_set_region() {
    let mut provider = create_mock_provider();
    assert_eq!(provider.current_region(), "us-east-1");

    provider.set_region("us-west-2").await.unwrap();
    assert_eq!(provider.current_region(), "us-west-2");
}

#[test]
fn test_resource_trait_methods() {
    let resource = MockResource {
        id: "test-123".to_string(),
        name: "test-resource".to_string(),
        resource_type: ResourceType::Compute,
        provider: Provider::AWS,
        region: "us-east-1".to_string(),
        state: ResourceState::Running,
        tags: HashMap::new(),
    };

    assert_eq!(resource.id(), "test-123");
    assert_eq!(resource.name(), "test-resource");
    assert_eq!(resource.resource_type(), ResourceType::Compute);
    assert_eq!(resource.provider(), Provider::AWS);
    assert_eq!(resource.region(), "us-east-1");
    assert_eq!(resource.state(), ResourceState::Running);
    assert_eq!(resource.cost_per_month(), Some(100.0));
    assert!(resource.tags().is_empty());
    assert!(resource.created_at().is_some());

    let actions = resource.supported_actions();
    assert_eq!(actions.len(), 3);
    assert!(actions.contains(&Action::Start));
}

#[test]
fn test_trait_object_usage() {
    let resource: Box<dyn CloudResource> = Box::new(MockResource {
        id: "test".to_string(),
        name: "test".to_string(),
        resource_type: ResourceType::Database,
        provider: Provider::GCP,
        region: "us-central1".to_string(),
        state: ResourceState::Stopped,
        tags: HashMap::new(),
    });

    assert_eq!(resource.provider(), Provider::GCP);
    assert_eq!(resource.state(), ResourceState::Stopped);
}