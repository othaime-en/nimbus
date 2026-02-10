use thiserror::Error;

#[derive(Debug, Error)]
pub enum NimbusError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Failed to read config file at {0}: {1}")]
    ConfigRead(std::path::PathBuf, std::io::Error),

    #[error("Failed to parse configuration: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("Authentication failed for {0}: {1}")]
    AuthenticationFailed(String, String),

    #[error("Provider error from {0}: {1}")]
    ProviderError(String, String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Action {0:?} not supported for resource type {1:?}")]
    UnsupportedAction(crate::core::action::Action, crate::core::resource::ResourceType),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid AWS credentials or profile")]
    InvalidAwsCredentials,

    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    #[error("Invalid region: {0}")]
    InvalidRegion(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, NimbusError>;