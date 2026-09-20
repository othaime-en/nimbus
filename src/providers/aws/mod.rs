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
use resources::{EC2Instance, ELBLoadBalancer, RDSInstance, Route53Zone, S3Bucket};

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
            NimbusError::auth("AWS", "Client not initialized. Call authenticate() first.")
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

    async fn list_ec2_instances(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        let client = self.get_client()?;
        let response = client.ec2.describe_instances().send().await.map_err(|e| {
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

    async fn list_rds_instances(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        let client = self.get_client()?;
        let response = client
            .rds
            .describe_db_instances()
            .send()
            .await
            .map_err(|e| {
                NimbusError::provider("AWS", format!("Failed to list RDS instances: {}", e))
            })?;

        let mut instances: Vec<Box<dyn CloudResource>> = Vec::new();

        for db_instance in response.db_instances() {
            let rds_instance = RDSInstance::from_aws_instance(db_instance, &self.config.region);
            instances.push(Box::new(rds_instance));
        }

        Ok(instances)
    }

    async fn list_s3_buckets(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        let client = self.get_client()?;
        let response = client.s3.list_buckets().send().await.map_err(|e| {
            NimbusError::provider("AWS", format!("Failed to list S3 buckets: {}", e))
        })?;

        let mut buckets: Vec<Box<dyn CloudResource>> = Vec::new();

        for bucket in response.buckets() {
            if let Some(name) = bucket.name() {
                let created_at = bucket.creation_date().and_then(|dt| {
                    chrono::DateTime::parse_from_rfc3339(&dt.to_string())
                        .ok()
                        .map(|parsed| parsed.with_timezone(&chrono::Utc))
                });

                let tags = self.get_bucket_tags(name).await.unwrap_or_default();

                let s3_bucket = S3Bucket::new(
                    name.to_string(),
                    self.config.region.clone(),
                    created_at,
                    tags,
                );

                buckets.push(Box::new(s3_bucket));
            }
        }

        Ok(buckets)
    }

    async fn get_bucket_tags(
        &self,
        bucket_name: &str,
    ) -> Result<std::collections::HashMap<String, String>> {
        let client = self.get_client()?;

        match client
            .s3
            .get_bucket_tagging()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(response) => {
                let tags = response
                    .tag_set()
                    .iter()
                    .map(|tag| (tag.key().to_string(), tag.value().to_string()))
                    .collect();
                Ok(tags)
            }
            Err(_) => Ok(std::collections::HashMap::new()),
        }
    }

    async fn list_load_balancers(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        let client = self.get_client()?;
        let response = client
            .elb
            .describe_load_balancers()
            .send()
            .await
            .map_err(|e| {
                NimbusError::provider("AWS", format!("Failed to list load balancers: {}", e))
            })?;

        let mut load_balancers: Vec<Box<dyn CloudResource>> = Vec::new();

        for lb in response.load_balancers() {
            let elb = ELBLoadBalancer::from_aws_lb(lb, &self.config.region);

            if let Some(arn) = lb.load_balancer_arn() {
                let tags = self.get_lb_tags(arn).await.unwrap_or_default();
                load_balancers.push(Box::new(elb.with_tags(tags)));
            } else {
                load_balancers.push(Box::new(elb));
            }
        }

        Ok(load_balancers)
    }

    async fn get_lb_tags(&self, lb_arn: &str) -> Result<std::collections::HashMap<String, String>> {
        let client = self.get_client()?;

        match client
            .elb
            .describe_tags()
            .resource_arns(lb_arn)
            .send()
            .await
        {
            Ok(response) => {
                let tags = response
                    .tag_descriptions()
                    .iter()
                    .flat_map(|desc| desc.tags())
                    .filter_map(|tag| match (tag.key(), tag.value()) {
                        (Some(key), Some(value)) => Some((key.to_string(), value.to_string())),
                        _ => None,
                    })
                    .collect();
                Ok(tags)
            }
            Err(_) => Ok(std::collections::HashMap::new()),
        }
    }

    async fn list_route53_zones(&self) -> Result<Vec<Box<dyn CloudResource>>> {
        let client = self.get_client()?;
        let response = client
            .route53
            .list_hosted_zones()
            .send()
            .await
            .map_err(|e| {
                NimbusError::provider("AWS", format!("Failed to list Route53 zones: {}", e))
            })?;

        let mut zones: Vec<Box<dyn CloudResource>> = Vec::new();

        for zone in response.hosted_zones() {
            let route53_zone = Route53Zone::from_aws_zone(zone, "global");
            let zone_id = zone.id();
            let tags = self.get_zone_tags(zone_id).await.unwrap_or_default();
            zones.push(Box::new(route53_zone.with_tags(tags)));
        }

        Ok(zones)
    }

    async fn get_zone_tags(
        &self,
        zone_id: &str,
    ) -> Result<std::collections::HashMap<String, String>> {
        let client = self.get_client()?;

        match client
            .route53
            .list_tags_for_resource()
            .resource_type(aws_sdk_route53::types::TagResourceType::Hostedzone)
            .resource_id(zone_id)
            .send()
            .await
        {
            Ok(response) => {
                let tags = response
                    .resource_tag_set()
                    .map(|set| {
                        set.tags()
                            .iter()
                            .filter_map(|tag| match (tag.key(), tag.value()) {
                                (Some(key), Some(value)) => {
                                    Some((key.to_string(), value.to_string()))
                                }
                                _ => None,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(tags)
            }
            Err(_) => Ok(std::collections::HashMap::new()),
        }
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

        if let Ok(ec2) = self.list_ec2_instances().await {
            all_resources.extend(ec2);
        }

        if let Ok(rds) = self.list_rds_instances().await {
            all_resources.extend(rds);
        }

        if let Ok(s3) = self.list_s3_buckets().await {
            all_resources.extend(s3);
        }

        if let Ok(elb) = self.list_load_balancers().await {
            all_resources.extend(elb);
        }

        if let Ok(route53) = self.list_route53_zones().await {
            all_resources.extend(route53);
        }

        Ok(all_resources)
    }

    async fn list_resources_by_type(
        &self,
        resource_type: ResourceType,
    ) -> Result<Vec<Box<dyn CloudResource>>> {
        self.ensure_authenticated().await?;

        match resource_type {
            ResourceType::Compute => self.list_ec2_instances().await,
            ResourceType::Database => self.list_rds_instances().await,
            ResourceType::Storage => self.list_s3_buckets().await,
            ResourceType::LoadBalancer => self.list_load_balancers().await,
            ResourceType::DNS => self.list_route53_zones().await,
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
            let response = client
                .ec2
                .describe_instances()
                .instance_ids(id)
                .send()
                .await
                .map_err(|e| {
                    NimbusError::provider("AWS", format!("Failed to get instance {}: {}", id, e))
                })?;

            for reservation in response.reservations() {
                for instance in reservation.instances() {
                    if instance.instance_id() == Some(id) {
                        let ec2_instance =
                            EC2Instance::from_aws_instance(instance, &self.config.region);
                        return Ok(Box::new(ec2_instance));
                    }
                }
            }
        }

        // RDS/S3/ELB/Route53 lookup-by-ID isn't implemented yet -- nothing
        // in the app currently calls get_resource for those types, but this
        // no longer silently mis-routes to the wrong service the way the
        // old ID-shape guess did.
        Err(NimbusError::ResourceNotFound(id.to_string()))
    }

    /// Dispatches a lifecycle action to the resource module that owns it,
    /// keyed on the caller-supplied resource_type (the caller already knows
    /// this from the CloudResource it's acting on -- see the trait docs).
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
                resources::ec2::execute_action(client, resource_id, action).await
            }
            ResourceType::Database => {
                resources::rds::execute_action(client, resource_id, action).await
            }
            // S3/ELB/Route53 list Terminate in their supported_actions but
            // there's no delete implementation for them yet. Fail cleanly
            // and correctly-typed rather than silently routing to RDS the
            // way the old ID-shape guess did.
            other => Err(NimbusError::UnsupportedAction(action, other)),
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
    use crate::core::ResourceState;

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
    }

    // --- Mock-response tests -------------------------------------------
    //
    // These build a real AwsClient wired to a fake HTTP layer (via the AWS
    // SDK's own `StaticReplayClient` test utility) that replays a canned
    // response instead of making a network call. This exercises our actual
    // parsing/mapping code (EC2Instance::from_aws_instance, state mapping,
    // error mapping) against real AWS response shapes, without touching a
    // real account. Fixtures live in tests/fixtures/mock_responses/ so they
    // can be reused for other resource types later.

    use aws_config::{BehaviorVersion, Region, SdkConfig};
    use aws_credential_types::provider::SharedCredentialsProvider;
    use aws_credential_types::Credentials;
    use aws_smithy_runtime::client::http::test_util::{ReplayEvent, StaticReplayClient};
    use aws_smithy_types::body::SdkBody;
    use http::{Request, Response};

    /// Builds an AwsClient whose underlying HTTP layer replays a single
    /// canned response for any request it receives.
    fn mock_aws_client(status: u16, body: &str) -> AwsClient {
        mock_aws_client_sequence(vec![(status, body.to_string())])
    }

    /// Builds an AwsClient whose underlying HTTP layer replays a fixed
    /// sequence of canned responses, in order. Needed for resource types
    /// that make more than one call per list (e.g. a list call followed by
    /// a per-resource tag lookup) — StaticReplayClient hands out queued
    /// responses strictly in call order, shared across every sub-client
    /// on this AwsClient since they all ride the same HTTP client.
    fn mock_aws_client_sequence(responses: Vec<(u16, String)>) -> AwsClient {
        let events = responses
            .into_iter()
            .map(|(status, body)| {
                ReplayEvent::new(
                    Request::builder().body(SdkBody::empty()).unwrap(),
                    Response::builder()
                        .status(status)
                        .body(SdkBody::from(body))
                        .unwrap(),
                )
            })
            .collect();

        let replay = StaticReplayClient::new(events);

        let sdk_config = SdkConfig::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(SharedCredentialsProvider::new(Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "nimbus-test",
            )))
            .http_client(replay)
            .build();

        AwsClient::new(&sdk_config)
    }

    /// Builds an AWSProvider that's "authenticated" against a mock client,
    /// bypassing real credential lookup entirely.
    fn test_provider_with_client(client: AwsClient) -> AWSProvider {
        AWSProvider {
            name: "AWS".to_string(),
            config: AwsConfig::default(),
            sdk_config: Some(
                SdkConfig::builder()
                    .behavior_version(BehaviorVersion::latest())
                    .build(),
            ),
            client: Some(client),
            cost_explorer: None,
        }
    }

    #[tokio::test]
    async fn test_list_ec2_instances_parses_mock_response() {
        let xml = include_str!("../../../tests/fixtures/mock_responses/ec2_describe_instances.xml");
        let provider = test_provider_with_client(mock_aws_client(200, xml));

        let resources = provider
            .list_ec2_instances()
            .await
            .expect("mock response should parse successfully");

        assert_eq!(resources.len(), 2);

        assert_eq!(resources[0].id(), "i-0123456789abcdef0");
        assert_eq!(resources[0].name(), "web-server-1");
        assert_eq!(resources[0].state(), ResourceState::Running);

        assert_eq!(resources[1].id(), "i-0fedcba9876543210");
        assert_eq!(resources[1].name(), "db-cache-node");
        assert_eq!(resources[1].state(), ResourceState::Stopped);
    }

    #[tokio::test]
    async fn test_list_ec2_instances_maps_api_error() {
        let xml = include_str!("../../../tests/fixtures/mock_responses/ec2_internal_error.xml");
        let provider = test_provider_with_client(mock_aws_client(500, xml));

        let result = provider.list_ec2_instances().await;

        assert!(matches!(result, Err(NimbusError::ProviderError(_, _))));
    }

    #[tokio::test]
    async fn test_list_rds_instances_parses_mock_response() {
        let xml =
            include_str!("../../../tests/fixtures/mock_responses/rds_describe_db_instances.xml");
        let provider = test_provider_with_client(mock_aws_client(200, xml));

        let resources = provider
            .list_rds_instances()
            .await
            .expect("mock response should parse successfully");

        assert_eq!(resources.len(), 2);

        assert_eq!(resources[0].id(), "prod-postgres-1");
        assert_eq!(resources[0].name(), "Production Postgres"); // from Name tag
        assert_eq!(resources[0].state(), ResourceState::Running); // "available"

        // No Name tag on this one -- name should fall back to the identifier.
        assert_eq!(resources[1].id(), "staging-mysql-1");
        assert_eq!(resources[1].name(), "staging-mysql-1");
        assert_eq!(resources[1].state(), ResourceState::Stopped);
    }

    #[tokio::test]
    async fn test_list_s3_buckets_parses_mock_response_and_tolerates_missing_tags() {
        let list_xml = include_str!("../../../tests/fixtures/mock_responses/s3_list_buckets.xml");
        let tags_xml =
            include_str!("../../../tests/fixtures/mock_responses/s3_get_bucket_tagging.xml");
        let no_tags_xml = include_str!(
            "../../../tests/fixtures/mock_responses/s3_get_bucket_tagging_not_found.xml"
        );

        // Call order: ListBuckets, then GetBucketTagging once per bucket.
        let provider = test_provider_with_client(mock_aws_client_sequence(vec![
            (200, list_xml.to_string()),
            (200, tags_xml.to_string()),
            (404, no_tags_xml.to_string()),
        ]));

        let resources = provider
            .list_s3_buckets()
            .await
            .expect("mock response should parse successfully");

        assert_eq!(resources.len(), 2);

        assert_eq!(resources[0].id(), "nimbus-app-assets");
        assert_eq!(
            resources[0].tags().get("Environment").map(String::as_str),
            Some("production")
        );

        // Second bucket's tagging call 404s (no tags configured); our code
        // treats that as "no tags" rather than failing the whole list.
        assert_eq!(resources[1].id(), "nimbus-backups");
        assert!(resources[1].tags().is_empty());
    }

    #[tokio::test]
    async fn test_list_load_balancers_parses_mock_response() {
        let lb_xml =
            include_str!("../../../tests/fixtures/mock_responses/elb_describe_load_balancers.xml");
        let tags_xml = include_str!("../../../tests/fixtures/mock_responses/elb_describe_tags.xml");

        // Call order: DescribeLoadBalancers, then DescribeTags for the one LB.
        let provider = test_provider_with_client(mock_aws_client_sequence(vec![
            (200, lb_xml.to_string()),
            (200, tags_xml.to_string()),
        ]));

        let resources = provider
            .list_load_balancers()
            .await
            .expect("mock response should parse successfully");

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name(), "nimbus-web-alb");
        assert_eq!(resources[0].state(), ResourceState::Running); // "active"
        assert_eq!(
            resources[0].tags().get("Team").map(String::as_str),
            Some("platform")
        );
    }

    #[tokio::test]
    async fn test_list_route53_zones_parses_mock_response() {
        let zones_xml =
            include_str!("../../../tests/fixtures/mock_responses/route53_list_hosted_zones.xml");
        let tags_xml = include_str!(
            "../../../tests/fixtures/mock_responses/route53_list_tags_for_resource.xml"
        );

        // Call order: ListHostedZones, then ListTagsForResource for the one zone.
        let provider = test_provider_with_client(mock_aws_client_sequence(vec![
            (200, zones_xml.to_string()),
            (200, tags_xml.to_string()),
        ]));

        let resources = provider
            .list_route53_zones()
            .await
            .expect("mock response should parse successfully");

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].id(), "/hostedzone/Z1D633PJN98FT9");
        assert_eq!(resources[0].name(), "nimbus-app.com.");
        assert_eq!(
            resources[0].tags().get("Name").map(String::as_str),
            Some("Nimbus App Zone")
        );
    }
}
