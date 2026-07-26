use crate::dsl::dsl_parser::TestPlan;
use crate::engine::state::{ExecutionPlan, ExecutionConfig, ExecutionTask, PlanMetadata};
use anyhow::{Result, bail};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

pub struct Compiler;

impl Compiler {
    /// Compiles a TestPlan AST into a flat, topologically-sorted ExecutionPlan DAG.
    pub fn compile(plan: &TestPlan) -> Result<ExecutionPlan> {
        let run_id = Uuid::new_v4().to_string();
        
        // Auto-derive namespace: Replace '.' with '/'
        let namespace = plan.test.replace(".", "/");
        
        let config = ExecutionConfig {
            virtual_users: plan.config.as_ref().and_then(|c| c.virtual_users).unwrap_or(1),
            // Default 60s for now, in a real implementation we would parse the duration string
            duration_ms: 60000, 
            mock_strategy: plan.config.as_ref()
                .and_then(|c| c.mock_strategy.clone())
                .unwrap_or_else(|| "required".to_string()),
        };

        let mut tasks = Vec::new();
        
        // Map from step_name -> task_id to resolve dependencies
        let mut step_to_task_id = HashMap::new();
        let mut previous_task_id: Option<String> = None;

        for step in &plan.scenario {
            let task_id = Uuid::new_v4().to_string();
            step_to_task_id.insert(step.name.clone(), task_id.clone());

            let mut dependencies = Vec::new();

            // Resolve explicit dependencies (wait_for) or fall back to implicit sequential
            if let Some(wait_for) = &step.wait_for {
                for dep_name in wait_for {
                    if let Some(dep_task_id) = step_to_task_id.get(dep_name) {
                        dependencies.push(dep_task_id.clone());
                    } else {
                        bail!("Step '{}' waits for unknown step '{}'", step.name, dep_name);
                    }
                }
            } else if let Some(prev) = &previous_task_id {
                // If not explicitly parallel or waiting, default to sequential after the last step
                dependencies.push(prev.clone());
            }
            
            tasks.push(ExecutionTask {
                task_id: task_id.clone(),
                step_name: step.name.clone(),
                dependencies,
                step_definition: step.clone(),
            });

            previous_task_id = Some(task_id);
        }

        Ok(ExecutionPlan {
            run_id,
            namespace,
            config,
            tasks,
            metadata: PlanMetadata {
                submitted_at: Utc::now(),
                description: plan.description.clone(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::dsl_parser::{Step, MockConfig};
    
    #[test]
    fn test_compiler_linear_flow() {
        let plan = TestPlan {
            test: "sales.buyflow.basic".to_string(),
            description: None,
            imports: None,
            config: None,
            data_sources: None,
            functions: None,
            scenario: vec![
                Step {
                    name: "step1".to_string(),
                    description: None,
                    protocol: Some("http".to_string()),
                    endpoint: None,
                    method: None,
                    headers: None,
                    body: None,
                    topic: None,
                    message: None,
                    mode: None,
                    expect: None,
                    within: None,
                    service: None,
                    connection: None,
                    query: None,
                    r#async: None,
                    validate: None,
                    on_success: None,
                    on_failure: None,
                    parallel: None,
                    loop_config: None,
                    do_steps: None,
                    wait_for: None,
                    timeout: None,
                    on_timeout: None,
                    retry: None,
                    extract: None,
                    track_metric: None,
                    script: None,
                    runtime: None,
                    transform: None,
                    use_module: None,
                    with: None,
                    outputs_to: None,
                    mock: Some(MockConfig {
                        response: None,
                        responses: None,
                        on_exhausted: None,
                        behavior: None,
                        request_schema: None,
                        response_schema: None,
                    }),
                },
                Step {
                    name: "step2".to_string(),
                    description: None,
                    protocol: Some("http".to_string()),
                    endpoint: None,
                    method: None,
                    headers: None,
                    body: None,
                    topic: None,
                    message: None,
                    mode: None,
                    expect: None,
                    within: None,
                    service: None,
                    connection: None,
                    query: None,
                    r#async: None,
                    validate: None,
                    on_success: None,
                    on_failure: None,
                    parallel: None,
                    loop_config: None,
                    do_steps: None,
                    wait_for: None,
                    timeout: None,
                    on_timeout: None,
                    retry: None,
                    extract: None,
                    track_metric: None,
                    script: None,
                    runtime: None,
                    transform: None,
                    use_module: None,
                    with: None,
                    outputs_to: None,
                    mock: Some(MockConfig {
                        response: None,
                        responses: None,
                        on_exhausted: None,
                        behavior: None,
                        request_schema: None,
                        response_schema: None,
                    }),
                },
            ],
            validation: None,
            outputs: None,
            metadata: None,
            base: None,
            extends: None,
        };

        let exec_plan = Compiler::compile(&plan).unwrap();
        
        assert_eq!(exec_plan.namespace, "sales/buyflow/basic");
        assert_eq!(exec_plan.tasks.len(), 2);
        
        // step2 should depend on step1
        assert_eq!(exec_plan.tasks[1].dependencies.len(), 1);
        assert_eq!(exec_plan.tasks[1].dependencies[0], exec_plan.tasks[0].task_id);
    }
}
