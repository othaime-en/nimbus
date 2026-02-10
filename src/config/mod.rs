use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::error::{NimbusError, Result};

pub mod aws_profile;
pub use aws_profile::AwsProfileDetector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NimbusConfig {
    #[serde(default)]
    pub providers: ProviderConfigs,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub refresh: RefreshConfig,
}

impl NimbusConfig {
    pub fn load() -> Result<Self> {
        if let Some(config_path) = Self::config_file_path() {
            if config_path.exists() {
                return Self::from_file(&config_path);
            }
        }
        
        let from_env = Self::from_env()?;
        Ok(from_env)
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| NimbusError::ConfigRead(path.to_path_buf(), e))?;
        
        toml::from_str(&contents).map_err(NimbusError::ConfigParse)
    }

    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();
        
        if let Ok(profile) = std::env::var("NIMBUS_AWS_PROFILE") {
            config.providers.aws.get_or_insert_with(AwsConfig::default).profile = Some(profile);
        }
        
        if let Ok(region) = std::env::var("NIMBUS_AWS_REGION") {
            config.providers.aws.get_or_insert_with(AwsConfig::default).region = region;
        }
        
        if let Ok(enabled) = std::env::var("NIMBUS_CACHE_ENABLED") {
            config.cache.enabled = enabled.parse().unwrap_or(true);
        }
        
        Ok(config)
    }

    pub fn merge(mut self, other: Self) -> Self {
        if other.providers.aws.is_some() {
            self.providers.aws = other.providers.aws;
        }
        if other.providers.gcp.is_some() {
            self.providers.gcp = other.providers.gcp;
        }
        if other.providers.azure.is_some() {
            self.providers.azure = other.providers.azure;
        }
        
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.providers.aws.is_none() 
            && self.providers.gcp.is_none() 
            && self.providers.azure.is_none() {
            return Err(NimbusError::ConfigError(
                "At least one cloud provider must be configured".to_string()
            ));
        }
        
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| NimbusError::ConfigError(format!("Failed to serialize config: {}", e)))?;
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(path, contents)?;
        Ok(())
    }

    pub fn config_file_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".nimbus").join("config.toml"))
    }

    pub fn config_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".nimbus"))
    }
}

impl Default for NimbusConfig {
    fn default() -> Self {
        Self {
            providers: ProviderConfigs::default(),
            ui: UiConfig::default(),
            cache: CacheConfig::default(),
            refresh: RefreshConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfigs {
    pub aws: Option<AwsConfig>,
    pub gcp: Option<GcpConfig>,
    pub azure: Option<AzureConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsConfig {
    pub profile: Option<String>,
    pub region: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}

impl Default for AwsConfig {
    fn default() -> Self {
        Self {
            profile: None,
            region: "us-east-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpConfig {
    pub project_id: String,
    pub credentials_file: Option<String>,
    pub region: String,
}

impl Default for GcpConfig {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            credentials_file: None,
            region: "us-central1".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureConfig {
    pub subscription_id: String,
    pub tenant_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

impl Default for AzureConfig {
    fn default() -> Self {
        Self {
            subscription_id: String::new(),
            tenant_id: None,
            client_id: None,
            client_secret: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub default_tab: String,
    pub auto_refresh: bool,
    pub confirm_destructive_actions: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            default_tab: "aws".to_string(),
            auto_refresh: true,
            confirm_destructive_actions: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_age_hours: u64,
    pub db_path: Option<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_age_hours: 24,
            db_path: None,
        }
    }
}

impl CacheConfig {
    pub fn get_db_path(&self) -> PathBuf {
        if let Some(ref path) = self.db_path {
            PathBuf::from(path)
        } else {
            NimbusConfig::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("cache.db")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshConfig {
    pub interval_seconds: u64,
    pub auto_refresh_on_focus: bool,
}

impl Default for RefreshConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 300,
            auto_refresh_on_focus: true,
        }
    }
}