pub mod sled_store;
pub mod postgres_store;

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait AnalyticsStore: Send + Sync {
    async fn insert_run(&self, run_id: Uuid, name: &str, status: &str) -> Result<()>;
    async fn insert_metric(&self, run_id: Uuid, step_name: &str, metric_type: &str, value: f64) -> Result<()>;
}
