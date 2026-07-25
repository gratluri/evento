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

pub struct AppState {
    pub status: Arc<RwLock<SystemStatus>>,
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
    
    let app_state = web::Data::new(AppState {
        status: shared_status.clone(),
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
