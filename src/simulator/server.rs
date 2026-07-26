use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use crate::engine::storage::sled_store::SledStore;
use std::sync::Arc;
use tokio::time::sleep;
use std::time::Duration;
use tracing::{info, warn, error};
use crate::engine::state::ExecutionPlan;

pub async fn run_simulator(port: u16, store: Arc<SledStore>) -> std::io::Result<()> {
    info!("Starting Target Service Simulator on port {}", port);
    
    let app_state = web::Data::new(store);
    
    let server = HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .default_service(web::route().to(handle_mock_request))
    })
    .bind(("0.0.0.0", port))?
    .run();
    
    server.await
}

async fn handle_mock_request(req: HttpRequest, store: web::Data<Arc<SledStore>>, _body: web::Bytes) -> impl Responder {
    let path = req.path().to_string();
    let method = req.method().as_str().to_string();
    let run_id_header = req.headers().get("X-Evento-Run-Id").map(|h| h.to_str().unwrap_or("").to_string());
    
    info!("Simulator received {} {}", method, path);

    let run_ids = match store.list_runs() {
        Ok(ids) => ids,
        Err(_) => return HttpResponse::InternalServerError().body("Store error"),
    };

    let mut matching_steps = Vec::new();

    for id in run_ids {
        if let Some(target_id) = &run_id_header {
            if &id != target_id {
                continue;
            }
        }
        
        if let Ok(Some(plan_str)) = store.get_run(&id) {
            if let Ok(plan) = serde_json::from_str::<ExecutionPlan>(&plan_str) {
                for task in &plan.tasks {
                    if let Some(ep) = &task.step_definition.endpoint {
                        if let Some(m) = &task.step_definition.method {
                            if ep == &path && m.eq_ignore_ascii_case(&method) {
                                matching_steps.push((id.clone(), task.step_definition.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    if matching_steps.is_empty() {
        return HttpResponse::NotFound().body(format!("No mock found for {} {}", method, path));
    }

    if matching_steps.len() > 1 && run_id_header.is_none() {
        return HttpResponse::Conflict().body("Multiple active test plans match this route. Please provide X-Evento-Run-Id header.");
    }

    let (_, step) = &matching_steps[0];

    // Apply Mock behaviors
    let mut status_code = 200;
    let mut latency_str = String::new();

    if let Some(mock) = &step.mock {
        if let Some(resp) = &mock.response {
            if let Some(status) = resp.status {
                status_code = status;
            }
            if let Some(latency) = &resp.latency {
                latency_str = latency.clone();
            }
        }
    }

    // Simulate latency
    if !latency_str.is_empty() {
        let mut ms: u64 = 0;
        if latency_str.ends_with("ms") {
            if let Ok(val) = latency_str.trim_end_matches("ms").parse::<u64>() {
                ms = val;
            }
        } else if latency_str.ends_with("s") {
            if let Ok(val) = latency_str.trim_end_matches('s').parse::<u64>() {
                ms = val * 1000;
            }
        }
        
        if ms > 0 {
            info!("Simulator injecting latency: {}ms", ms);
            sleep(Duration::from_millis(ms)).await;
        }
    }

    let mut response_builder = HttpResponse::build(actix_web::http::StatusCode::from_u16(status_code).unwrap_or(actix_web::http::StatusCode::OK));
    response_builder.insert_header(("X-Evento-Mock", "true"));

    if let Some(mock) = &step.mock {
        if let Some(resp) = &mock.response {
            if let Some(body_val) = &resp.body {
                if let Ok(json_str) = serde_json::to_string(body_val) {
                    return response_builder.content_type("application/json").body(json_str);
                }
            }
        }
    }

    response_builder.body("Mocked successfully by Evento Simulator")
}
