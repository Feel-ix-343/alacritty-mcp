use alacritty_mcp::{AlacrittyManager, types::*};
use serde_json::json;

#[tokio::test]
async fn test_alacritty_manager_creation() {
    let _manager = AlacrittyManager::new();
    // Test passes if no panic occurs
}

#[tokio::test]
async fn test_alacritty_instance_serialization() {
    let instance = AlacrittyInstance {
        id: "test-id".to_string(),
        pid: 12345,
        window_id: Some(67890),
        title: "test-title".to_string(),
        command: "test-command".to_string(),
        created_at: 1234567890,
    };

    let json_str = serde_json::to_string(&instance).unwrap();
    let deserialized: AlacrittyInstance = serde_json::from_str(&json_str).unwrap();

    assert_eq!(instance.id, deserialized.id);
    assert_eq!(instance.pid, deserialized.pid);
    assert_eq!(instance.window_id, deserialized.window_id);
    assert_eq!(instance.title, deserialized.title);
    assert_eq!(instance.command, deserialized.command);
    assert_eq!(instance.created_at, deserialized.created_at);
}

#[tokio::test]
async fn test_spawn_params_deserialization() {
    let json_data = json!({
        "command": "bash",
        "args": ["-c", "echo hello"],
        "working_directory": "/tmp",
        "title": "Test Terminal"
    });

    let params: SpawnParams = serde_json::from_value(json_data).unwrap();
    
    assert_eq!(params.command, Some("bash".to_string()));
    assert_eq!(params.args, Some(vec!["-c".to_string(), "echo hello".to_string()]));
    assert_eq!(params.working_directory, Some("/tmp".to_string()));
    assert_eq!(params.title, Some("Test Terminal".to_string()));
}

#[tokio::test]
async fn test_spawn_params_minimal() {
    let json_data = json!({});

    let params: SpawnParams = serde_json::from_value(json_data).unwrap();
    
    assert_eq!(params.command, None);
    assert_eq!(params.args, None);
    assert_eq!(params.working_directory, None);
    assert_eq!(params.title, None);
}

#[tokio::test]
async fn test_send_keys_params() {
    let json_data = json!({
        "instance_id": "test-id",
        "keys": "ctrl+c"
    });

    let params: SendKeysParams = serde_json::from_value(json_data).unwrap();
    
    assert_eq!(params.instance_id, "test-id");
    assert_eq!(params.keys, "ctrl+c");
}

#[tokio::test]
async fn test_screenshot_params() {
    let json_data = json!({
        "instance_id": "test-id",
        "format": "text"
    });

    let params: ScreenshotParams = serde_json::from_value(json_data).unwrap();
    
    assert_eq!(params.instance_id, "test-id");
    assert_eq!(params.format, Some("text".to_string()));
}

#[tokio::test]
async fn test_screenshot_params_default_format() {
    let json_data = json!({
        "instance_id": "test-id"
    });

    let params: ScreenshotParams = serde_json::from_value(json_data).unwrap();
    
    assert_eq!(params.instance_id, "test-id");
    assert_eq!(params.format, None);
}

#[tokio::test]
async fn test_json_rpc_request_deserialization() {
    let json_data = json!({
        "jsonrpc": "2.0",
        "method": "test_method",
        "params": {"key": "value"},
        "id": 42
    });

    let request: JsonRpcRequest = serde_json::from_value(json_data).unwrap();
    
    assert_eq!(request.jsonrpc, "2.0");
    assert_eq!(request.method, "test_method");
    assert!(request.params.is_some());
    assert_eq!(request.id, Some(json!(42)));
}

#[tokio::test]
async fn test_json_rpc_response_serialization() {
    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(json!({"success": true})),
        error: None,
        id: Some(json!(1)),
    };

    let json_str = serde_json::to_string(&response).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["result"]["success"], true);
    assert!(parsed["error"].is_null());
    assert_eq!(parsed["id"], 1);
}

#[tokio::test]
async fn test_json_rpc_error_response() {
    let error = JsonRpcError {
        code: -32601,
        message: "Method not found".to_string(),
        data: None,
    };

    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(error),
        id: Some(json!(1)),
    };

    let json_str = serde_json::to_string(&response).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert!(parsed["result"].is_null());
    assert_eq!(parsed["error"]["code"], -32601);
    assert_eq!(parsed["error"]["message"], "Method not found");
    assert_eq!(parsed["id"], 1);
}

#[test]
fn test_base64_encoding() {
    // Test our custom base64 implementation
    use alacritty_mcp::alacritty_manager::base64;
    
    let test_data = b"Hello, World!";
    let encoded = base64::encode(test_data);
    
    // This should match the standard base64 encoding
    assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
    
    let empty_data = b"";
    let encoded_empty = base64::encode(empty_data);
    assert_eq!(encoded_empty, "");
    
    let single_byte = b"A";
    let encoded_single = base64::encode(single_byte);
    assert_eq!(encoded_single, "QQ==");
}