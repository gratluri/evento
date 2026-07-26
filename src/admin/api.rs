use actix_web::{get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use crate::admin::server::AppState;
use crate::dsl::dsl_parser::TestPlan;
use crate::engine::compiler::Compiler;
use crate::engine::manager::RunManager;
use tokio::task;

#[derive(Deserialize)]
pub struct RunRequest {
    pub yaml_content: String,
}

#[derive(Serialize)]
pub struct RunResponse {
    pub run_id: String,
    pub namespace: String,
    pub status: String,
}

#[post("/api/v1/tests/run")]
async fn submit_run(data: web::Data<AppState>, req: web::Json<RunRequest>) -> impl Responder {
    // 1. Parse YAML
    let test_plan = match TestPlan::from_yaml_str(&req.yaml_content) {
        Ok(plan) => plan,
        Err(e) => return HttpResponse::BadRequest().body(format!("Invalid YAML: {}", e)),
    };

    // 2. Compile to ExecutionPlan
    let exec_plan = match Compiler::compile(&test_plan) {
        Ok(plan) => plan,
        Err(e) => return HttpResponse::BadRequest().body(format!("Compilation failed: {}", e)),
    };

    let run_id = exec_plan.run_id.clone();
    let namespace = exec_plan.namespace.clone();

    // 3. Store the plan in Sled
    if let Err(e) = data.sled.store_run(&run_id, &serde_json::to_string(&exec_plan).unwrap()) {
        return HttpResponse::InternalServerError().body(format!("Storage error: {}", e));
    }
    
    // Mark as Submitted
    let submitted_state = serde_json::to_string(&crate::engine::state::RunState::Submitted).unwrap();
    if let Err(e) = data.sled.set_run_state(&run_id, &submitted_state) {
        return HttpResponse::InternalServerError().body(format!("Storage error: {}", e));
    }

    // 4. Spawn background manager
    let sled_clone = data.sled.clone();
    let run_id_clone = run_id.clone();
    task::spawn(async move {
        let manager = RunManager::new(exec_plan, sled_clone);
        if let Err(e) = manager.execute().await {
            eprintln!("RunManager failed for {}: {}", run_id_clone, e);
        }
    });

    HttpResponse::Accepted().json(RunResponse {
        run_id,
        namespace,
        status: "Submitted".to_string(),
    })
}

#[get("/api/v1/tests/runs")]
async fn list_runs(data: web::Data<AppState>) -> impl Responder {
    match data.sled.list_runs() {
        Ok(runs) => HttpResponse::Ok().json(runs),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error listing runs: {}", e)),
    }
}

#[get("/api/v1/tests/runs/{run_id}")]
async fn get_run(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let run_id = path.into_inner();
    
    // Get plan metadata
    let plan_str = match data.sled.get_run(&run_id) {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().body("Run not found"),
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    
    // Get state
    let state_str = match data.sled.get_run_state(&run_id) {
        Ok(Some(s)) => s,
        Ok(None) => "Unknown".to_string(),
        Err(_) => "Error".to_string(),
    };
    
    // Simplistic response for now
    let response = serde_json::json!({
        "run_id": run_id,
        "plan": serde_json::from_str::<serde_json::Value>(&plan_str).unwrap_or(serde_json::Value::Null),
        "state": serde_json::from_str::<serde_json::Value>(&state_str).unwrap_or(serde_json::Value::String(state_str))
    });
    
    HttpResponse::Ok().json(response)
}

#[get("/api/v1/tests/runs/{run_id}/results")]
async fn get_run_results(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let run_id = path.into_inner();
    match data.sled.get_run_results(&run_id) {
        Ok(results) => {
            // Parse each string as JSON
            let json_results: Vec<serde_json::Value> = results.into_iter()
                .filter_map(|r| serde_json::from_str(&r).ok())
                .collect();
            HttpResponse::Ok().json(json_results)
        },
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/api/v1/system/metrics/history")]
async fn get_metrics_history(data: web::Data<AppState>) -> impl Responder {
    let metrics = data.metrics_history.read().await;
    let metrics_vec: Vec<_> = metrics.iter().cloned().collect();
    HttpResponse::Ok().json(metrics_vec)
}

pub fn configure_api_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(submit_run)
       .service(list_runs)
       .service(get_run)
       .service(get_run_results)
       .service(get_metrics_history);
}
