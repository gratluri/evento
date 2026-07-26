use clap::Parser;
use cli::parser::{Cli, Commands};
use mcp::server::McpServer;
use dsl::dsl_parser::TestPlan;
use anyhow::Result;

pub mod cli;
pub mod dsl;
pub mod engine;
pub mod mcp;
pub mod metrics;
pub mod protocols;
pub mod admin;
pub mod simulator;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Run { plan } => {
            println!("Loading Test Plan from: {:?}", plan);
            let test_plan = TestPlan::from_file(plan)?;
            println!("Successfully parsed Test Plan:");
            println!("{:#?}", test_plan);
            // TODO: Pass test_plan to engine runner
        }
        Commands::Mcp { port } => {
            let server = McpServer::new(*port);
            server.start().await?;
        }
        Commands::Server { port } => {
            let config = engine::config::StorageConfig::default();
            admin::server::start_admin_server(*port, config).await?;
        }
    }

    Ok(())
}
