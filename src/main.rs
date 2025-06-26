use std::io::{self, BufRead, Write};
use anyhow::Result;
use tracing::{info, error};

mod alacritty_manager;
mod mcp_server;
mod types;

use alacritty_manager::AlacrittyManager;
use mcp_server::McpServer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let manager = AlacrittyManager::new();
    let mut server = McpServer::new(manager);
    
    info!("Starting Alacritty MCP Server");
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        
        match server.handle_request(&line).await {
            Ok(response) => {
                writeln!(stdout, "{}", response)?;
                stdout.flush()?;
            }
            Err(e) => {
                error!("Error handling request: {}", e);
                let error_response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32603,
                        "message": e.to_string()
                    },
                    "id": null
                });
                writeln!(stdout, "{}", error_response)?;
                stdout.flush()?;
            }
        }
    }
    
    Ok(())
}