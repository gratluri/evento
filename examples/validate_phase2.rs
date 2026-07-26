use anyhow::Result;
use evento::dsl::dsl_parser::TestPlan;
use evento::engine::compiler::Compiler;
use evento::engine::manager::RunManager;
use evento::engine::config::StorageConfig;
use evento::engine::storage::sled_store::SledStore;
use std::sync::Arc;
use tempfile::tempdir;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // 1. Setup temporary Sled store
    let temp = tempdir()?;
    let mut config = StorageConfig::default();
    config.data_dir = temp.path().to_path_buf();
    let store = Arc::new(SledStore::new(&config)?);

    // 2. Load the checkout mock plan (or a simple plan in memory)
    let yaml = r#"
test: validate_phase2
config:
  virtual_users: 2
  duration: 5s
scenario:
  - name: login
    protocol: http
    mock:
      response:
        status: 200
        latency: 100ms
  - name: add_to_cart
    protocol: http
    mock:
      response:
        status: 200
        latency: 150ms
"#;
    let test_plan = TestPlan::from_yaml_str(yaml)?;

    // 3. Compile
    let exec_plan = Compiler::compile(&test_plan)?;
    println!("Compiled Plan: run_id={}", exec_plan.run_id);
    
    // Store plan
    store.store_run(&exec_plan.run_id, &serde_json::to_string(&exec_plan)?)?;

    // 4. Execute
    let manager = RunManager::new(exec_plan.clone(), store.clone());
    println!("Starting RunManager...");
    manager.execute().await?;
    println!("RunManager completed.");

    // 5. Verify results
    let run_state = store.get_run_state(&exec_plan.run_id)?.unwrap();
    println!("Final Run State: {}", run_state);

    // Let's print out the step results for VU 0
    println!("Results for VU 0:");
    for task in &exec_plan.tasks {
        let key = format!("run:{}:vu:{}:result:{}", exec_plan.run_id, 0, task.task_id);
        if let Some(res_json) = store.get_vu_context(&exec_plan.run_id, 0, &key)? {
            println!("  Task {}: {}", task.step_name, res_json);
        }
    }

    Ok(())
}
