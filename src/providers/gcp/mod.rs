use async_trait::async_trait;

use crate::config::GcpConfig;
use crate::core::{
    Action, CloudProvider, CloudResource, CostBreakdown, CostPeriod, Provider, ResourceType,
};
use crate::error::{NimbusError, Result};

mod auth;
mod client;
mod cost;
pub mod resources;

use auth::GcpAuth;
use client::GcpClient;
use cost::GcpCostEstimator;
use resources::{CloudSqlInstance, GceInstance, GcsBucket};

pub struct GCPProvider {
    name: String,
    config: GcpConfig,
    client: Option<GcpClient>,
}

impl GCPProvider {
    pub fn new(config: GcpConfig) -> Self {
        Self {
            name: "GCP".to_string(),
            config,
            client: None,
        }
    }

    async fn ensure_authenticated(&self) -> Result<()> {
        if self.client.is_none() {
            return Err(NimbusError::auth(
                "GCP",
                "Provider not authenticated. Call authenticate() first.",
            ));
        }
        Ok(())
    }

    fn get_client(&self) -> Result<&GcpClient> {
        self.client.as_ref().ok_or_else(|| {
            NimbusError::auth("GCP", "Client not initialized. Call authenticate() first.")
        })
    }

    /// See the doc comment on `resources::gce::default_zone` -- Compute
    /// Engine's APIs are zone-scoped but `GcpConfig` only carries a region.
    fn zone(&self) -> String {
        resources::gce::default_zone(&self.config.region)
    }

    async fn list_gce_instances(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        use google_cloud_gax::paginator::ItemPaginator;

        let client = self.get_client()?;
        let zone = self.zone();

        let mut page = client
            .instances
            .list()
            .set_project(&self.config.project_id)
            .set_zone(&zone)
            .by_item();

        let mut instances: Vec<Box<dyn CloudResource>> = Vec::new();
        while let Some(item) = page.next().await.transpose().map_err(|e| {
            NimbusError::provider("GCP", format!("Failed to list Compute Engine instances: {}", e))
        })? {
            instances.push(Box::new(GceInstance::from_gce_instance(&item, &zone)));
        }

        Ok(instances)
    }

    async fn list_cloudsql_instances(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        use google_cloud_gax::paginator::ItemPaginator;

        let client = self.get_client()?;

        let mut page = client
            .sql_instances
            .list()
            .set_project(&self.config.project_id)
            .by_item();

        let mut instances: Vec<Box<dyn CloudResource>> = Vec::new();
        while let Some(item) = page.next().await.transpose().map_err(|e| {
            NimbusError::provider("GCP", format!("Failed to list Cloud SQL instances: {}", e))
        })? {
            instances.push(Box::new(CloudSqlInstance::from_sql_instance(&item)));
        }

        Ok(instances)
    }

    async fn list_gcs_buckets(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        use google_cloud_gax::paginator::ItemPaginator;

        let client = self.get_client()?;
        let parent = format!("projects/{}", self.config.project_id);

        let mut page = client
            .storage_control
            .list_buckets()
            .set_parent(&parent)
            .by_item();

        let mut buckets: Vec<Box<dyn CloudResource>> = Vec::new();
        while let Some(item) = page.next().await.transpose().map_err(|e| {
            NimbusError::provider("GCP", format!("Failed to list Cloud Storage buckets: {}", e))
        })? {
            buckets.push(Box::new(GcsBucket::from_bucket(&item)));
        }

        Ok(buckets)
    }
}

#[async_trait]
impl CloudProvider for GCPProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn provider_type(&self) -> Provider {
        Provider::GCP
    }

    async fn authenticate(&mut self) -> Result<()> {
        GcpAuth::require_project_id(&self.config)?;

        let credentials = GcpAuth::build_credentials(&self.config).await?;
        let client = GcpClient::new(credentials).await?;

        self.client = Some(client);
        Ok(())
    }

    async fn test_connection(&self) -> Result<bool> {
        self.ensure_authenticated().await?;

        // A cheap, harmless read: listing instances in the configured zone
        // doubles as an auth/connectivity check, same idea as AWS's
        // describe_regions() call.
        match self.list_gce_instances().await {
            Ok(_) => Ok(true),
            Err(e) => Err(NimbusError::provider(
                "GCP",
                format!("Connection test failed: {}", e),
            )),
        }
    }

    async fn list_all_resources(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        self.ensure_authenticated().await?;

        let mut all_resources: Vec<Box<dyn CloudResource>> = Vec::new();

        if let Ok(gce) = self.list_gce_instances().await {
            all_resources.extend(gce);
        }

        if let Ok(sql) = self.list_cloudsql_instances().await {
            all_resources.extend(sql);
        }

        if let Ok(gcs) = self.list_gcs_buckets().await {
            all_resources.extend(gcs);
        }

        Ok(all_resources)
    }

    async fn list_resources_by_type(
        &self,
        resource_type: ResourceType,
    ) -> Result<Vec<Box<dyn CloudResource>>> {
        self.ensure_authenticated().await?;

        match resource_type {
            ResourceType::Compute => self.list_gce_instances().await,
            ResourceType::Database => self.list_cloudsql_instances().await,
            ResourceType::Storage => self.list_gcs_buckets().await,
            _ => Ok(Vec::new()),
        }
    }

    async fn get_resource(
        &self,
        id: &str,
        resource_type: ResourceType,
    ) -> Result<Box<dyn CloudResource>> {
        self.ensure_authenticated().await?;

        if resource_type == ResourceType::Compute {
            let client = self.get_client()?;
            let zone = self.zone();

            let instance = client
                .instances
                .get()
                .set_project(&self.config.project_id)
                .set_zone(&zone)
                .set_instance(id)
                .send()
                .await
                .map_err(|e| {
                    NimbusError::provider("GCP", format!("Failed to get instance {}: {}", id, e))
                })?;

            return Ok(Box::new(GceInstance::from_gce_instance(&instance, &zone)));
        }

        // Cloud SQL / GCS lookup-by-ID isn't implemented yet -- nothing in
        // the app currently calls get_resource for those types, mirroring
        // the same gap (and same reasoning) on the AWS side.
        Err(NimbusError::ResourceNotFound(id.to_string()))
    }

    /// Dispatches a lifecycle action to the resource module that owns it.
    /// Only Compute (GCE) is wired up so far -- see the Phase 3.1 handoff
    /// notes on why Cloud SQL actions are deferred, and `GcsBucket`'s
    /// `supported_actions()` for why storage never offers them.
    async fn execute_action(
        &self,
        resource_id: &str,
        resource_type: ResourceType,
        action: Action,
    ) -> Result<()> {
        self.ensure_authenticated().await?;
        let client = self.get_client()?;

        match resource_type {
            ResourceType::Compute => {
                resources::gce::execute_action(
                    client,
                    &self.config.project_id,
                    &self.zone(),
                    resource_id,
                    action,
                )
                .await
            }
            other => Err(NimbusError::UnsupportedAction(action, other)),
        }
    }

    async fn get_total_cost(&self, period: CostPeriod) -> Result<f64> {
        self.ensure_authenticated().await?;
        let resources = self.list_all_resources().await?;
        Ok(GcpCostEstimator::total_cost(&resources, period))
    }

    async fn get_cost_breakdown(&self) -> Result<CostBreakdown> {
        self.ensure_authenticated().await?;
        let resources = self.list_all_resources().await?;
        Ok(GcpCostEstimator::cost_breakdown(&resources))
    }

    fn regions(&self) -> Vec<String> {
        vec![
            "us-central1".to_string(),
            "us-east1".to_string(),
            "us-east4".to_string(),
            "us-west1".to_string(),
            "us-west4".to_string(),
            "europe-west1".to_string(),
            "europe-west4".to_string(),
            "asia-east1".to_string(),
            "asia-southeast1".to_string(),
            "asia-northeast1".to_string(),
        ]
    }

    fn current_region(&self) -> &str {
        &self.config.region
    }

    async fn set_region(&mut self, region: &str) -> Result<()> {
        if !self.regions().contains(&region.to_string()) {
            return Err(NimbusError::InvalidRegion(region.to_string()));
        }

        self.config.region = region.to_string();
        // Unlike AWS, GCP's clients aren't constructed with a region baked
        // in -- the zone/project is passed per-request -- so no re-auth is
        // needed here.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let config = GcpConfig {
            project_id: "my-project".to_string(),
            credentials_file: None,
            region: "us-central1".to_string(),
        };

        let provider = GCPProvider::new(config);
        assert_eq!(provider.name(), "GCP");
        assert_eq!(provider.provider_type(), Provider::GCP);
    }

    #[test]
    fn test_zone_derivation() {
        let provider = GCPProvider::new(GcpConfig {
            project_id: "p".to_string(),
            credentials_file: None,
            region: "us-central1".to_string(),
        });
        assert_eq!(provider.zone(), "us-central1-a");

        let provider = GCPProvider::new(GcpConfig {
            project_id: "p".to_string(),
            credentials_file: None,
            region: "europe-west4-b".to_string(),
        });
        assert_eq!(provider.zone(), "europe-west4-b");
    }

    #[tokio::test]
    async fn test_unauthenticated_operations_fail() {
        let provider = GCPProvider::new(GcpConfig::default());

        assert!(provider.list_all_resources().await.is_err());
        assert!(provider.get_total_cost(CostPeriod::ThisMonth).await.is_err());
        assert!(matches!(
            provider.list_all_resources().await,
            Err(NimbusError::AuthenticationFailed(_, _))
        ));
    }

    #[test]
    fn test_set_region_rejects_unknown_region() {
        let mut provider = GCPProvider::new(GcpConfig {
            project_id: "p".to_string(),
            credentials_file: None,
            region: "us-central1".to_string(),
        });

        let result = tokio_test_block_on(provider.set_region("mars-north-1"));
        assert!(matches!(result, Err(NimbusError::InvalidRegion(_))));
    }

    // Small helper so the sync test above doesn't need #[tokio::test]'s
    // full runtime just to await one call.
    fn tokio_test_block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(f)
    }
}
