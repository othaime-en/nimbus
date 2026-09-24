use crate::config::GcpConfig;
use crate::error::{NimbusError, Result};
use google_cloud_auth::credentials::{service_account, Credentials};

/// Resolves GCP credentials from Nimbus configuration.
///
/// Mirrors the AWS credential priority pattern in spirit, but GCP's own
/// Application Default Credentials (ADC) chain already covers most of it
/// (GOOGLE_APPLICATION_CREDENTIALS env var, gcloud's local login, and the
/// GCE/GKE metadata server, in that order) -- so the only thing Nimbus
/// needs to handle itself is the explicit `credentials_file` config option,
/// which takes priority over ADC when set.
pub struct GcpAuth;

impl GcpAuth {
    /// Builds explicit credentials from `config.credentials_file` if set.
    /// Returns `Ok(None)` when no file is configured, telling callers to
    /// let each GCP client fall back to its own default ADC resolution
    /// rather than Nimbus duplicating that logic.
    pub async fn build_credentials(config: &GcpConfig) -> Result<Option<Credentials>> {
        let Some(path) = &config.credentials_file else {
            return Ok(None);
        };

        let key_json = tokio::fs::read_to_string(path).await.map_err(|e| {
            NimbusError::auth(
                "GCP",
                format!("Could not read service account file {}: {}", path, e),
            )
        })?;

        let key_value: serde_json::Value = serde_json::from_str(&key_json).map_err(|e| {
            NimbusError::auth(
                "GCP",
                format!("Service account file {} is not valid JSON: {}", path, e),
            )
        })?;

        let credentials = service_account::Builder::new(key_value)
            .build()
            .map_err(|e| {
                NimbusError::auth(
                    "GCP",
                    format!("Invalid service account key in {}: {}", path, e),
                )
            })?;

        Ok(Some(credentials))
    }

    /// Fails fast on a missing project ID rather than letting every
    /// subsequent API call surface a confusing "project not found" error.
    pub fn require_project_id(config: &GcpConfig) -> Result<()> {
        if config.project_id.trim().is_empty() {
            return Err(NimbusError::MissingConfig(
                "providers.gcp.project_id".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_project_id_missing() {
        let config = GcpConfig {
            project_id: String::new(),
            credentials_file: None,
            region: "us-central1".to_string(),
        };
        assert!(GcpAuth::require_project_id(&config).is_err());
    }

    #[test]
    fn test_require_project_id_present() {
        let config = GcpConfig {
            project_id: "my-project".to_string(),
            credentials_file: None,
            region: "us-central1".to_string(),
        };
        assert!(GcpAuth::require_project_id(&config).is_ok());
    }

    #[tokio::test]
    async fn test_build_credentials_none_when_unset() {
        let config = GcpConfig {
            project_id: "my-project".to_string(),
            credentials_file: None,
            region: "us-central1".to_string(),
        };
        let result = GcpAuth::build_credentials(&config).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_build_credentials_missing_file_errors() {
        let config = GcpConfig {
            project_id: "my-project".to_string(),
            credentials_file: Some("/nonexistent/key.json".to_string()),
            region: "us-central1".to_string(),
        };
        let result = GcpAuth::build_credentials(&config).await;
        assert!(matches!(result, Err(NimbusError::AuthenticationFailed(_, _))));
    }
}
