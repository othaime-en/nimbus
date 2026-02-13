use async_trait::async_trait;
use aws_config::SdkConfig;

use crate::config::AwsConfig;
use crate::core::{
    Action, CloudProvider, CloudResource, CostBreakdown, CostPeriod, Provider, ResourceType,
};
use crate::error::{NimbusError, Result};

mod auth;
mod client;
mod cost;
pub mod resources;

use auth::AwsAuth;
use client::AwsClient;
use cost::AwsCostExplorer;
use resources::EC2Instance;

pub struct AWSProvider {
    name: String,
    config: AwsConfig,
    sdk_config: Option<SdkConfig>,
    client: Option<AwsClient>,
    cost_explorer: Option<AwsCostExplorer>,
}

impl AWSProvider {
    pub fn new(config: AwsConfig) -> Self {
        Self {
            name: "AWS".to_string(),
            config,
            sdk_config: None,
            client: None,
            cost_explorer: None,
        }
    }

    async fn ensure_authenticated(&self) -> Result<()> {
        if self.sdk_config.is_none() {
            return Err(NimbusError::auth(
                "AWS",
                "Provider not authenticated. Call authenticate() first.",
            ));
        }
        Ok(())
    }

    fn get_client(&self) -> Result<&AwsClient> {
        self.client.as_ref().ok_or_else(|| {
            NimbusError::auth(
                "AWS",
                "Client not initialized. Call authenticate() first.",
            )
        })
    }

    fn get_cost_explorer(&self) -> Result<&AwsCostExplorer> {
        self.cost_explorer.as_ref().ok_or_else(|| {
            NimbusError::auth(
                "AWS",
                "Cost explorer not initialized. Call authenticate() first.",
            )
        })
    }
}

#[async_trait]
impl CloudProvider for AWSProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn provider_type(&self) -> Provider {
        Provider::AWS
    }

    async fn authenticate(&mut self) -> Result<()> {
        let sdk_config = AwsAuth::create_config(&self.config).await?;

        AwsAuth::test_credentials(&sdk_config).await?;

        let client = AwsClient::new(&sdk_config);
        let cost_explorer = AwsCostExplorer::new(client.cost_explorer.clone());

        self.sdk_config = Some(sdk_config);
        self.client = Some(client);
        self.cost_explorer = Some(cost_explorer);

        Ok(())
    }

    async fn test_connection(&self) -> Result<bool> {
        self.ensure_authenticated().await?;
        let client = self.get_client()?;

        match client.ec2.describe_regions().send().await {
            Ok(_) => Ok(true),
            Err(e) => Err(NimbusError::provider(
                "AWS",
                format!("Connection test failed: {}", e),
            )),
        }
    }

    async fn list_all_resources(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        self.ensure_authenticated().await?;

        let mut all_resources: Vec<Box<dyn CloudResource>> = Vec::new();

        let ec2_instances = self.list_resources_by_type(ResourceType::Compute).await?;
        all_resources.extend(ec2_instances);

        Ok(all_resources)
    }

    async fn list_resources_by_type(
        &self,
        resource_type: ResourceType,
    ) -> Result<Vec<Box<dyn CloudResource>>> {
        self.ensure_authenticated().await?;

        match resource_type {
            ResourceType::Compute => {
                let client = self.get_client()?;
                let response = client
                    .ec2
                    .describe_instances()
                    .send()
                    .await
                    .map_err(|e| {
                        NimbusError::provider("AWS", format!("Failed to list EC2 instances: {}", e))
                    })?;

                let mut instances: Vec<Box<dyn CloudResource>> = Vec::new();

                for reservation in response.reservations() {
                    for instance in reservation.instances() {
                        let ec2_instance = EC2Instance::from_aws_instance(instance, &self.config.region);
                        instances.push(Box::new(ec2_instance));
                    }
                }

                Ok(instances)
            }
            _ => Ok(Vec::new()),
        }
    }

    async fn get_resource(&self, id: &str) -> Result<Box<dyn CloudResource>> {
        self.ensure_authenticated().await?;
        let client = self.get_client()?;

        let response = client
            .ec2
            .describe_instances()
            .instance_ids(id)
            .send()
            .await
            .map_err(|e| {
                NimbusError::provider(
                    "AWS",
                    format!("Failed to get instance {}: {}", id, e),
                )
            })?;

        for reservation in response.reservations() {
            for instance in reservation.instances() {
                if instance.instance_id() == Some(id) {
                    let ec2_instance = EC2Instance::from_aws_instance(instance, &self.config.region);
                    return Ok(Box::new(ec2_instance));
                }
            }
        }

        Err(NimbusError::ResourceNotFound(id.to_string()))
    }

    async fn execute_action(&self, resource_id: &str, action: Action) -> Result<()> {
        self.ensure_authenticated().await?;
        let client = self.get_client()?;

        match action {
            Action::Start => {
                client
                    .ec2
                    .start_instances()
                    .instance_ids(resource_id)
                    .send()
                    .await
                    .map_err(|e| {
                        NimbusError::provider(
                            "AWS",
                            format!("Failed to start instance {}: {}", resource_id, e),
                        )
                    })?;
                Ok(())
            }
            Action::Stop => {
                client
                    .ec2
                    .stop_instances()
                    .instance_ids(resource_id)
                    .send()
                    .await
                    .map_err(|e| {
                        NimbusError::provider(
                            "AWS",
                            format!("Failed to stop instance {}: {}", resource_id, e),
                        )
                    })?;
                Ok(())
            }
            Action::Restart => {
                client
                    .ec2
                    .reboot_instances()
                    .instance_ids(resource_id)
                    .send()
                    .await
                    .map_err(|e| {
                        NimbusError::provider(
                            "AWS",
                            format!("Failed to restart instance {}: {}", resource_id, e),
                        )
                    })?;
                Ok(())
            }
            Action::Terminate => {
                client
                    .ec2
                    .terminate_instances()
                    .instance_ids(resource_id)
                    .send()
                    .await
                    .map_err(|e| {
                        NimbusError::provider(
                            "AWS",
                            format!("Failed to terminate instance {}: {}", resource_id, e),
                        )
                    })?;
                Ok(())
            }
            _ => Err(NimbusError::UnsupportedAction(
                action,
                ResourceType::Compute,
            )),
        }
    }

    async fn get_total_cost(&self, period: CostPeriod) -> Result<f64> {
        self.ensure_authenticated().await?;
        let cost_explorer = self.get_cost_explorer()?;
        cost_explorer.get_total_cost(period).await
    }

    async fn get_cost_breakdown(&self) -> Result<CostBreakdown> {
        self.ensure_authenticated().await?;
        let cost_explorer = self.get_cost_explorer()?;
        cost_explorer.get_cost_breakdown().await
    }

    fn regions(&self) -> Vec<String> {
        vec![
            "us-east-1".to_string(),
            "us-east-2".to_string(),
            "us-west-1".to_string(),
            "us-west-2".to_string(),
            "eu-west-1".to_string(),
            "eu-west-2".to_string(),
            "eu-west-3".to_string(),
            "eu-central-1".to_string(),
            "ap-northeast-1".to_string(),
            "ap-northeast-2".to_string(),
            "ap-southeast-1".to_string(),
            "ap-southeast-2".to_string(),
            "ap-south-1".to_string(),
            "sa-east-1".to_string(),
            "ca-central-1".to_string(),
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
        self.authenticate().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let config = AwsConfig {
            profile: Some("default".to_string()),
            region: "us-east-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
        };

        let provider = AWSProvider::new(config);
        assert_eq!(provider.name(), "AWS");
        assert_eq!(provider.provider_type(), Provider::AWS);
        assert_eq!(provider.current_region(), "us-east-1");
    }

    #[test]
    fn test_provider_regions() {
        let config = AwsConfig::default();
        let provider = AWSProvider::new(config);
        let regions = provider.regions();

        assert!(regions.contains(&"us-east-1".to_string()));
        assert!(regions.contains(&"eu-west-1".to_string()));
        assert!(regions.len() > 10);
    }

    #[tokio::test]
    async fn test_unauthenticated_error() {
        let config = AwsConfig::default();
        let provider = AWSProvider::new(config);

        let result = provider.list_all_resources().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NimbusError::AuthenticationFailed(_, _)));
    }
}