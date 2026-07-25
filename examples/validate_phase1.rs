use evento::engine::config::StorageConfig;
use evento::engine::storage::sled_store::SledStore;
use evento::admin::server::start_admin_server;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Phase 1 & 1.5 Validation ===");
    
    // 1. Initialize configuration (using a local test directory)
    let mut config = StorageConfig::default();
    let mut test_dir = std::env::current_dir()?;
    test_dir.push(".evento_test_data");
    config.data_dir = test_dir;
    
    println!("\n[1] Configuration Paths:");
    println!("  Data Dir:   {:?}", config.data_dir);
    println!("  Sled:       {:?}", config.sled_dir());

    // 2. Initialize Sled (Operational Hot Path)
    println!("\n[2] Initializing SledStore...");
    let sled = SledStore::new(&config)?;
    
    let run_id = "run-12345";
    let state_json = r#"{"status": "running", "active_vus": 10}"#;
    
    sled.set_run_state(run_id, state_json)?;
    println!("  Set Run State   -> OK");
    
    sled.set_vu_context(run_id, 42, "auth_token", "super_secret_jwt")?;
    println!("  Set VU Context  -> OK");

    let retrieved_state = sled.get_run_state(run_id)?.unwrap_or_else(|| "None".to_string());
    println!("  Got Run State   -> {}", retrieved_state);
    
    let retrieved_ctx = sled.get_vu_context(run_id, 42, "auth_token")?.unwrap_or_else(|| "None".to_string());
    println!("  Got VU Context  -> {}", retrieved_ctx);

    println!("\n=== Validation Complete ===");
    println!("Starting Admin Dashboard on http://0.0.0.0:8080 ...");
    
    // Spawn Admin Web Server on port 8080
    start_admin_server(8080, config).await?;

    Ok(())
}
