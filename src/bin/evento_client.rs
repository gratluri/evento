use clap::{Parser, Subcommand};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};

#[derive(Parser, Debug)]
#[command(name = "evento-client", version, about = "CLI for interacting with Evento Server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Submit a test plan for execution
    Run {
        /// The URL of the Evento server
        #[arg(short, long, default_value = "http://localhost:8080")]
        server: String,
        
        /// The path to the YAML test plan
        #[arg(short, long, value_name = "FILE")]
        plan: PathBuf,
    },
    /// List all runs
    List {
        #[arg(short, long, default_value = "http://localhost:8080")]
        server: String,
    },
    /// Get the status of a specific run
    Status {
        #[arg(short, long, default_value = "http://localhost:8080")]
        server: String,
        
        #[arg(short, long)]
        run_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new();

    match cli.command {
        Commands::Run { server, plan } => {
            let yaml_content = fs::read_to_string(&plan)
                .with_context(|| format!("Failed to read plan file {:?}", plan))?;
            
            let url = format!("{}/api/v1/tests/run", server);
            let payload = serde_json::json!({
                "yaml_content": yaml_content
            });
            
            let res = client.post(&url)
                .json(&payload)
                .send()
                .await
                .with_context(|| format!("Failed to connect to server at {}", server))?;
            
            if res.status().is_success() {
                let response_json: Value = res.json().await?;
                println!("Run submitted successfully!");
                println!("{}", serde_json::to_string_pretty(&response_json)?);
            } else {
                eprintln!("Error submitting run: {}", res.status());
                eprintln!("{}", res.text().await?);
            }
        }
        Commands::List { server } => {
            let url = format!("{}/api/v1/tests/runs", server);
            let res = client.get(&url)
                .send()
                .await?;
                
            if res.status().is_success() {
                let runs: Vec<String> = res.json().await?;
                println!("Runs:");
                for run in runs {
                    println!("  - {}", run);
                }
            } else {
                eprintln!("Error listing runs: {}", res.status());
            }
        }
        Commands::Status { server, run_id } => {
            let url = format!("{}/api/v1/tests/runs/{}", server, run_id);
            let res = client.get(&url)
                .send()
                .await?;
                
            if res.status().is_success() {
                let status: Value = res.json().await?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                eprintln!("Error fetching status: {}", res.status());
                eprintln!("{}", res.text().await?);
            }
        }
    }

    Ok(())
}
