use crate::engine::state::{ExecutionPlan, StepResult, StepStatus, ExecutionTask};
use crate::engine::context::VuContext;
use crate::engine::storage::sled_store::SledStore;
use crate::dsl::dsl_parser::DoBlock;
use anyhow::Result;
use std::sync::Arc;
use chrono::Utc;
use tokio::time::{sleep, Duration};


pub struct VuWorker {
    pub vu_id: u32,
    pub plan: Arc<ExecutionPlan>,
    pub store: Arc<SledStore>,
}

impl VuWorker {
    pub fn new(vu_id: u32, plan: Arc<ExecutionPlan>, store: Arc<SledStore>) -> Self {
        Self { vu_id, plan, store }
    }

    pub async fn run(&mut self) -> Result<()> {
        let _context = VuContext::new(self.plan.run_id.clone(), self.vu_id);

        let mut current_idx = 0;
        let num_tasks = self.plan.tasks.len();

        while current_idx < num_tasks {
            // Check if cancelled
            if let Ok(Some(state_json)) = self.store.get_run_state(&self.plan.run_id) {
                if state_json.contains("Cancelled") {
                    break;
                }
            }

            let task = &self.plan.tasks[current_idx];
            
            // Execute the step (could be a simple step, or a loop)
            let status = self.execute_task(task).await?;

            if let StepStatus::Failed(_) = status {
                // Check on_failure
                if let Some(on_failure) = &task.step_definition.on_failure {
                    if let Some(idx) = self.plan.tasks.iter().position(|t| t.step_name == *on_failure) {
                        current_idx = idx;
                        continue;
                    }
                }
                break; // Break if no fallback
            } else {
                // Check on_success
                if let Some(on_success) = &task.step_definition.on_success {
                    if let Some(idx) = self.plan.tasks.iter().position(|t| t.step_name == *on_success) {
                        current_idx = idx;
                        continue;
                    }
                }
            }

            current_idx += 1;
        }

        Ok(())
    }

    async fn execute_task(&self, task: &ExecutionTask) -> Result<StepStatus> {
        let step_def = &task.step_definition;
        
        // Handle Loop Execution
        if let Some(loop_config) = &step_def.loop_config {
            let count = match &loop_config.count {
                Some(serde_yaml::Value::Number(n)) => n.as_u64().unwrap_or(0),
                _ => 0, // Fallback
            };

            if count > 0 {
                for iter in 0..count {
                    if let Some(do_block) = &step_def.do_steps {
                        self.execute_do_block(do_block, &task.task_id, iter).await?;
                    }
                }
            }
            return Ok(StepStatus::Success);
        }

        // Handle Retry Logic
        let max_attempts = step_def.retry.as_ref().map(|r| r.max_attempts).unwrap_or(1);
        let mut attempt = 1;
        let mut final_status = StepStatus::Success;

        while attempt <= max_attempts {
            let start = Utc::now();
            let mut status = StepStatus::Success;

            // Phase 3.2: Mock & Chaos
            if let Some(mock) = &step_def.mock {
                if let Some(behavior) = &mock.behavior {
                    // Probabilistic Error Injection
                    if let Some(error_rate) = behavior.error_rate {
                        if rand::random::<f64>() < error_rate {
                            status = StepStatus::Failed("Chaos Error Injected".to_string());
                            if let Some(err_resp) = &behavior.error_response {
                                if let Some(code) = err_resp.status {
                                    status = StepStatus::Failed(format!("HTTP {}", code));
                                }
                            }
                        }
                    }

                    // Simulated Latency
                    if let Some(latency) = &behavior.latency {
                        if let crate::dsl::dsl_parser::MockLatency::Fixed(fixed_str) = latency {
                            if fixed_str.ends_with("ms") {
                                if let Ok(ms) = fixed_str.trim_end_matches("ms").parse::<u64>() {
                                    sleep(Duration::from_millis(ms)).await;
                                }
                            }
                        }
                    }
                }
                
                // Normal Response (if no chaos error triggered)
                if let StepStatus::Success = status {
                    if let Some(response) = &mock.response {
                        if let Some(latency_str) = &response.latency {
                            if latency_str.ends_with("ms") {
                                if let Ok(ms) = latency_str.trim_end_matches("ms").parse::<u64>() {
                                    sleep(Duration::from_millis(ms)).await;
                                }
                            }
                        }
                        if let Some(status_code) = response.status {
                            if status_code >= 400 {
                                status = StepStatus::Failed(format!("HTTP {}", status_code));
                            }
                        }
                    }
                }
            } else if step_def.protocol.is_some() {
                status = StepStatus::Failed("No mock configuration found".to_string());
            }

            let latency_ms = (Utc::now() - start).num_milliseconds() as u64;

            let result = StepResult {
                run_id: self.plan.run_id.clone(),
                vu_id: self.vu_id,
                task_id: task.task_id.clone(),
                retry_attempt: attempt,
                status: status.clone(),
                latency_ms,
                executed_at: Utc::now(),
            };

            // Write telemetry for auditability
            let result_json = serde_json::to_string(&result)?;
            let key = format!("run:{}:vu:{}:task:{}:attempt:{}", self.plan.run_id, self.vu_id, task.task_id, attempt);
            self.store.set_vu_context(&self.plan.run_id, self.vu_id, &key, &result_json)?;

            final_status = status.clone();

            if let StepStatus::Success = status {
                break; // Succeeded, exit retry loop
            } else if attempt < max_attempts {
                // Sleep for retry delay
                if let Some(retry) = &step_def.retry {
                    if let Some(delay_str) = &retry.delay {
                        if delay_str.ends_with("ms") {
                            if let Ok(ms) = delay_str.trim_end_matches("ms").parse::<u64>() {
                                sleep(Duration::from_millis(ms)).await;
                            }
                        } else if delay_str.ends_with("s") {
                            if let Ok(secs) = delay_str.trim_end_matches("s").parse::<u64>() {
                                sleep(Duration::from_secs(secs)).await;
                            }
                        }
                    }
                }
            }
            attempt += 1;
        }

        Ok(final_status)
    }

    async fn execute_do_block(&self, do_block: &DoBlock, parent_task_id: &str, iter: u64) -> Result<()> {
        // Evaluate the block dynamically.
        match do_block {
            DoBlock::Single(step) => {
                let task = ExecutionTask {
                    task_id: format!("{}_iter_{}_inner", parent_task_id, iter),
                    step_name: step.name.clone(),
                    dependencies: vec![],
                    step_definition: *step.clone(),
                };
                Box::pin(self.execute_task(&task)).await?;
            },
            DoBlock::Multiple(steps) => {
                for (i, step) in steps.iter().enumerate() {
                    let task = ExecutionTask {
                        task_id: format!("{}_iter_{}_inner_{}", parent_task_id, iter, i),
                        step_name: step.name.clone(),
                        dependencies: vec![],
                        step_definition: step.clone(),
                    };
                    Box::pin(self.execute_task(&task)).await?;
                }
            },
            DoBlock::Inline(_inline) => {
                sleep(Duration::from_millis(10)).await;
            }
        }
        Ok(())
    }
}
