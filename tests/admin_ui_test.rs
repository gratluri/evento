use actix_web::{test, web, App};
use evento::admin::server::{configure_admin_routes, AppState, SystemStatus};
use evento::engine::storage::sled_store::SledStore;
use evento::engine::config::StorageConfig;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::RwLock;
use scraper::{Html, Selector};
use serde_json::Value;

fn create_mock_state() -> web::Data<AppState> {
    let temp = tempdir().unwrap();
    let mut config = StorageConfig::default();
    config.data_dir = temp.path().to_path_buf();
    let sled_store = Arc::new(SledStore::new(&config).unwrap());

    web::Data::new(AppState {
        status: Arc::new(RwLock::new(SystemStatus {
            evento_core: "Active".to_string(),
            sled_store: "Green".to_string(),
            postgres_store: "Green".to_string(),
            mcp_server: "Green".to_string(),
            uptime_seconds: 42,
        })),
        sled: sled_store,
    })
}

#[actix_web::test]
async fn test_admin_ui_html_components() {
    // 1. Initialize the app for testing
    let app = test::init_service(App::new().app_data(create_mock_state()).configure(configure_admin_routes)).await;

    // 2. Make a request to the root HTML route
    let req = test::TestRequest::get().uri("/").to_request();
    let resp = test::call_service(&app, req).await;

    // 3. Ensure it returns 200 OK
    assert!(resp.status().is_success());

    // 4. Extract HTML content
    let body = test::read_body(resp).await;
    let html_str = String::from_utf8(body.to_vec()).expect("Response is not valid UTF-8");
    
    // 5. Parse the HTML using scraper
    let document = Html::parse_document(&html_str);

    // 6. Define selectors for required components
    let evento_card_selector = Selector::parse("#card-evento").unwrap();
    let sled_card_selector = Selector::parse("#card-sled").unwrap();
    let postgres_card_selector = Selector::parse("#card-postgres").unwrap();
    let runs_section_selector = Selector::parse("#runs").unwrap();
    let inspector_selector = Selector::parse("#run-inspector").unwrap();
    
    // 7. Assert that all critical UI components exist in the DOM
    assert!(document.select(&evento_card_selector).next().is_some(), "Evento card missing from UI");
    assert!(document.select(&sled_card_selector).next().is_some(), "Sled card missing from UI");
    assert!(document.select(&postgres_card_selector).next().is_some(), "Postgres card missing from UI");
    assert!(document.select(&runs_section_selector).next().is_some(), "Runs section missing from UI");
    assert!(document.select(&inspector_selector).next().is_some(), "Run inspector missing from UI");
}

#[actix_web::test]
async fn test_admin_api_status() {
    let app = test::init_service(App::new().app_data(create_mock_state()).configure(configure_admin_routes)).await;
    let req = test::TestRequest::get().uri("/api/status").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let json: Value = serde_json::from_slice(&body).expect("Response is not valid JSON");

    // Ensure all critical status fields are returned
    assert!(json.get("evento_core").is_some(), "Missing evento_core status");
    assert!(json.get("sled_store").is_some(), "Missing sled_store status");
    assert!(json.get("postgres_store").is_some(), "Missing postgres_store status");
    assert!(json.get("mcp_server").is_some(), "Missing mcp_server status");
}
