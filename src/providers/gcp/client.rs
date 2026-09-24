use crate::error::{NimbusError, Result};
use google_cloud_auth::credentials::Credentials;
use google_cloud_compute_v1::client::Instances;
use google_cloud_sql_v1::client::SqlInstancesService;
use google_cloud_storage::client::StorageControl;

/// Bundles the per-service GCP clients Nimbus talks to.
///
/// Each of these is a generated `google-cloud-rust` client (the GCP
/// equivalent of this project's `AwsClient` bundle in
/// `providers/aws/client.rs`). Like the AWS SDK clients, each holds its own
/// internal connection pool and is cheap to keep around for the lifetime of
/// the provider.
pub struct GcpClient {
    pub instances: Instances,
    pub sql_instances: SqlInstancesService,
    pub storage_control: StorageControl,
}

impl GcpClient {
    /// Builds all three service clients against the same credentials.
    /// `credentials: None` lets each client fall back to its own
    /// Application Default Credentials resolution.
    pub async fn new(credentials: Option<Credentials>) -> Result<Self> {
        let instances = match &credentials {
            Some(creds) => Instances::builder().with_credentials(creds.clone()).build().await,
            None => Instances::builder().build().await,
        }
        .map_err(|e| {
            NimbusError::auth("GCP", format!("Failed to build Compute Engine client: {}", e))
        })?;

        let sql_instances = match &credentials {
            Some(creds) => SqlInstancesService::builder()
                .with_credentials(creds.clone())
                .build()
                .await,
            None => SqlInstancesService::builder().build().await,
        }
        .map_err(|e| NimbusError::auth("GCP", format!("Failed to build Cloud SQL client: {}", e)))?;

        let storage_control = match &credentials {
            Some(creds) => StorageControl::builder()
                .with_credentials(creds.clone())
                .build()
                .await,
            None => StorageControl::builder().build().await,
        }
        .map_err(|e| {
            NimbusError::auth("GCP", format!("Failed to build Cloud Storage client: {}", e))
        })?;

        Ok(Self {
            instances,
            sql_instances,
            storage_control,
        })
    }
}
