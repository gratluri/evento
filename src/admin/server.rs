use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use anyhow::Result;
use std::sync::Arc;
use std::time::{Instant, Duration};
use tokio::sync::RwLock;
use sqlx::postgres::PgPoolOptions;
use crate::engine::config::StorageConfig;

#[derive(Serialize, Clone)]
pub struct SystemStatus {
    pub evento_core: String,
    pub sled_store: String,
    pub postgres_store: String,
    pub mcp_server: String,
    pub uptime_seconds: u64,
}

use crate::engine::storage::sled_store::SledStore;
use std::collections::VecDeque;

#[derive(Serialize, Clone)]
pub struct SystemMetricSnapshot {
    pub timestamp: u64,
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub tasks_created: usize,
    pub tasks_completed: usize,
}

pub struct AppState {
    pub status: Arc<RwLock<SystemStatus>>,
    pub metrics_history: Arc<RwLock<VecDeque<SystemMetricSnapshot>>>,
    pub sled: Arc<SledStore>,
}

#[get("/api/status")]
async fn status(data: web::Data<AppState>) -> impl Responder {
    let current_status = data.status.read().await.clone();
    HttpResponse::Ok().json(current_status)
}

#[get("/")]
async fn index() -> impl Responder {
    let html = include_str!("ui/index.html");
    HttpResponse::Ok().content_type("text/html").body(html)
}

pub fn configure_admin_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
       .service(status);
       
    crate::admin::api::configure_api_routes(cfg);
}

pub async fn start_admin_server(port: u16, config: StorageConfig) -> Result<()> {
    println!("Starting Admin UI on http://0.0.0.0:{}", port);
    
    let shared_status = Arc::new(RwLock::new(SystemStatus {
        evento_core: "Active".to_string(),
        sled_store: "Loading".to_string(),
        postgres_store: "Loading".to_string(),
        mcp_server: "Green".to_string(), // mocked for now
        uptime_seconds: 0,
    }));
    
    let sled_store = Arc::new(SledStore::new(&config)?);
    
    // Spawn Simulator Server
    let sim_store = sled_store.clone();
    let sim_port = port + 1;
    tokio::spawn(async move {
        if let Err(e) = crate::simulator::server::run_simulator(sim_port, sim_store).await {
            tracing::error!("Simulator server failed: {}", e);
        }
    });
    
    let metrics_history = Arc::new(RwLock::new(VecDeque::with_capacity(720)));
    
    let app_state = web::Data::new(AppState {
        status: shared_status.clone(),
        metrics_history: metrics_history.clone(),
        sled: sled_store.clone(),
    });
    
    // Spawn Background Health Monitor
    let monitor_status = shared_status.clone();
    let monitor_config = config.clone();
    tokio::spawn(async move {
        let start_time = Instant::now();
        let mut interval = tokio::time::interval(Duration::from_millis(monitor_config.health_check_interval_ms));
        
        loop {
            interval.tick().await;
            
            let uptime = start_time.elapsed().as_secs();
            
            // Ping Sled (basic file accessibility check for hot path)
            let sled_status = if monitor_config.sled_dir().exists() {
                "Green".to_string()
            } else {
                "Offline".to_string()
            };
            
            // Ping Postgres
            let pg_status = match PgPoolOptions::new().max_connections(1).connect(&monitor_config.postgres_url).await {
                Ok(pool) => {
                    // Try to execute a simple query
                    match sqlx::query("SELECT 1").execute(&pool).await {
                        Ok(_) => "Green".to_string(),
                        Err(_) => "Error".to_string(),
                    }
                }
                Err(_) => "Offline".to_string(),
            };
            
            // Update shared state
            let mut write_lock = monitor_status.write().await;
            write_lock.uptime_seconds = uptime;
            write_lock.sled_store = sled_status;
            write_lock.postgres_store = pg_status;
        }
    });

    let monitor_metrics = metrics_history.clone();
    let monitor_store = sled_store.clone();
    tokio::spawn(async move {
        let mut sys = sysinfo::System::new_all();
        let mut networks = sysinfo::Networks::new_with_refreshed_list();
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            networks.refresh();
            
            let cpu_usage = sys.global_cpu_info().cpu_usage();
            let memory_used = sys.used_memory();
            let memory_total = sys.total_memory();
            
            let mut network_rx = 0;
            let mut network_tx = 0;
            for (_interface_name, data) in &networks {
                network_rx += data.received();
                network_tx += data.transmitted();
            }
            
            // Calculate tasks created and completed
            let mut tasks_created = 0;
            let mut tasks_completed = 0;
            if let Ok(runs) = monitor_store.list_runs() {
                tasks_created = runs.len();
                for run_id in runs {
                    if let Ok(Some(state)) = monitor_store.get_run_state(&run_id) {
                        if state.contains("Completed") || state.contains("Failed") {
                            tasks_completed += 1;
                        }
                    }
                }
            }

            let mut metrics_lock = monitor_metrics.write().await;
            if metrics_lock.len() >= 720 {
                metrics_lock.pop_front();
            }
            metrics_lock.push_back(SystemMetricSnapshot {
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                cpu_usage,
                memory_used,
                memory_total,
                network_rx,
                network_tx,
                tasks_created,
                tasks_completed,
            });
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .configure(configure_admin_routes)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;
    
    Ok(())
}
