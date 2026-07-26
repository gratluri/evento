use crate::engine::state::{ExecutionPlan, StepResult, StepStatus, ExecutionTask};
use crate::engine::context::VuContext;
use crate::engine::protocol::{HttpExecutor, PostgresExecutor, KafkaExecutor, CassandraExecutor, ProtocolExecutor};
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
        let mut context = VuContext::new(self.plan.run_id.clone(), self.vu_id);

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
            let status = self.execute_task(task, &mut context).await?;

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

    async fn execute_task(&self, task: &ExecutionTask, ctx: &mut VuContext) -> Result<StepStatus> {
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
                        self.execute_do_block(do_block, &task.task_id, iter, ctx).await?;
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
            let context_before = ctx.variables.clone();
            let mut status = StepStatus::Success;

            let mut mock_failed = false;
            let mock_strategy = self.plan.config.mock_strategy.as_str();

            let mut executed_protocol = false;
            
            // Phase 4: Real Protocol Execution
            if (mock_strategy == "auto" || mock_strategy == "disabled") && step_def.protocol.is_some() {
                let protocol = step_def.protocol.as_ref().unwrap();
                executed_protocol = true;
                
                let execution_result = match protocol.as_str() {
                    "http" => HttpExecutor.execute(step_def, ctx).await,
                    "postgres" => PostgresExecutor.execute(step_def, ctx).await,
                    "cassandra" => CassandraExecutor.execute(step_def, ctx).await,
                    "kafka" | "eventhub" => KafkaExecutor.execute(step_def, ctx).await,
                    _ => {
                        executed_protocol = false;
                        Err(anyhow::anyhow!("Unsupported protocol: {}", protocol))
                    }
                };

                if executed_protocol {
                    match execution_result {
                        Ok(response) => {
                            if response.status_code >= 400 {
                                status = StepStatus::Failed(format!("Protocol Error {}", response.status_code));
                            } else {
                                status = StepStatus::Success;
                                // Handle Extraction
                                if let Some(extract_map) = &step_def.extract {
                                    for (var_name, extract_path) in extract_map {
                                        if let Some(extracted_val) = response.extract(extract_path) {
                                            ctx.set(var_name.clone(), extracted_val);
                                        }
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            status = StepStatus::Failed(e.to_string());
                        }
                    }
                } else {
                    status = StepStatus::Failed(format!("Unsupported protocol: {}", protocol));
                }
            }

            // Phase 3.2: Mock & Chaos
            // Execute mock if required, OR if auto strategy and protocol execution failed/was skipped
            let should_run_mock = mock_strategy == "required" || 
                                  (mock_strategy == "auto" && (!executed_protocol || matches!(status, StepStatus::Failed(_))));

            if should_run_mock {
                if let Some(mock) = &step_def.mock {
                    // Reset status since we are falling back to mock
                    status = StepStatus::Success;
                    
                    if let Some(behavior) = &mock.behavior {
                        // Probabilistic Error Injection
                        if let Some(error_rate) = behavior.error_rate {
                            if rand::random::<f64>() < error_rate {
                                status = StepStatus::Failed("Chaos Error Injected".to_string());
                                mock_failed = true;
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
                    if !mock_failed {
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
                } else if !executed_protocol {
                    status = StepStatus::Failed("No mock or protocol configuration found".to_string());
                }
            }

            let latency_ms = (Utc::now() - start).num_milliseconds() as u64;
            let context_after = ctx.variables.clone();

            let result = StepResult {
                run_id: self.plan.run_id.clone(),
                vu_id: self.vu_id,
                task_id: task.task_id.clone(),
                retry_attempt: attempt,
                status: status.clone(),
                latency_ms,
                executed_at: Utc::now(),
                context_before,
                context_after,
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

    async fn execute_do_block(&self, do_block: &DoBlock, parent_task_id: &str, iter: u64, ctx: &mut VuContext) -> Result<()> {
        // Evaluate the block dynamically.
        match do_block {
            DoBlock::Single(step) => {
                let task = ExecutionTask {
                    task_id: format!("{}_iter_{}_inner", parent_task_id, iter),
                    step_name: step.name.clone(),
                    dependencies: vec![],
                    step_definition: *step.clone(),
                };
                Box::pin(self.execute_task(&task, ctx)).await?;
            },
            DoBlock::Multiple(steps) => {
                for (i, step) in steps.iter().enumerate() {
                    let task = ExecutionTask {
                        task_id: format!("{}_iter_{}_inner_{}", parent_task_id, iter, i),
                        step_name: step.name.clone(),
                        dependencies: vec![],
                        step_definition: step.clone(),
                    };
                    Box::pin(self.execute_task(&task, ctx)).await?;
                }
            },
            DoBlock::Inline(_inline) => {
                sleep(Duration::from_millis(10)).await;
            }
        }
        Ok(())
    }
}
