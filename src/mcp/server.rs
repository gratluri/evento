use anyhow::Result;
use mcp_core::{
    server::Server,
    transport::ServerSseTransport,
    types::{CallToolRequest, ServerCapabilities, Tool},
};
use serde_json::json;
use std::fs;
use std::path::Path;

use crate::dsl::dsl_parser::TestPlan;

pub struct McpServer {
    port: u16,
}

impl McpServer {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn start(&self) -> Result<()> {
        println!("Initializing Evento MCP Server on port {}...", self.port);

        let get_spec_tool = Tool {
            name: "get_dsl_specification".to_string(),
            description: Some("Retrieve the markdown specification for the Evento DSL to understand how to construct valid load testing scenarios.".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {},
            }),
            annotations: None,
        };

        let validate_dsl_tool = Tool {
            name: "validate_dsl".to_string(),
            description: Some("Validates a generated Evento YAML string to ensure it parses correctly into the TestPlan AST.".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "yaml_content": { 
                        "type": "string",
                        "description": "The raw YAML string representing the Evento test plan."
                    }
                },
                "required": ["yaml_content"]
            }),
            annotations: None,
        };

        let server = Server::builder("evento-mcp".to_string(), "0.1.0".to_string(), mcp_core::types::LATEST_PROTOCOL_VERSION)
            .set_capabilities(ServerCapabilities {
                tools: Some(mcp_core::types::ToolCapabilities { list_changed: Some(false) }),
                ..Default::default()
            })
            .register_tool(get_spec_tool, |_: CallToolRequest| {
                Box::pin(async move {
                    // In a real scenario, this path might need to be resolved differently or embedded in the binary
                    let spec_path = Path::new("DSL_SPECIFICATION.md");
                    match fs::read_to_string(spec_path) {
                        Ok(content) => mcp_core::tool_text_response!(content),
                        Err(e) => mcp_core::tool_text_response!(format!("Failed to read specification: {}", e)),
                    }
                })
            })
            .register_tool(validate_dsl_tool, |request: CallToolRequest| {
                Box::pin(async move {
                    let args = request.arguments.unwrap_or_default();
                    if let Some(yaml_value) = args.get("yaml_content") {
                        if let Some(yaml_str) = yaml_value.as_str() {
                            match TestPlan::from_yaml_str(yaml_str) {
                                Ok(_) => return mcp_core::tool_text_response!("Validation successful. The YAML is a valid Evento TestPlan."),
                                Err(e) => return mcp_core::tool_text_response!(format!("Validation failed: {:?}", e)),
                            }
                        }
                    }
                    mcp_core::tool_text_response!("Invalid arguments: missing or invalid 'yaml_content' string.")
                })
            })
            .build();

        let transport = ServerSseTransport::new("0.0.0.0".to_string(), self.port, server);

        println!("Starting SSE MCP server on http://0.0.0.0:{}/sse", self.port);
        Server::start(transport).await?;

        Ok(())
    }
}
