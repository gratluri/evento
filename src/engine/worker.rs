use crate::engine::state::{ExecutionPlan, StepResult, StepStatus, ExecutionTask};
use crate::engine::context::VuContext;
use crate::engine::protocol::{HttpExecutor, PostgresExecutor, KafkaExecutor, CassandraExecutor, ProtocolExecutor};
use crate::engine::storage::sled_store::SledStore;
use crate::dsl::dsl_parser::DoBlock;
use anyhow::Result;
use std::sync::Arc;
use chrono::Utc;
use tokio::time::{sleep, Duration};

pub fn parse_duration(s: &str) -> Option<Duration> {
    if let Some(ms_str) = s.strip_suffix("ms") {
        ms_str.parse::<u64>().ok().map(Duration::from_millis)
    } else if let Some(s_str) = s.strip_suffix("s") {
        s_str.parse::<u64>().ok().map(Duration::from_secs)
    } else if let Some(m_str) = s.strip_suffix("m") {
        m_str.parse::<u64>().ok().map(|m| Duration::from_secs(m * 60))
    } else {
        None
    }
}

pub struct VuWorker {
    pub vu_id: u32,
    pub plan: Arc<ExecutionPlan>,
    pub store: Arc<SledStore>,
    pub async_tasks: std::collections::HashMap<String, tokio::task::JoinHandle<()>>,
}

impl VuWorker {
    pub fn new(vu_id: u32, plan: Arc<ExecutionPlan>, store: Arc<SledStore>) -> Self {
        Self { vu_id, plan, store, async_tasks: std::collections::HashMap::new() }
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
            
            // Handle wait_for barrier
            if let Some(wait_list) = &task.step_definition.wait_for {
                for wait_step in wait_list {
                    if let Some(handle) = self.async_tasks.remove(wait_step) {
                        let _ = handle.await;
                    }
                }
            }

            // Handle async execution (fire-and-forget)
            if task.step_definition.r#async.unwrap_or(false) {
                let task_clone = task.clone();
                let mut ctx_clone = context.clone();
                // Create a clone of VuWorker that doesn't own any async tasks to prevent nested issues
                let worker_clone = VuWorker::new(self.vu_id, self.plan.clone(), self.store.clone());
                
                let handle = tokio::task::spawn(async move {
                    let _ = worker_clone.execute_task(&task_clone, &mut ctx_clone).await;
                });
                
                self.async_tasks.insert(task.step_name.clone(), handle);
                current_idx += 1;
                continue;
            }

            // Execute the step sequentially
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
            // Check for over/from collection iteration
            if let (Some(over_var), Some(from_expr)) = (&loop_config.over, &loop_config.from) {
                let resolved_from = ctx.interpolate(from_expr);
                if let Ok(serde_json::Value::Array(items)) = serde_json::from_str(&resolved_from) {
                    for (iter, item) in items.iter().enumerate() {
                        // Inject item into context
                        let item_str = match item {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        ctx.set(over_var.clone(), item_str);
                        
                        if let Some(do_block) = &step_def.do_steps {
                            self.execute_do_block(do_block, &task.task_id, iter as u64, ctx).await?;
                        }
                    }
                }
            } else {
                // Check for numeric count iteration
                let count = match &loop_config.count {
                    Some(serde_yaml::Value::Number(n)) => n.as_u64().unwrap_or(0),
                    Some(serde_yaml::Value::String(s)) => {
                        let resolved = ctx.interpolate(s);
                        resolved.parse::<u64>().unwrap_or(0)
                    },
                    _ => 0,
                };

                if count > 0 {
                    for iter in 0..count {
                        if let Some(do_block) = &step_def.do_steps {
                            self.execute_do_block(do_block, &task.task_id, iter, ctx).await?;
                        }
                    }
                }
            }
            return Ok(StepStatus::Success);
        }

        // Handle Retry and Within Logic
        let max_attempts = step_def.retry.as_ref().map(|r| r.max_attempts).unwrap_or(1);
        let within_duration = step_def.within.as_ref().and_then(|w| {
            match w {
                crate::dsl::dsl_parser::WithinConfig::Duration(d) => Some(d),
                crate::dsl::dsl_parser::WithinConfig::Structured(s) => Some(&s.duration),
            }
        }).and_then(|d| parse_duration(d));

        let step_timeout = step_def.timeout.as_deref().and_then(parse_duration).unwrap_or(Duration::from_secs(10));

        let mut attempt = 1;
        let mut final_status = StepStatus::Success;
        let loop_start_time = std::time::Instant::now();

        loop {
            if attempt > max_attempts && within_duration.is_none() {
                break;
            }
            if let Some(limit) = within_duration {
                if loop_start_time.elapsed() >= limit {
                    final_status = StepStatus::Failed("Within duration exceeded".to_string());
                    break;
                }
            }
            let start = Utc::now();
            let context_before = ctx.variables.clone();
            let mut status = StepStatus::Success;

            let mut mock_failed = false;
            let mock_strategy = self.plan.config.mock_strategy.as_str();

            let mut executed_protocol = false;
            let has_protocol = step_def.protocol.is_some();
            let has_script = step_def.script.is_some();
            let is_script_protocol = step_def.protocol.as_deref() == Some("script");
            let is_script_only = (!has_protocol && has_script) || is_script_protocol;
            let has_protocol_or_script = has_protocol || has_script;
            
            // Phase 4: Real Protocol Execution
            if (mock_strategy == "auto" || mock_strategy == "disabled" || is_script_only) && has_protocol_or_script {
                let protocol_str = step_def.protocol.as_deref().unwrap_or("script");
                let is_supported = matches!(protocol_str, "http" | "postgres" | "cassandra" | "kafka" | "eventhub" | "script");
                executed_protocol = is_supported;

                if executed_protocol {
                    let execution_future = async {
                        match protocol_str {
                            "http" => crate::engine::protocol::HttpExecutor.execute(step_def, ctx).await,
                            "postgres" => crate::engine::protocol::PostgresExecutor.execute(step_def, ctx).await,
                            "cassandra" => crate::engine::protocol::CassandraExecutor.execute(step_def, ctx).await,
                            "kafka" | "eventhub" => crate::engine::protocol::KafkaExecutor.execute(step_def, ctx).await,
                            "script" => crate::engine::protocol::ScriptExecutor.execute(step_def, ctx).await,
                            _ => unreachable!(),
                        }
                    };

                    // Apply step timeout wrapper
                    let timeout_result = tokio::time::timeout(step_timeout, execution_future).await;
                    
                    match timeout_result {
                        Ok(Ok(response)) => {
                            if response.status_code >= 400 {
                                status = StepStatus::Failed(format!("Protocol Error {}", response.status_code));
                            } else {
                                status = StepStatus::Success;
                                
                                // Extract variables so they can be validated
                                if let Some(extract_map) = &step_def.extract {
                                    for (var_name, extract_path) in extract_map {
                                        if let Some(extracted_val) = response.extract(extract_path) {
                                            ctx.set(var_name.clone(), extracted_val);
                                        }
                                    }
                                }

                                // Evaluate validate block
                                if let Some(validations) = &step_def.validate {
                                    for rule in validations {
                                        if !ctx.evaluate_boolean_rule(rule, &response) {
                                            status = StepStatus::Failed(format!("Validation failed: {}", rule));
                                            break;
                                        }
                                    }
                                }
                            }
                        },
                        Ok(Err(e)) => {
                            status = StepStatus::Failed(e.to_string());
                        },
                        Err(_) => {
                            status = StepStatus::Failed(format!("Step timeout of {:?} exceeded", step_timeout));
                        }
                    }
                } else {
                    status = StepStatus::Failed(format!("Unsupported protocol: {}", protocol_str));
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
                    if step_def.protocol.is_some() {
                        status = StepStatus::Failed(format!("Mock strategy is '{}', but no mock configuration found for protocol", mock_strategy));
                    } else {
                        status = StepStatus::Failed("No mock or protocol configuration found".to_string());
                    }
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
            } else {
                // Sleep for retry delay or within poll interval
                let delay = if within_duration.is_some() {
                    Duration::from_millis(500) // Poll interval for within
                } else if let Some(retry) = &step_def.retry {
                    retry.delay.as_deref().and_then(parse_duration).unwrap_or(Duration::from_millis(1000))
                } else {
                    Duration::from_millis(0)
                };
                
                if delay.as_millis() > 0 {
                    sleep(delay).await;
                }
                
                if within_duration.is_none() && attempt >= max_attempts {
                    break;
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
