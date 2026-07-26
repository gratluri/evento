use crate::engine::config::StorageConfig;
use anyhow::{Context, Result};
use std::fs;

#[derive(Debug)]
pub struct SledStore {
    db: sled::Db,
}

impl SledStore {
    pub fn new(config: &StorageConfig) -> Result<Self> {
        let dir = config.sled_dir();
        
        // Ensure directory exists
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to create sled directory at {:?}", dir))?;
        }

        let db = sled::Config::new()
            .path(&dir)
            .cache_capacity(config.sled_cache_capacity)
            .flush_every_ms(Some(config.sled_flush_interval_ms))
            .open()
            .with_context(|| format!("Failed to open Sled database at {:?}", dir))?;

        Ok(Self { db })
    }

    /// Store a run state machine entry
    pub fn set_run_state(&self, run_id: &str, state_json: &str) -> Result<()> {
        let key = format!("run:{}:state", run_id);
        self.db.insert(key, state_json.as_bytes())?;
        Ok(())
    }

    /// Retrieve a run state machine entry
    pub fn get_run_state(&self, run_id: &str) -> Result<Option<String>> {
        let key = format!("run:{}:state", run_id);
        if let Some(ivec) = self.db.get(key)? {
            let str_val = String::from_utf8(ivec.to_vec())?;
            Ok(Some(str_val))
        } else {
            Ok(None)
        }
    }

    /// Set a Virtual User context variable (for `extract`)
    pub fn set_vu_context(&self, run_id: &str, vu_id: u32, key: &str, value: &str) -> Result<()> {
        let db_key = format!("run:{}:ctx:{}:{}", run_id, vu_id, key);
        self.db.insert(db_key, value.as_bytes())?;
        Ok(())
    }

    /// Set a Virtual User context variable
    pub fn get_vu_context(&self, run_id: &str, vu_id: u32, key: &str) -> Result<Option<String>> {
        let db_key = format!("run:{}:ctx:{}:{}", run_id, vu_id, key);
        if let Some(ivec) = self.db.get(db_key)? {
            let str_val = String::from_utf8(ivec.to_vec())?;
            Ok(Some(str_val))
        } else {
            Ok(None)
        }
    }

    /// Store a run plan
    pub fn store_run(&self, run_id: &str, plan_json: &str) -> Result<()> {
        let key = format!("plan:{}", run_id);
        self.db.insert(key, plan_json.as_bytes())?;
        Ok(())
    }

    /// Retrieve a run plan
    pub fn get_run(&self, run_id: &str) -> Result<Option<String>> {
        let key = format!("plan:{}", run_id);
        if let Some(ivec) = self.db.get(key)? {
            let str_val = String::from_utf8(ivec.to_vec())?;
            Ok(Some(str_val))
        } else {
            Ok(None)
        }
    }

    /// List all run IDs
    pub fn list_runs(&self) -> Result<Vec<String>> {
        let mut runs = Vec::new();
        // Sled provides a scan prefix method
        for result in self.db.scan_prefix("plan:") {
            let (k, _) = result?;
            let k_str = String::from_utf8(k.to_vec())?;
            if let Some(run_id) = k_str.strip_prefix("plan:") {
                runs.push(run_id.to_string());
            }
        }
        Ok(runs)
    }

    /// Retrieve all step results for a specific run
    pub fn get_run_results(&self, run_id: &str) -> Result<Vec<String>> {
        let mut results = Vec::new();
        let prefix = format!("run:{}:ctx:", run_id);
        
        for result in self.db.scan_prefix(&prefix) {
            let (_, v) = result?;
            let v_str = String::from_utf8(v.to_vec())?;
            // A simple heuristic: if it deserializes to StepResult (has status, task_id etc)
            if v_str.contains("\"task_id\"") && v_str.contains("\"status\"") {
                results.push(v_str);
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sled_store_lifecycle() {
        let temp = tempdir().unwrap();
        let mut config = StorageConfig::default();
        config.data_dir = temp.path().to_path_buf();

        let store = SledStore::new(&config).expect("Failed to initialize SledStore");

        // Test run state
        store.set_run_state("test-run-123", r#"{"status":"running"}"#).unwrap();
        let state = store.get_run_state("test-run-123").unwrap();
        assert_eq!(state.unwrap(), r#"{"status":"running"}"#);

        // Test VU context
        store.set_vu_context("test-run-123", 42, "auth_token", "secret123").unwrap();
        let ctx = store.get_vu_context("test-run-123", 42, "auth_token").unwrap();
        assert_eq!(ctx.unwrap(), "secret123");

        // Test store/list runs
        store.store_run("test-run-123", r#"{"run_id":"test-run-123"}"#).unwrap();
        let plan = store.get_run("test-run-123").unwrap();
        assert_eq!(plan.unwrap(), r#"{"run_id":"test-run-123"}"#);

        let runs = store.list_runs().unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0], "test-run-123");
    }
}
