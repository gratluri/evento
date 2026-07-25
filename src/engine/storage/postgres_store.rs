use super::AnalyticsStore;
use crate::engine::config::StorageConfig;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::Utc;

pub struct PostgresStore {
    pool: Pool<Postgres>,
}

impl PostgresStore {
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.postgres_url)
            .await
            .context("Failed to connect to Postgres")?;
            
        let store = Self { pool };
        store.init_schema().await?;
        
        Ok(store)
    }

    async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runs (
                run_id UUID PRIMARY KEY,
                name VARCHAR NOT NULL,
                status VARCHAR NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS metrics (
                id SERIAL PRIMARY KEY,
                run_id UUID NOT NULL REFERENCES runs(run_id),
                step_name VARCHAR NOT NULL,
                metric_type VARCHAR NOT NULL,
                value DOUBLE PRECISION NOT NULL,
                recorded_at TIMESTAMPTZ NOT NULL
            );
            "#
        )
        .execute(&self.pool)
        .await
        .context("Failed to initialize Postgres schema")?;
        
        Ok(())
    }
}

#[async_trait]
impl AnalyticsStore for PostgresStore {
    async fn insert_run(&self, run_id: Uuid, name: &str, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO runs (run_id, name, status, created_at)
            VALUES ($1, $2, $3, $4)
            "#
        )
        .bind(run_id)
        .bind(name)
        .bind(status)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .context("Failed to insert run")?;
        
        Ok(())
    }

    async fn insert_metric(&self, run_id: Uuid, step_name: &str, metric_type: &str, value: f64) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO metrics (run_id, step_name, metric_type, value, recorded_at)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(run_id)
        .bind(step_name)
        .bind(metric_type)
        .bind(value)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .context("Failed to insert metric")?;
        
        Ok(())
    }
}
