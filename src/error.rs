use thiserror::Error;

/// Error types for Nimbus operations.
#[derive(Debug, Error)]
pub enum NimbusError {
    /// General configuration error with context.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Failed to read a configuration file.
    #[error("Failed to read config file at {0}: {1}")]
    ConfigRead(std::path::PathBuf, std::io::Error),

    /// Failed to parse configuration (invalid TOML).
    #[error("Failed to parse configuration: {0}")]
    ConfigParse(#[from] toml::de::Error),

    /// Authentication with a cloud provider failed.
    #[error("Authentication failed for {0}: {1}")]
    AuthenticationFailed(String, String),

    /// A cloud provider API call failed.
    #[error("Provider error from {0}: {1}")]
    ProviderError(String, String),

    /// Requested resource was not found.
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// Attempted action is not supported for this resource type.
    #[error("Action {0:?} not supported for resource type {1:?}")]
    UnsupportedAction(crate::core::action::Action, crate::core::resource::ResourceType),

    /// Cache operation failed.
    #[error("Cache error: {0}")]
    CacheError(String),

    /// SQLite database operation failed.
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    /// Filesystem I/O operation failed.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// AWS credentials are missing or invalid.
    #[error("Invalid AWS credentials or profile")]
    InvalidAwsCredentials,

    /// Required configuration value is missing.
    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    /// Invalid region specified.
    #[error("Invalid region: {0}")]
    InvalidRegion(String),

    /// Catch-all for other errors.
    #[error("{0}")]
    Other(String),
}

impl NimbusError {
    /// Creates a generic configuration error.
    pub fn config<S: Into<String>>(msg: S) -> Self {
        NimbusError::ConfigError(msg.into())
    }

    /// Creates a provider error for a specific provider.
    pub fn provider<S1: Into<String>, S2: Into<String>>(provider: S1, msg: S2) -> Self {
        NimbusError::ProviderError(provider.into(), msg.into())
    }

    /// Creates an authentication error.
    pub fn auth<S1: Into<String>, S2: Into<String>>(provider: S1, msg: S2) -> Self {
        NimbusError::AuthenticationFailed(provider.into(), msg.into())
    }

    /// Returns true if this error is recoverable (can retry).
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            NimbusError::ProviderError(_, _) | NimbusError::CacheError(_)
        )
    }

    /// Returns true if this error is related to authentication.
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            NimbusError::AuthenticationFailed(_, _) | NimbusError::InvalidAwsCredentials
        )
    }
}

/// Convenient Result type for Nimbus operations.
pub type Result<T> = std::result::Result<T, NimbusError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Action, ResourceType};

    #[test]
    fn test_config_error() {
        let err = NimbusError::config("missing field");
        assert_eq!(err.to_string(), "Configuration error: missing field");
    }

    #[test]
    fn test_provider_error() {
        let err = NimbusError::provider("AWS", "API rate limit");
        assert_eq!(err.to_string(), "Provider error from AWS: API rate limit");
    }

    #[test]
    fn test_auth_error() {
        let err = NimbusError::auth("GCP", "invalid token");
        assert_eq!(
            err.to_string(),
            "Authentication failed for GCP: invalid token"
        );
    }

    #[test]
    fn test_unsupported_action_error() {
        let err = NimbusError::UnsupportedAction(Action::Start, ResourceType::Storage);
        assert!(err.to_string().contains("Start"));
        assert!(err.to_string().contains("Storage"));
    }

    #[test]
    fn test_resource_not_found_error() {
        let err = NimbusError::ResourceNotFound("i-1234567890".to_string());
        assert_eq!(err.to_string(), "Resource not found: i-1234567890");
    }

    #[test]
    fn test_is_recoverable() {
        let err = NimbusError::provider("AWS", "timeout");
        assert!(err.is_recoverable());

        let err = NimbusError::CacheError("disk full".to_string());
        assert!(err.is_recoverable());

        let err = NimbusError::InvalidAwsCredentials;
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_is_auth_error() {
        let err = NimbusError::InvalidAwsCredentials;
        assert!(err.is_auth_error());

        let err = NimbusError::auth("AWS", "bad credentials");
        assert!(err.is_auth_error());

        let err = NimbusError::config("missing file");
        assert!(!err.is_auth_error());
    }

    #[test]
    fn test_missing_config_error() {
        let err = NimbusError::MissingConfig("api_key".to_string());
        assert_eq!(err.to_string(), "Missing required configuration: api_key");
    }

    #[test]
    fn test_invalid_region_error() {
        let err = NimbusError::InvalidRegion("mars-north-1".to_string());
        assert_eq!(err.to_string(), "Invalid region: mars-north-1");
    }

    #[test]
    fn test_cache_error() {
        let err = NimbusError::CacheError("write failed".to_string());
        assert_eq!(err.to_string(), "Cache error: write failed");
    }

    #[test]
    fn test_other_error() {
        let err = NimbusError::Other("something went wrong".to_string());
        assert_eq!(err.to_string(), "something went wrong");
    }
}