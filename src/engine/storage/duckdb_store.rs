use crate::engine::config::StorageConfig;
use anyhow::{Context, Result};
use duckdb::{Connection, params};
use std::fs;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug)]
pub struct DuckDbStore {
    conn: Connection,
}

impl DuckDbStore {
    pub fn new(config: &StorageConfig) -> Result<Self> {
        let path = config.duckdb_path();
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create duckdb directory at {:?}", parent))?;
            }
        }

        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open DuckDB at {:?}", path))?;

        let store = Self { conn };
        store.initialize_schema()?;

        Ok(store)
    }

    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS test_runs (
                run_id UUID PRIMARY KEY,
                plan_name VARCHAR,
                status VARCHAR,
                config JSON,
                started_at TIMESTAMP,
                completed_at TIMESTAMP,
                total_vus INTEGER,
                total_steps INTEGER
            );
            "#,
            [],
        )?;

        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS step_results (
                id UUID PRIMARY KEY,
                run_id UUID REFERENCES test_runs(run_id),
                vu_id INTEGER,
                step_name VARCHAR,
                protocol VARCHAR,
                endpoint VARCHAR,
                method VARCHAR,
                status VARCHAR,
                status_code INTEGER,
                request_body TEXT,
                response_body TEXT,
                latency_ms DOUBLE,
                started_at TIMESTAMP,
                completed_at TIMESTAMP,
                error_message TEXT,
                is_mocked BOOLEAN,
                retry_count INTEGER DEFAULT 0
            );
            "#,
            [],
        )?;

        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS metrics (
                run_id UUID,
                step_name VARCHAR,
                metric_name VARCHAR,
                metric_value DOUBLE,
                dimensions JSON,
                recorded_at TIMESTAMP
            );
            "#,
            [],
        )?;

        Ok(())
    }

    pub fn insert_run(&self, run_id: Uuid, plan_name: &str, status: &str) -> Result<()> {
        let now = Utc::now().naive_utc();
        self.conn.execute(
            "INSERT INTO test_runs (run_id, plan_name, status, started_at) VALUES (?, ?, ?, ?)",
            params![run_id, plan_name, status, now],
        )?;
        Ok(())
    }
    
    pub fn insert_metric(&self, run_id: Uuid, step_name: &str, name: &str, value: f64) -> Result<()> {
        let now = Utc::now().naive_utc();
        self.conn.execute(
            "INSERT INTO metrics (run_id, step_name, metric_name, metric_value, recorded_at) VALUES (?, ?, ?, ?, ?)",
            params![run_id, step_name, name, value, now],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_duckdb_store_lifecycle() {
        let temp = tempdir().unwrap();
        let mut config = StorageConfig::default();
        config.data_dir = temp.path().to_path_buf();

        let store = DuckDbStore::new(&config).expect("Failed to initialize DuckDbStore");

        let run_id = Uuid::new_v4();
        store.insert_run(run_id, "my_test_plan", "running").unwrap();
        
        store.insert_metric(run_id, "login_step", "auth_latency", 150.5).unwrap();

        // Verify insertion
        let mut stmt = store.conn.prepare("SELECT count(*) FROM test_runs WHERE run_id = ?").unwrap();
        let mut rows = stmt.query(params![run_id]).unwrap();
        let count: i64 = rows.next().unwrap().unwrap().get(0).unwrap();
        assert_eq!(count, 1);
        
        let mut stmt = store.conn.prepare("SELECT count(*) FROM metrics WHERE run_id = ?").unwrap();
        let mut rows = stmt.query(params![run_id]).unwrap();
        let count: i64 = rows.next().unwrap().unwrap().get(0).unwrap();
        assert_eq!(count, 1);
    }
}
