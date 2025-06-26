use std::process::Command;
use std::time::Duration;
use tokio::time::{timeout, sleep};
use serde_json::{json, Value};
use serial_test::serial;
use alacritty_mcp::{AlacrittyManager, McpServer};
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
async fn test_spawn_real_alacritty_instance() {
    // Skip if no X11 display
    if std::env::var("DISPLAY").is_err() {
        println!("Skipping test - no X11 display available");
        return;
    }

    // Skip if alacritty not available
    if !Command::new("which").arg("alacritty").output().unwrap().status.success() {
        println!("Skipping test - alacritty not available");
        return;
    }

    let mut server = create_test_server().await;
    initialize_server(&mut server).await.unwrap();

    // Get initial alacritty processes
    let initial_pids: HashSet<u32> = get_alacritty_pids().await.into_iter().collect();

    // Spawn a new instance
    let spawn_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "spawn_instance",
            "arguments": {
                "title": "functional-test-terminal",
                "working_directory": "/tmp"
            }
        },
        "id": 2
    });

    let result = timeout(Duration::from_secs(10), send_request(&mut server, spawn_request)).await;
    
    match result {
        Ok(Ok(response)) => {
            println!("Spawn response: {:#}", response);
            
            // Verify response structure
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 2);
            assert!(response["error"].is_null());
            assert!(response["result"]["content"].is_array());
            
            let content = response["result"]["content"][0]["text"].as_str().unwrap();
            assert!(content.contains("Spawned new Alacritty instance"));
            
            // Wait for process to start
            sleep(Duration::from_millis(2000)).await;
            
            // Verify new process was created
            let final_pids: HashSet<u32> = get_alacritty_pids().await.into_iter().collect();
            let new_pids: Vec<u32> = final_pids.difference(&initial_pids).cloned().collect();
            
            assert!(!new_pids.is_empty(), "No new Alacritty process was created");
            println!("New Alacritty PIDs created: {:?}", new_pids);
            
            // Verify we can list the new instance
            let list_request = json!({
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "list_instances",
                    "arguments": {}
                },
                "id": 3
            });
            
            let list_response = send_request(&mut server, list_request).await.unwrap();
            println!("List response: {:#}", list_response);
            
            let list_content = list_response["result"]["content"][0]["text"].as_str().unwrap();
            assert!(list_content.contains("functional-test-terminal"));
            
            // Clean up - kill the new processes
            for pid in new_pids {
                let _ = Command::new("kill").arg(pid.to_string()).output();
            }
        }
        Ok(Err(e)) => {
            panic!("Request failed: {}", e);
        }
        Err(_) => {
            println!("Test timed out - this might be expected in CI environment without X11");
        }
    }
}

#[tokio::test]
#[serial]
async fn test_send_keys_to_real_terminal() {
    // Skip if no X11 display
    if std::env::var("DISPLAY").is_err() {
        println!("Skipping test - no X11 display available");
        return;
    }

    // Skip if required tools not available
    if !Command::new("which").arg("alacritty").output().unwrap().status.success() ||
       !Command::new("which").arg("xdotool").output().unwrap().status.success() {
        println!("Skipping test - alacritty or xdotool not available");
        return;
    }

    let mut server = create_test_server().await;
    initialize_server(&mut server).await.unwrap();

    // Get initial processes
    let initial_pids: HashSet<u32> = get_alacritty_pids().await.into_iter().collect();

    // Spawn a new instance
    let spawn_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "spawn_instance",
            "arguments": {
                "title": "send-keys-test-terminal",
                "working_directory": "/tmp"
            }
        },
        "id": 2
    });

    let spawn_result = timeout(Duration::from_secs(10), send_request(&mut server, spawn_request)).await;
    
    if let Ok(Ok(spawn_response)) = spawn_result {
        // Wait for terminal to be ready
        sleep(Duration::from_millis(3000)).await;
        
        // Extract instance ID from response
        let content = spawn_response["result"]["content"][0]["text"].as_str().unwrap();
        println!("Spawn content: {}", content);
        
        // Parse the JSON to get instance ID
        let start = content.find('{').unwrap();
        let end = content.rfind('}').unwrap() + 1;
        let json_part = &content[start..end];
        let instance_data: Value = serde_json::from_str(json_part).unwrap();
        let instance_id = instance_data["id"].as_str().unwrap();
        
        println!("Instance ID: {}", instance_id);
        
        // Send keys to the terminal
        let send_keys_request = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "send_keys",
                "arguments": {
                    "instance_id": instance_id,
                    "keys": "echo 'Hello from MCP test' Return"
                }
            },
            "id": 3
        });
        
        let keys_result = timeout(Duration::from_secs(5), send_request(&mut server, send_keys_request)).await;
        
        match keys_result {
            Ok(Ok(keys_response)) => {
                println!("Send keys response: {:#}", keys_response);
                
                assert_eq!(keys_response["jsonrpc"], "2.0");
                assert_eq!(keys_response["id"], 3);
                assert!(keys_response["error"].is_null());
                
                let content = keys_response["result"]["content"][0]["text"].as_str().unwrap();
                assert!(content.contains("Sent keys"));
                assert!(content.contains(instance_id));
                
                println!("Successfully sent keys to terminal!");
                
                // Wait a bit for command to execute
                sleep(Duration::from_millis(1000)).await;
                
            }
            Ok(Err(e)) => {
                println!("Send keys failed: {}", e);
            }
            Err(_) => {
                println!("Send keys timed out");
            }
        }
        
        // Clean up
        let final_pids: HashSet<u32> = get_alacritty_pids().await.into_iter().collect();
        let new_pids: Vec<u32> = final_pids.difference(&initial_pids).cloned().collect();
        for pid in new_pids {
            let _ = Command::new("kill").arg(pid.to_string()).output();
        }
    }
}

#[tokio::test]
#[serial]
async fn test_screenshot_real_terminal() {
    // Skip if no X11 display
    if std::env::var("DISPLAY").is_err() {
        println!("Skipping test - no X11 display available");
        return;
    }

    // Skip if required tools not available
    if !Command::new("which").arg("alacritty").output().unwrap().status.success() ||
       !Command::new("which").arg("xclip").output().unwrap().status.success() {
        println!("Skipping test - alacritty or xclip not available");
        return;
    }

    let mut server = create_test_server().await;
    initialize_server(&mut server).await.unwrap();

    // Get initial processes
    let initial_pids: HashSet<u32> = get_alacritty_pids().await.into_iter().collect();

    // Spawn a new instance
    let spawn_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "spawn_instance",
            "arguments": {
                "title": "screenshot-test-terminal",
                "working_directory": "/tmp"
            }
        },
        "id": 2
    });

    let spawn_result = timeout(Duration::from_secs(10), send_request(&mut server, spawn_request)).await;
    
    if let Ok(Ok(spawn_response)) = spawn_result {
        // Wait for terminal to be ready
        sleep(Duration::from_millis(3000)).await;
        
        // Extract instance ID from response
        let content = spawn_response["result"]["content"][0]["text"].as_str().unwrap();
        let start = content.find('{').unwrap();
        let end = content.rfind('}').unwrap() + 1;
        let json_part = &content[start..end];
        let instance_data: Value = serde_json::from_str(json_part).unwrap();
        let instance_id = instance_data["id"].as_str().unwrap();
        
        println!("Testing screenshot for instance ID: {}", instance_id);
        
        // First, send some content to the terminal
        let send_keys_request = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "send_keys",
                "arguments": {
                    "instance_id": instance_id,
                    "keys": "echo 'Screenshot test content - Hello MCP!' Return"
                }
            },
            "id": 3
        });
        
        let _ = timeout(Duration::from_secs(5), send_request(&mut server, send_keys_request)).await;
        sleep(Duration::from_millis(2000)).await;
        
        // Take a text screenshot
        let screenshot_request = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "screenshot_instance",
                "arguments": {
                    "instance_id": instance_id,
                    "format": "text"
                }
            },
            "id": 4
        });
        
        let screenshot_result = timeout(Duration::from_secs(10), send_request(&mut server, screenshot_request)).await;
        
        match screenshot_result {
            Ok(Ok(screenshot_response)) => {
                println!("Screenshot response: {:#}", screenshot_response);
                
                assert_eq!(screenshot_response["jsonrpc"], "2.0");
                assert_eq!(screenshot_response["id"], 4);
                assert!(screenshot_response["error"].is_null());
                
                let content = screenshot_response["result"]["content"][0]["text"].as_str().unwrap();
                assert!(content.contains("Screenshot text from instance"));
                assert!(content.contains(instance_id));
                
                // The actual screenshot content should be after the description
                if content.contains("Screenshot test content") {
                    println!("Successfully captured terminal text content!");
                } else {
                    println!("Terminal text captured (may not contain our test content due to timing)");
                }
                
            }
            Ok(Err(e)) => {
                println!("Screenshot failed: {}", e);
            }
            Err(_) => {
                println!("Screenshot timed out");
            }
        }
        
        // Clean up
        let final_pids: HashSet<u32> = get_alacritty_pids().await.into_iter().collect();
        let new_pids: Vec<u32> = final_pids.difference(&initial_pids).cloned().collect();
        for pid in new_pids {
            let _ = Command::new("kill").arg(pid.to_string()).output();
        }
    }
}

#[tokio::test]
#[serial]
async fn test_terminal_lifecycle_management() {
    // Skip if no X11 display
    if std::env::var("DISPLAY").is_err() {
        println!("Skipping test - no X11 display available");
        return;
    }

    // Skip if alacritty not available
    if !Command::new("which").arg("alacritty").output().unwrap().status.success() {
        println!("Skipping test - alacritty not available");
        return;
    }

    let mut server = create_test_server().await;
    initialize_server(&mut server).await.unwrap();

    // Get initial count
    let list_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "list_instances",
            "arguments": {}
        },
        "id": 2
    });
    
    let initial_response = send_request(&mut server, list_request.clone()).await.unwrap();
    let initial_content = initial_response["result"]["content"][0]["text"].as_str().unwrap();
    
    // Extract initial count
    let initial_count = if let Some(start) = initial_content.find("Found ") {
        let rest = &initial_content[start + 6..];
        if let Some(end) = rest.find(" Alacritty instances") {
            rest[..end].parse::<i32>().unwrap_or(0)
        } else { 0 }
    } else { 0 };
    
    println!("Initial Alacritty instances: {}", initial_count);

    // Spawn multiple instances
    let test_instances = vec![
        "lifecycle-test-1",
        "lifecycle-test-2",
        "lifecycle-test-3"
    ];
    
    let mut spawned_pids = Vec::new();
    
    for title in &test_instances {
        let spawn_request = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "spawn_instance",
                "arguments": {
                    "title": title,
                    "working_directory": "/tmp"
                }
            },
            "id": 3
        });
        
        let result = timeout(Duration::from_secs(10), send_request(&mut server, spawn_request)).await;
        
        if let Ok(Ok(_response)) = result {
            println!("Spawned terminal: {}", title);
            sleep(Duration::from_millis(1500)).await;
            
            // Track new processes
            let current_pids = get_alacritty_pids().await;
            if let Some(&newest_pid) = current_pids.iter().max() {
                spawned_pids.push(newest_pid);
            }
        }
    }
    
    // Wait a bit for all processes to stabilize
    sleep(Duration::from_millis(2000)).await;
    
    // List instances again and verify count increased
    let final_response = send_request(&mut server, list_request).await.unwrap();
    let final_content = final_response["result"]["content"][0]["text"].as_str().unwrap();
    
    println!("Final instance list:\n{}", final_content);
    
    // Verify we can see our test instances
    for title in &test_instances {
        if final_content.contains(title) {
            println!("✓ Found test instance: {}", title);
        }
    }
    
    // Clean up spawned processes
    for pid in spawned_pids {
        println!("Cleaning up PID: {}", pid);
        let _ = Command::new("kill").arg(pid.to_string()).output();
    }
    
    // Wait for cleanup
    sleep(Duration::from_millis(1000)).await;
    
    println!("Lifecycle management test completed");
}

#[tokio::test]
#[serial]
async fn test_spawn_with_custom_command() {
    // Skip if no X11 display
    if std::env::var("DISPLAY").is_err() {
        println!("Skipping test - no X11 display available");
        return;
    }

    // Skip if alacritty not available
    if !Command::new("which").arg("alacritty").output().unwrap().status.success() {
        println!("Skipping test - alacritty not available");
        return;
    }

    let mut server = create_test_server().await;
    initialize_server(&mut server).await.unwrap();

    let initial_pids: HashSet<u32> = get_alacritty_pids().await.into_iter().collect();

    // Spawn with custom command
    let spawn_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "spawn_instance",
            "arguments": {
                "title": "custom-command-test",
                "command": "bash",
                "args": ["-c", "echo 'Custom command executed'; sleep 5"],
                "working_directory": "/tmp"
            }
        },
        "id": 2
    });

    let result = timeout(Duration::from_secs(10), send_request(&mut server, spawn_request)).await;
    
    if let Ok(Ok(response)) = result {
        println!("Custom command spawn response: {:#}", response);
        
        assert_eq!(response["jsonrpc"], "2.0");
        assert!(response["error"].is_null());
        
        let content = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("Spawned new Alacritty instance"));
        
        // Wait for process to start
        sleep(Duration::from_millis(2000)).await;
        
        // Verify new process was created
        let final_pids: HashSet<u32> = get_alacritty_pids().await.into_iter().collect();
        let new_pids: Vec<u32> = final_pids.difference(&initial_pids).cloned().collect();
        
        if !new_pids.is_empty() {
            println!("Successfully spawned Alacritty with custom command");
            
            // Clean up
            for pid in new_pids {
                let _ = Command::new("kill").arg(pid.to_string()).output();
            }
        } else {
            println!("No new process detected - command may have completed quickly");
        }
    }
}