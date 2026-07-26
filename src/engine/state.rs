use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// The overall execution plan for a given TestPlan
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionPlan {
    pub run_id: String,
    pub namespace: String,
    pub config: ExecutionConfig,
    pub tasks: Vec<ExecutionTask>,
    pub metadata: PlanMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionConfig {
    pub virtual_users: u32,
    pub duration_ms: u64,
    pub mock_strategy: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanMetadata {
    pub submitted_at: DateTime<Utc>,
    pub description: Option<String>,
}

/// A single execution unit within the plan (a flattened DAG node)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionTask {
    pub task_id: String,
    pub step_name: String,
    pub dependencies: Vec<String>,
    // The raw DSL step to execute
    pub step_definition: crate::dsl::dsl_parser::Step,
}

/// State of an entire test run
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum RunState {
    Submitted,
    Compiling,
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// State of an individual Virtual User instance
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum VuState {
    Spawned,
    Executing(String), // task_id
    WaitingOnDependency,
    Completed,
    Failed { task_id: String, error: String },
}

/// The result of executing a single step
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StepResult {
    pub run_id: String,
    pub vu_id: u32,
    pub task_id: String,
    pub retry_attempt: u32,
    pub status: StepStatus,
    pub latency_ms: u64,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Success,
    Failed(String),
}
