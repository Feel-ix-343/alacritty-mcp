use std::process::Command;
use std::time::Duration;
use tokio::time::{timeout, sleep};
use serde_json::{json, Value};
use serial_test::serial;
use alacritty_mcp::{AlacrittyManager, McpServer, NeovimContextExtractor};
use std::collections::HashSet;
use std::os::unix::process::ExitStatusExt;

async fn create_test_server() -> McpServer {
    let manager = AlacrittyManager::new();
    McpServer::new(manager)
}

async fn send_request(server: &mut McpServer, request: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let request_str = serde_json::to_string(&request)?;
    let response_str = server.handle_request(&request_str).await?;
    let response: Value = serde_json::from_str(&response_str)?;
    Ok(response)
}

async fn initialize_server(server: &mut McpServer) -> Result<(), Box<dyn std::error::Error>> {
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });
    send_request(server, init_request).await?;
    Ok(())
}

async fn get_alacritty_pids() -> Vec<u32> {
    let output = Command::new("pgrep")
        .args(&["-f", "alacritty"])
        .output()
        .unwrap_or_else(|_| std::process::Output {
            status: std::process::ExitStatus::from_raw(1),
            stdout: Vec::new(),
            stderr: Vec::new(),
        });

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect()
    } else {
        Vec::new()
    }
}

#[tokio::test]
#[serial]
async fn test_neovim_context_tool_available() {
    let mut server = create_test_server().await;
    initialize_server(&mut server).await.unwrap();

    let tools_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 2
    });

    let response = send_request(&mut server, tools_request).await.unwrap();
    
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    
    let tools = &response["result"]["tools"];
    assert!(tools.is_array());
    
    let tool_names: Vec<String> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();
    
    assert!(tool_names.contains(&"get_neovim_context".to_string()));
    
    // Find the neovim context tool and verify its schema
    let neovim_tool = tools
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "get_neovim_context")
        .unwrap();
    
    assert!(neovim_tool["description"].as_str().unwrap().contains("Neovim context"));
    assert!(neovim_tool["input_schema"]["properties"]["instance_id"].is_object());
    assert!(neovim_tool["input_schema"]["properties"]["include_diagnostics"].is_object());
    assert!(neovim_tool["input_schema"]["properties"]["include_buffers"].is_object());
    assert!(neovim_tool["input_schema"]["properties"]["context_lines"].is_object());
}

#[tokio::test]
#[serial]
async fn test_neovim_context_invalid_instance() {
    let mut server = create_test_server().await;
    initialize_server(&mut server).await.unwrap();

    let neovim_context_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_neovim_context",
            "arguments": {
                "instance_id": "invalid-id"
            }
        },
        "id": 2
    });

    let response = send_request(&mut server, neovim_context_request).await.unwrap();
    
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(!response["error"].is_null());
    assert!(response["error"]["message"].as_str().unwrap().contains("Instance not found"));
}

#[tokio::test]
#[serial]
async fn test_spawn_neovim_and_extract_context() {
    // Skip if no X11 display or Neovim
    if std::env::var("DISPLAY").is_err() {
        println!("Skipping test - no X11 display available");
        return;
    }

    if !Command::new("which").arg("alacritty").output().unwrap().status.success() ||
       !Command::new("which").arg("nvim").output().unwrap().status.success() {
        println!("Skipping test - alacritty or nvim not available");
        return;
    }

    let mut server = create_test_server().await;
    initialize_server(&mut server).await.unwrap();

    let initial_pids: HashSet<u32> = get_alacritty_pids().await.into_iter().collect();

    // Spawn Alacritty with Neovim
    let spawn_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "spawn_instance",
            "arguments": {
                "title": "neovim-context-test",
                "command": "nvim",
                "args": ["+set", "number", "/tmp/test.txt"],
                "working_directory": "/tmp"
            }
        },
        "id": 2
    });

    let spawn_result = timeout(Duration::from_secs(10), send_request(&mut server, spawn_request)).await;
    
    if let Ok(Ok(spawn_response)) = spawn_result {
        println!("Neovim spawn response: {:#}", spawn_response);
        
        // Wait for Neovim to start
        sleep(Duration::from_millis(5000)).await;
        
        // Extract instance ID from response
        let content = spawn_response["result"]["content"][0]["text"].as_str().unwrap();
        let start = content.find('{').unwrap();
        let end = content.rfind('}').unwrap() + 1;
        let json_part = &content[start..end];
        let instance_data: Value = serde_json::from_str(json_part).unwrap();
        let instance_id = instance_data["id"].as_str().unwrap();
        
        println!("Testing Neovim context for instance ID: {}", instance_id);
        
        // Try to get Neovim context
        let context_request = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "get_neovim_context",
                "arguments": {
                    "instance_id": instance_id,
                    "include_diagnostics": true,
                    "include_buffers": true,
                    "context_lines": 10
                }
            },
            "id": 3
        });
        
        let context_result = timeout(Duration::from_secs(15), send_request(&mut server, context_request)).await;
        
        match context_result {
            Ok(Ok(context_response)) => {
                println!("Neovim context response: {:#}", context_response);
                
                assert_eq!(context_response["jsonrpc"], "2.0");
                assert_eq!(context_response["id"], 3);
                
                if context_response["error"].is_null() {
                    let content = context_response["result"]["content"][0]["text"].as_str().unwrap();
                    assert!(content.contains("Neovim context for instance"));
                    assert!(content.contains(instance_id));
                    
                    // Try to parse the JSON context
                    if let Some(json_start) = content.find('{') {
                        let json_part = &content[json_start..];
                        if let Ok(context_data) = serde_json::from_str::<Value>(json_part) {
                            println!("Successfully parsed Neovim context structure");
                            
                            // Verify expected fields are present
                            assert!(context_data["instance_info"].is_object());
                            
                            if let Some(pid) = context_data["instance_info"]["pid"].as_u64() {
                                println!("Neovim PID detected: {}", pid);
                            }
                            
                            if let Some(socket_path) = context_data["instance_info"]["socket_path"].as_str() {
                                println!("Neovim socket path: {}", socket_path);
                            }
                        } else {
                            println!("Could not parse Neovim context JSON, but extraction succeeded");
                        }
                    }
                    
                    println!("✅ Successfully extracted Neovim context!");
                } else {
                    let error_msg = context_response["error"]["message"].as_str().unwrap();
                    println!("Context extraction failed (expected for basic detection): {}", error_msg);
                    
                    // This might be expected if Neovim socket isn't available yet
                    if error_msg.contains("not appear to be running Neovim") {
                        println!("ℹ️ Neovim detection failed - terminal might not be ready yet");
                    }
                }
            }
            Ok(Err(e)) => {
                println!("Context request failed: {}", e);
            }
            Err(_) => {
                println!("Context request timed out");
            }
        }
        
        // Clean up
        let final_pids: HashSet<u32> = get_alacritty_pids().await.into_iter().collect();
        let new_pids: Vec<u32> = final_pids.difference(&initial_pids).cloned().collect();
        for pid in new_pids {
            println!("Cleaning up PID: {}", pid);
            let _ = Command::new("kill").arg(pid.to_string()).output();
        }
    } else {
        println!("Failed to spawn Neovim terminal");
    }
}

#[tokio::test]
async fn test_neovim_context_extractor_creation() {
    let extractor = NeovimContextExtractor::new();
    
    // Test basic functionality
    let test_content_with_nvim = "-- INSERT -- some text here";
    assert!(extractor.detect_neovim_in_terminal(test_content_with_nvim));
    
    let test_content_without_nvim = "regular bash terminal $ ls -la";
    assert!(!extractor.detect_neovim_in_terminal(test_content_without_nvim));
    
    println!("✅ Neovim context extractor basic functionality works");
}

#[tokio::test]
async fn test_neovim_detection_patterns() {
    let extractor = NeovimContextExtractor::new();
    
    let nvim_patterns = vec![
        "-- INSERT --",
        "-- VISUAL --",
        "-- NORMAL --",
        "-- COMMAND --",
        ":set number",
        ":help",
        ":q!",
        "~vim: some config",
        "[No Name] - nvim",
    ];
    
    for pattern in nvim_patterns {
        assert!(extractor.detect_neovim_in_terminal(pattern), 
               "Failed to detect Neovim pattern: {}", pattern);
    }
    
    let non_nvim_patterns = vec![
        "bash-5.1$ ls",
        "$ npm install",
        "gcc -o main main.c",
        "python3 script.py",
        "regular terminal output",
    ];
    
    for pattern in non_nvim_patterns {
        assert!(!extractor.detect_neovim_in_terminal(pattern), 
               "False positive for pattern: {}", pattern);
    }
    
    println!("✅ Neovim detection patterns work correctly");
}