use nimbus::{NimbusConfig, Result};
use tempfile::TempDir;
use std::fs;

#[test]
fn test_default_config() {
    let config = NimbusConfig::default();
    assert!(config.cache.enabled);
    assert_eq!(config.cache.max_age_hours, 24);
    assert_eq!(config.refresh.interval_seconds, 300);
}

#[test]
fn test_config_from_file() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    let config_content = r#"
[providers.aws]
profile = "test"
region = "us-west-2"

[ui]
default_tab = "aws"
auto_refresh = false

[cache]
enabled = false
    "#;
    
    fs::write(&config_path, config_content).unwrap();
    
    let config = NimbusConfig::from_file(&config_path)?;
    
    assert!(config.providers.aws.is_some());
    let aws_config = config.providers.aws.unwrap();
    assert_eq!(aws_config.profile, Some("test".to_string()));
    assert_eq!(aws_config.region, "us-west-2");
    assert!(!config.ui.auto_refresh);
    assert!(!config.cache.enabled);
    
    Ok(())
}

#[test]
fn test_config_validation() {
    let mut config = NimbusConfig::default();
    
    let result = config.validate();
    assert!(result.is_err());
    
    config.providers.aws = Some(nimbus::config::AwsConfig::default());
    
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_config_merge() {
    let mut config1 = NimbusConfig::default();
    config1.providers.aws = Some(nimbus::config::AwsConfig {
        profile: Some("default".to_string()),
        region: "us-east-1".to_string(),
        access_key_id: None,
        secret_access_key: None,
    });
    
    let mut config2 = NimbusConfig::default();
    config2.providers.aws = Some(nimbus::config::AwsConfig {
        profile: Some("production".to_string()),
        region: "us-west-2".to_string(),
        access_key_id: None,
        secret_access_key: None,
    });
    
    let merged = config1.merge(config2);
    
    let aws_config = merged.providers.aws.unwrap();
    assert_eq!(aws_config.profile, Some("production".to_string()));
    assert_eq!(aws_config.region, "us-west-2");
}