use crate::config::AwsConfig;
use crate::error::{NimbusError, Result};
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_credential_types::Credentials;

pub struct AwsAuth;

impl AwsAuth {
    pub async fn create_config(aws_config: &AwsConfig) -> Result<SdkConfig> {
        let region = Region::new(aws_config.region.clone());

        if let (Some(access_key), Some(secret_key)) =
            (&aws_config.access_key_id, &aws_config.secret_access_key)
        {
            let creds = Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "nimbus-static-credentials",
            );
            let creds_provider = SharedCredentialsProvider::new(creds);

            let config = aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .credentials_provider(creds_provider)
                .load()
                .await;

            return Ok(config);
        }

        if let Some(profile) = &aws_config.profile {
            let config = aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .profile_name(profile)
                .load()
                .await;

            return Ok(config);
        }

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .load()
            .await;

        Ok(config)
    }

    pub async fn test_credentials(config: &SdkConfig) -> Result<bool> {
        let sts_client = aws_sdk_ec2::Client::new(config);

        match sts_client.describe_regions().send().await {
            Ok(_) => Ok(true),
            Err(e) => Err(NimbusError::auth(
                "AWS",
                format!("Failed to verify credentials: {}", e),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_config_with_defaults() {
        let aws_config = AwsConfig {
            profile: None,
            region: "us-east-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
        };

        let result = AwsAuth::create_config(&aws_config).await;
        assert!(result.is_ok());
    }
}