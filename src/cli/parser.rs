use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a test plan
    Run {
        /// The path to the YAML test plan
        #[arg(short, long, value_name = "FILE")]
        plan: PathBuf,
    },
    /// Start the MCP server
    Mcp {
        /// Port to run the MCP server on
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
    /// Start the Evento Server (Admin UI, REST API)
    Server {
        /// Port to run the server on
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    }
}
