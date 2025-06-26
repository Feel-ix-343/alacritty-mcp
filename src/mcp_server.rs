use serde_json::{json, Value};
use anyhow::{Result, anyhow};
use tracing::{error, debug};

use crate::alacritty_manager::AlacrittyManager;
use crate::types::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcError, Tool, ServerCapabilities,
    InitializeParams, SpawnParams, SendKeysParams, ScreenshotParams, NeovimContextParams
};

pub struct McpServer {
    manager: AlacrittyManager,
    initialized: bool,
}

impl McpServer {
    pub fn new(manager: AlacrittyManager) -> Self {
        Self {
            manager,
            initialized: false,
        }
    }

    pub async fn handle_request(&mut self, request_str: &str) -> Result<String> {
        debug!("Received request: {}", request_str);
        
        let request: JsonRpcRequest = serde_json::from_str(request_str)
            .map_err(|e| anyhow!("Invalid JSON-RPC request: {}", e))?;

        let response = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params, request.id).await,
            "tools/list" => self.handle_tools_list(request.id).await,
            "tools/call" => self.handle_tools_call(request.params, request.id).await,
            _ => {
                let error = JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                };
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(error),
                    id: request.id,
                }
            }
        };

        let response_str = serde_json::to_string(&response)?;
        debug!("Sending response: {}", response_str);
        Ok(response_str)
    }

    async fn handle_initialize(&mut self, params: Option<Value>, id: Option<Value>) -> JsonRpcResponse {
        match params.and_then(|p| serde_json::from_value::<InitializeParams>(p).ok()) {
            Some(_init_params) => {
                self.initialized = true;
                let capabilities = ServerCapabilities {
                    tools: self.get_tools(),
                };
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": capabilities,
                        "serverInfo": {
                            "name": "alacritty-mcp",
                            "version": "0.1.0"
                        }
                    })),
                    error: None,
                    id,
                }
            }
            None => {
                let error = JsonRpcError {
                    code: -32602,
                    message: "Invalid initialize params".to_string(),
                    data: None,
                };
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(error),
                    id,
                }
            }
        }
    }

    async fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        if !self.initialized {
            let error = JsonRpcError {
                code: -32002,
                message: "Server not initialized".to_string(),
                data: None,
            };
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(error),
                id,
            };
        }

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({
                "tools": self.get_tools()
            })),
            error: None,
            id,
        }
    }

    async fn handle_tools_call(&mut self, params: Option<Value>, id: Option<Value>) -> JsonRpcResponse {
        if !self.initialized {
            let error = JsonRpcError {
                code: -32002,
                message: "Server not initialized".to_string(),
                data: None,
            };
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(error),
                id,
            };
        }

        let call_params = match params {
            Some(p) => p,
            None => {
                let error = JsonRpcError {
                    code: -32602,
                    message: "Missing call parameters".to_string(),
                    data: None,
                };
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(error),
                    id,
                };
            }
        };

        let tool_name = match call_params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                let error = JsonRpcError {
                    code: -32602,
                    message: "Missing tool name".to_string(),
                    data: None,
                };
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(error),
                    id,
                };
            }
        };

        let arguments = call_params.get("arguments").cloned().unwrap_or(json!({}));

        let result = match tool_name {
            "list_instances" => self.handle_list_instances().await,
            "spawn_instance" => self.handle_spawn_instance(arguments).await,
            "send_keys" => self.handle_send_keys(arguments).await,
            "screenshot_instance" => self.handle_screenshot_instance(arguments).await,
            "get_neovim_context" => self.handle_get_neovim_context(arguments).await,
            _ => Err(anyhow!("Unknown tool: {}", tool_name)),
        };

        match result {
            Ok(content) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": content
                        }
                    ]
                })),
                error: None,
                id,
            },
            Err(e) => {
                error!("Tool call error: {}", e);
                let error = JsonRpcError {
                    code: -32603,
                    message: e.to_string(),
                    data: None,
                };
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(error),
                    id,
                }
            }
        }
    }

    async fn handle_list_instances(&mut self) -> Result<String> {
        let instances = self.manager.list_instances().await?;
        let json_result = serde_json::to_string_pretty(&instances)?;
        Ok(format!("Found {} Alacritty instances:\n{}", instances.len(), json_result))
    }

    async fn handle_spawn_instance(&mut self, arguments: Value) -> Result<String> {
        let params: SpawnParams = serde_json::from_value(arguments)
            .map_err(|e| anyhow!("Invalid spawn parameters: {}", e))?;
        
        let instance = self.manager.spawn_instance(params).await?;
        let json_result = serde_json::to_string_pretty(&instance)?;
        Ok(format!("Spawned new Alacritty instance:\n{}", json_result))
    }

    async fn handle_send_keys(&mut self, arguments: Value) -> Result<String> {
        let params: SendKeysParams = serde_json::from_value(arguments)
            .map_err(|e| anyhow!("Invalid send keys parameters: {}", e))?;
        
        self.manager.send_keys(params.clone()).await?;
        Ok(format!("Sent keys '{}' to instance {}", params.keys, params.instance_id))
    }

    async fn handle_screenshot_instance(&mut self, arguments: Value) -> Result<String> {
        let params: ScreenshotParams = serde_json::from_value(arguments)
            .map_err(|e| anyhow!("Invalid screenshot parameters: {}", e))?;
        
        let screenshot = self.manager.screenshot_instance(params.clone()).await?;
        let format = params.format.as_deref().unwrap_or("text");
        
        match format {
            "text" => Ok(format!("Screenshot text from instance {}:\n{}", params.instance_id, screenshot)),
            "image" => Ok(format!("Screenshot image from instance {} (base64): {}", params.instance_id, screenshot)),
            _ => Err(anyhow!("Unsupported format: {}", format)),
        }
    }

    async fn handle_get_neovim_context(&mut self, arguments: Value) -> Result<String> {
        let params: NeovimContextParams = serde_json::from_value(arguments)
            .map_err(|e| anyhow!("Invalid neovim context parameters: {}", e))?;
        
        let context = self.manager.get_neovim_context(params.clone()).await?;
        let json_result = serde_json::to_string_pretty(&context)?;
        
        Ok(format!("Neovim context for instance {}:\n{}", params.instance_id, json_result))
    }

    fn get_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "list_instances".to_string(),
                description: "List all running Alacritty terminal instances".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "spawn_instance".to_string(),
                description: "Spawn a new Alacritty terminal instance".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Command to run in the terminal"
                        },
                        "args": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Arguments for the command"
                        },
                        "working_directory": {
                            "type": "string",
                            "description": "Working directory for the terminal"
                        },
                        "title": {
                            "type": "string",
                            "description": "Title for the terminal window"
                        }
                    },
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "send_keys".to_string(),
                description: "Send key commands to an Alacritty instance".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "instance_id": {
                            "type": "string",
                            "description": "ID of the Alacritty instance"
                        },
                        "keys": {
                            "type": "string",
                            "description": "Keys to send (xdotool format, e.g., 'ctrl+c', 'Return', 'Hello')"
                        }
                    },
                    "required": ["instance_id", "keys"],
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "screenshot_instance".to_string(),
                description: "Take a screenshot of an Alacritty instance".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "instance_id": {
                            "type": "string",
                            "description": "ID of the Alacritty instance"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["text", "image"],
                            "description": "Format of the screenshot: 'text' for terminal text content, 'image' for visual screenshot",
                            "default": "text"
                        }
                    },
                    "required": ["instance_id"],
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "get_neovim_context".to_string(),
                description: "Extract comprehensive Neovim context including cursor position, diagnostics, open buffers, and LSP status".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "instance_id": {
                            "type": "string",
                            "description": "ID of the Alacritty instance running Neovim"
                        },
                        "include_diagnostics": {
                            "type": "boolean",
                            "description": "Include LSP diagnostics in the context",
                            "default": true
                        },
                        "include_buffers": {
                            "type": "boolean", 
                            "description": "Include list of open buffers",
                            "default": true
                        },
                        "context_lines": {
                            "type": "number",
                            "description": "Number of lines around cursor to include",
                            "default": 5,
                            "minimum": 0,
                            "maximum": 50
                        }
                    },
                    "required": ["instance_id"],
                    "additionalProperties": false
                }),
            },
        ]
    }
}