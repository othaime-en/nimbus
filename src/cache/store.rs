use crate::core::{CloudResource, Provider, ResourceType};
use crate::error::{NimbusError, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResource {
    pub id: String,
    pub provider: Provider,
    pub resource_type: ResourceType,
    pub data: String,
    pub cached_at: DateTime<Utc>,
}

pub struct CacheStore {
    conn: Connection,
    max_age: Duration,
}

impl CacheStore {
    pub fn new(db_path: &Path, max_age_hours: u64) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;
        let store = Self {
            conn,
            max_age: Duration::hours(max_age_hours as i64),
        };

        store.initialize_schema()?;
        Ok(store)
    }

    pub fn initialize_schema(&self) -> Result<()> {
        let schema = include_str!("schema.sql");
        self.conn.execute_batch(schema)?;
        Ok(())
    }

    pub fn cache_resource(&self, resource: &dyn CloudResource) -> Result<()> {
        let serialized_data = serde_json::to_string(&SerializableResource::from_resource(resource))
            .map_err(|e| NimbusError::CacheError(format!("Failed to serialize resource: {}", e)))?;

        let cached_at = Utc::now().timestamp();

        self.conn.execute(
            "INSERT OR REPLACE INTO resources (id, provider, resource_type, region, data, cached_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                resource.id(),
                resource.provider().as_str(),
                resource.resource_type().as_str(),
                resource.region(),
                serialized_data,
                cached_at,
            ],
        )?;

        Ok(())
    }

    pub fn cache_resources(&self, resources: &[Box<dyn CloudResource>]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        for resource in resources {
            let serialized_data = serde_json::to_string(&SerializableResource::from_resource(resource.as_ref()))
                .map_err(|e| NimbusError::CacheError(format!("Failed to serialize resource: {}", e)))?;

            let cached_at = Utc::now().timestamp();

            tx.execute(
                "INSERT OR REPLACE INTO resources (id, provider, resource_type, region, data, cached_at) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    resource.id(),
                    resource.provider().as_str(),
                    resource.resource_type().as_str(),
                    resource.region(),
                    serialized_data,
                    cached_at,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_cached_resources(&self, provider: Provider) -> Result<Vec<CachedResource>> {
        let cutoff_time = (Utc::now() - self.max_age).timestamp();

        let mut stmt = self.conn.prepare(
            "SELECT id, provider, resource_type, data, cached_at 
             FROM resources 
             WHERE provider = ?1 AND cached_at > ?2
             ORDER BY cached_at DESC",
        )?;

        let resources = stmt
            .query_map(params![provider.as_str(), cutoff_time], |row| {
                let provider_str: String = row.get(1)?;
                let type_str: String = row.get(2)?;
                let cached_at_timestamp: i64 = row.get(4)?;

                Ok(CachedResource {
                    id: row.get(0)?,
                    provider: parse_provider(&provider_str),
                    resource_type: parse_resource_type(&type_str),
                    data: row.get(3)?,
                    cached_at: DateTime::from_timestamp(cached_at_timestamp, 0)
                        .unwrap_or_else(|| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(resources)
    }

    pub fn get_all_cached_resources(&self) -> Result<Vec<CachedResource>> {
        let cutoff_time = (Utc::now() - self.max_age).timestamp();

        let mut stmt = self.conn.prepare(
            "SELECT id, provider, resource_type, data, cached_at 
             FROM resources 
             WHERE cached_at > ?1
             ORDER BY cached_at DESC",
        )?;

        let resources = stmt
            .query_map(params![cutoff_time], |row| {
                let provider_str: String = row.get(1)?;
                let type_str: String = row.get(2)?;
                let cached_at_timestamp: i64 = row.get(4)?;

                Ok(CachedResource {
                    id: row.get(0)?,
                    provider: parse_provider(&provider_str),
                    resource_type: parse_resource_type(&type_str),
                    data: row.get(3)?,
                    cached_at: DateTime::from_timestamp(cached_at_timestamp, 0)
                        .unwrap_or_else(|| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(resources)
    }

    pub fn get_last_sync_time(&self, provider: Provider) -> Result<Option<DateTime<Utc>>> {
        let mut stmt = self.conn.prepare(
            "SELECT MAX(cached_at) FROM resources WHERE provider = ?1",
        )?;

        let timestamp: Option<i64> = stmt.query_row(params![provider.as_str()], |row| row.get(0)).ok();

        Ok(timestamp.and_then(|ts| DateTime::from_timestamp(ts, 0)))
    }

    pub fn get_cache_count(&self) -> Result<usize> {
        let count: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM resources", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn clear_cache(&self, provider: Option<Provider>) -> Result<()> {
        match provider {
            Some(p) => {
                self.conn.execute(
                    "DELETE FROM resources WHERE provider = ?1",
                    params![p.as_str()],
                )?;
            }
            None => {
                self.conn.execute("DELETE FROM resources", [])?;
            }
        }
        Ok(())
    }

    pub fn prune_old_entries(&self, max_age: Duration) -> Result<usize> {
        let cutoff_time = (Utc::now() - max_age).timestamp();

        let deleted = self.conn.execute(
            "DELETE FROM resources WHERE cached_at < ?1",
            params![cutoff_time],
        )?;

        Ok(deleted)
    }

    pub fn is_cache_stale(&self, provider: Provider) -> Result<bool> {
        match self.get_last_sync_time(provider)? {
            Some(last_sync) => {
                let age = Utc::now().signed_duration_since(last_sync);
                Ok(age > self.max_age)
            }
            None => Ok(true),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializableResource {
    id: String,
    name: String,
    resource_type: String,
    provider: String,
    region: String,
    state: String,
    cost_per_month: Option<f64>,
    tags: std::collections::HashMap<String, String>,
    created_at: Option<DateTime<Utc>>,
}

impl SerializableResource {
    fn from_resource(resource: &dyn CloudResource) -> Self {
        Self {
            id: resource.id().to_string(),
            name: resource.name().to_string(),
            resource_type: resource.resource_type().as_str().to_string(),
            provider: resource.provider().as_str().to_string(),
            region: resource.region().to_string(),
            state: resource.state().as_str().to_string(),
            cost_per_month: resource.cost_per_month(),
            tags: resource.tags().clone(),
            created_at: resource.created_at(),
        }
    }
}

fn parse_provider(s: &str) -> Provider {
    match s {
        "AWS" => Provider::AWS,
        "GCP" => Provider::GCP,
        "Azure" => Provider::Azure,
        _ => Provider::AWS,
    }
}

fn parse_resource_type(s: &str) -> ResourceType {
    match s {
        "Compute" => ResourceType::Compute,
        "Database" => ResourceType::Database,
        "Storage" => ResourceType::Storage,
        "Load Balancer" => ResourceType::LoadBalancer,
        "DNS" => ResourceType::DNS,
        "Container" => ResourceType::Container,
        "Serverless" => ResourceType::Serverless,
        "Network" => ResourceType::Network,
        "Cache" => ResourceType::Cache,
        "Queue" => ResourceType::Queue,
        _ => ResourceType::Compute,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_store_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let store = CacheStore::new(&db_path, 24).unwrap();
        assert!(db_path.exists());

        let count = store.get_cache_count().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = CacheStore::new(&db_path, 24).unwrap();

        store.clear_cache(None).unwrap();
        let count = store.get_cache_count().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_prune_old_entries() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = CacheStore::new(&db_path, 24).unwrap();

        let deleted = store.prune_old_entries(Duration::hours(1)).unwrap();
        assert_eq!(deleted, 0);
    }
}