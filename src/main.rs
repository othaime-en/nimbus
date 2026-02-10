use nimbus::{NimbusConfig, Result};
use log::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("Nimbus - Cloud Resource Manager");
    info!("Starting application...");
    
    let config = match NimbusConfig::load() {
        Ok(config) => {
            info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            info!("Using default configuration");
            NimbusConfig::default()
        }
    };
    
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        error!("Please configure at least one cloud provider.");
        error!("Configuration file location: {:?}", NimbusConfig::config_file_path());
        return Err(e);
    }
    
    info!("Configuration validated successfully");
    
    if config.providers.aws.is_some() {
        info!("AWS provider configured");
    }
    if config.providers.gcp.is_some() {
        info!("GCP provider configured");
    }
    if config.providers.azure.is_some() {
        info!("Azure provider configured");
    }
    
    println!("\n✓ Nimbus initialized successfully!");
    println!("✓ Configuration loaded and validated");
    println!("\nPhase 1.1 Complete: Project setup and configuration loading working!");
    println!("\nNext steps:");
    println!("  - Phase 1.2: Implement core abstractions");
    println!("  - Phase 1.3: Build TUI interface");
    println!("  - Phase 1.4: Add AWS provider integration");
    
    Ok(())
}