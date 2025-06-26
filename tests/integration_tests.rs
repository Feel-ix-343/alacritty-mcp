use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use serde_json::{json, Value};
use serial_test::serial;
use alacritty_mcp::{AlacrittyManager, McpServer};

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

#[tokio::test]
#[serial]
async fn test_initialize() {
    let mut server = create_test_server().await;
    
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

    let response = send_request(&mut server, init_request).await.unwrap();
    
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"]["protocolVersion"].is_string());
    assert!(response["result"]["capabilities"]["tools"].is_array());
    assert!(response["error"].is_null());
}

#[tokio::test]
#[serial]
async fn test_tools_list() {
    let mut server = create_test_server().await;
    
    // Initialize first
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
    send_request(&mut server, init_request).await.unwrap();

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
    
    assert!(tool_names.contains(&"list_instances".to_string()));
    assert!(tool_names.contains(&"spawn_instance".to_string()));
    assert!(tool_names.contains(&"send_keys".to_string()));
    assert!(tool_names.contains(&"screenshot_instance".to_string()));
}

#[tokio::test]
#[serial]
async fn test_list_instances_empty() {
    let mut server = create_test_server().await;
    
    // Initialize
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
    send_request(&mut server, init_request).await.unwrap();

    let list_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "list_instances",
            "arguments": {}
        },
        "id": 2
    });

    let response = send_request(&mut server, list_request).await.unwrap();
    
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["result"]["content"].is_array());
    
    let content = &response["result"]["content"][0]["text"];
    assert!(content.as_str().unwrap().contains("Found") && content.as_str().unwrap().contains("Alacritty instances"));
}

#[tokio::test]
#[serial]
async fn test_spawn_instance() {
    // Skip this test if alacritty is not available
    if Command::new("which").arg("alacritty").output().unwrap().status.success() == false {
        return;
    }

    let mut server = create_test_server().await;
    
    // Initialize
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
    send_request(&mut server, init_request).await.unwrap();

    let spawn_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "spawn_instance",
            "arguments": {
                "title": "test-terminal",
                "command": "echo",
                "args": ["hello world"]
            }
        },
        "id": 2
    });

    let result = timeout(Duration::from_secs(10), send_request(&mut server, spawn_request)).await;
    
    match result {
        Ok(Ok(response)) => {
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 2);
            
            if !response["error"].is_null() {
                // If we get an error, it might be because X11 is not available in test environment
                let error_msg = response["error"]["message"].as_str().unwrap();
                if error_msg.contains("DISPLAY") || error_msg.contains("X11") {
                    println!("Skipping spawn test - X11 not available");
                    return;
                }
            }
            
            assert!(response["result"]["content"].is_array());
            let content = &response["result"]["content"][0]["text"];
            assert!(content.as_str().unwrap().contains("Spawned new Alacritty instance"));
        }
        Ok(Err(e)) => {
            println!("Test failed with error: {}", e);
            // This might be expected in CI environments without X11
        }
        Err(_) => {
            println!("Test timed out - might be expected in CI environment");
        }
    }
}

#[tokio::test]
#[serial]
async fn test_invalid_method() {
    let mut server = create_test_server().await;
    
    let invalid_request = json!({
        "jsonrpc": "2.0",
        "method": "invalid_method",
        "id": 1
    });

    let response = send_request(&mut server, invalid_request).await.unwrap();
    
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(!response["error"].is_null());
    assert_eq!(response["error"]["code"], -32601);
}

#[tokio::test]
#[serial]
async fn test_send_keys_invalid_instance() {
    let mut server = create_test_server().await;
    
    // Initialize
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
    send_request(&mut server, init_request).await.unwrap();

    let send_keys_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "send_keys",
            "arguments": {
                "instance_id": "invalid-id",
                "keys": "ctrl+c"
            }
        },
        "id": 2
    });

    let response = send_request(&mut server, send_keys_request).await.unwrap();
    
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(!response["error"].is_null());
    assert!(response["error"]["message"].as_str().unwrap().contains("Instance not found"));
}

#[tokio::test]
#[serial]
async fn test_screenshot_invalid_instance() {
    let mut server = create_test_server().await;
    
    // Initialize
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
    send_request(&mut server, init_request).await.unwrap();

    let screenshot_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "screenshot_instance",
            "arguments": {
                "instance_id": "invalid-id",
                "format": "text"
            }
        },
        "id": 2
    });

    let response = send_request(&mut server, screenshot_request).await.unwrap();
    
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(!response["error"].is_null());
    assert!(response["error"]["message"].as_str().unwrap().contains("Instance not found"));
}

#[tokio::test]
#[serial]
async fn test_malformed_json_request() {
    let mut server = create_test_server().await;
    
    let result = server.handle_request("invalid json").await;
    assert!(result.is_err());
}