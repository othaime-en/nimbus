use std::path::PathBuf;
use std::fs;
use crate::error::{NimbusError, Result};

pub struct AwsProfileDetector;

impl AwsProfileDetector {
    pub fn detect_profiles() -> Result<Vec<String>> {
        let credentials_path = Self::credentials_path()?;
        
        if !credentials_path.exists() {
            return Ok(vec!["default".to_string()]);
        }
        
        let contents = fs::read_to_string(&credentials_path)
            .map_err(|e| NimbusError::ConfigError(format!("Failed to read AWS credentials: {}", e)))?;
        
        let profiles = Self::parse_profiles(&contents);
        
        if profiles.is_empty() {
            Ok(vec!["default".to_string()])
        } else {
            Ok(profiles)
        }
    }

    pub fn get_profile_config(profile: &str) -> Result<ProfileConfig> {
        let credentials_path = Self::credentials_path()?;
        
        if !credentials_path.exists() {
            return Err(NimbusError::InvalidAwsCredentials);
        }
        
        let contents = fs::read_to_string(&credentials_path)
            .map_err(|e| NimbusError::ConfigError(format!("Failed to read AWS credentials: {}", e)))?;
        
        Self::parse_profile_config(&contents, profile)
    }

    fn credentials_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| NimbusError::ConfigError("Could not determine home directory".to_string()))?;
        
        Ok(home.join(".aws").join("credentials"))
    }

    fn parse_profiles(contents: &str) -> Vec<String> {
        let mut profiles = Vec::new();
        
        for line in contents.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                let profile = &line[1..line.len()-1];
                profiles.push(profile.to_string());
            }
        }
        
        profiles
    }

    fn parse_profile_config(contents: &str, profile: &str) -> Result<ProfileConfig> {
        let profile_header = format!("[{}]", profile);
        let mut in_profile = false;
        let mut access_key_id = None;
        let mut secret_access_key = None;
        let mut region = None;
        
        for line in contents.lines() {
            let line = line.trim();
            
            if line == profile_header {
                in_profile = true;
                continue;
            }
            
            if line.starts_with('[') {
                in_profile = false;
            }
            
            if in_profile {
                if let Some(key_val) = line.split_once('=') {
                    let key = key_val.0.trim();
                    let val = key_val.1.trim();
                    
                    match key {
                        "aws_access_key_id" => access_key_id = Some(val.to_string()),
                        "aws_secret_access_key" => secret_access_key = Some(val.to_string()),
                        "region" => region = Some(val.to_string()),
                        _ => {}
                    }
                }
            }
        }
        
        if access_key_id.is_none() || secret_access_key.is_none() {
            return Err(NimbusError::ConfigError(
                format!("Profile '{}' is missing required credentials", profile)
            ));
        }
        
        Ok(ProfileConfig {
            access_key_id: access_key_id.unwrap(),
            secret_access_key: secret_access_key.unwrap(),
            region,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ProfileConfig {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_profiles() {
        let contents = r#"
[default]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

[production]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE2
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2
        "#;
        
        let profiles = AwsProfileDetector::parse_profiles(contents);
        assert_eq!(profiles.len(), 2);
        assert!(profiles.contains(&"default".to_string()));
        assert!(profiles.contains(&"production".to_string()));
    }

    #[test]
    fn test_parse_profile_config() {
        let contents = r#"
[default]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
region = us-west-2
        "#;
        
        let config = AwsProfileDetector::parse_profile_config(contents, "default").unwrap();
        assert_eq!(config.access_key_id, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(config.region, Some("us-west-2".to_string()));
    }
}