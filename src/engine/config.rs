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
        }
    }
}

impl StorageConfig {
    pub fn sled_dir(&self) -> PathBuf {
        let mut dir = self.data_dir.clone();
        dir.push("sled");
        dir
    }

    pub fn duckdb_path(&self) -> PathBuf {
        let mut path = self.data_dir.clone();
        path.push("duckdb");
        path.push("evento.db");
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_paths() {
        let config = StorageConfig::default();
        let duckdb = config.duckdb_path();
        let sled = config.sled_dir();
        
        assert!(duckdb.ends_with("evento.db"));
        assert!(sled.ends_with("sled"));
    }
}
