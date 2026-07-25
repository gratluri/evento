use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Root directory for all persistent data. Default: ~/.evento/data
    pub data_dir: PathBuf,
    
    /// Sled cache capacity in bytes (default: 1GB = 1024 * 1024 * 1024)
    pub sled_cache_capacity: u64,
    
    /// Sled flush interval in milliseconds (default: 2000)
    pub sled_flush_interval_ms: u64,
    
    /// External Postgres URL for cold analytics data
    pub postgres_url: String,
    
    /// Interval in milliseconds to run background health checks (default: 60000)
    pub health_check_interval_ms: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let mut data_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        data_dir.push(".evento");
        data_dir.push("data");

        Self {
            data_dir,
            sled_cache_capacity: 1024 * 1024 * 1024,
            sled_flush_interval_ms: 2000,
            health_check_interval_ms: 60_000,
            postgres_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/evento".to_string()),
        }
    }
}

impl StorageConfig {
    #[must_use]
    pub fn sled_dir(&self) -> PathBuf {
        self.data_dir.join("sled")
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_paths() {
        let config = StorageConfig::default();
        let sled = config.sled_dir();
        
        assert!(sled.ends_with("sled"));
    }
}
