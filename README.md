# Alacritty MCP Server

A Model Context Protocol (MCP) server for controlling Alacritty terminal instances.

## Use Cases

This MCP server enables Claude to interact with terminal applications in powerful ways:

### 🔍 **Context Reading & Application Monitoring**
- **Read Neovim context** - Extract current file, cursor position, LSP diagnostics, open buffers, and vim mode
- **LSP integration** - Access error messages, warnings, and code intelligence from language servers
- **Monitor build processes** - Watch compilation output and test results in real-time  
- **Debug application state** - Inspect running processes and system information
- **Track log files** - Monitor application logs and system events

### 🧪 **Interactive Testing & Development**
- **Test TUI applications** - Validate interactive terminal interfaces Claude has built
- **Run development workflows** - Execute build scripts, tests, and deployment commands
- **Interactive debugging** - Step through code execution and inspect variables
- **Prototype validation** - Quickly test CLI tools and terminal-based applications

### 🚀 **Automation & Orchestration**
- **Multi-terminal workflows** - Coordinate complex development environments
- **Parallel task execution** - Run multiple processes simultaneously across terminals
- **Environment setup** - Automate development environment configuration
- **CI/CD integration** - Execute and monitor build pipelines

### 📊 **System Administration**
- **Server monitoring** - Track system resources and process health
- **Remote task execution** - Run administrative commands across multiple sessions
- **Log aggregation** - Collect and analyze output from various system components

## Features

- **List Instances**: Discover all running Alacritty terminal instances
- **Spawn Instances**: Create new Alacritty terminals with custom configurations
- **Send Keys**: Send keyboard commands to specific terminal instances
- **Screenshot**: Capture terminal content as text or visual screenshots
- **Neovim Context**: Extract comprehensive editing context from Neovim instances

## Requirements

- Rust 1.70+
- Alacritty terminal emulator
- X11 environment (Linux)
- System utilities: `xdotool`, `xclip`, `imagemagick` (for screenshots)

## Installation

```bash
cargo build --release
```

## Usage

The server communicates via JSON-RPC over stdin/stdout:

```bash
./target/release/alacritty-mcp
```

## MCP Tools

### list_instances
Lists all running Alacritty instances.

**Parameters:** None

**Returns:** Array of AlacrittyInstance objects with details like PID, window ID, title, and command.

### spawn_instance
Creates a new Alacritty terminal instance.

**Parameters:**
- `command` (optional): Command to run in the terminal
- `args` (optional): Arguments for the command
- `working_directory` (optional): Working directory for the terminal
- `title` (optional): Title for the terminal window

**Returns:** Details of the newly created instance.

### send_keys
Sends keyboard input to a specific Alacritty instance.

**Parameters:**
- `instance_id` (required): ID of the target instance
- `keys` (required): Keys to send (xdotool format, e.g., 'ctrl+c', 'Return', 'Hello')

**Returns:** Confirmation of keys sent.

### screenshot_instance
Captures content from an Alacritty instance.

**Parameters:**
- `instance_id` (required): ID of the target instance
- `format` (optional): 'text' for terminal text content, 'image' for visual screenshot (default: 'text')

**Returns:** Screenshot content in the requested format.

### get_neovim_context
Extracts comprehensive context from a Neovim instance running in an Alacritty terminal.

**Parameters:**
- `instance_id` (required): ID of the Alacritty instance running Neovim
- `include_diagnostics` (optional): Include LSP diagnostics (default: true)
- `include_buffers` (optional): Include list of open buffers (default: true)
- `context_lines` (optional): Number of lines around cursor to include (default: 5)

**Returns:** Structured Neovim context including:
- Current file and cursor position
- LSP diagnostics (errors, warnings, hints)
- Open buffers and their status
- Vim mode and working directory
- Active LSP clients and their status
- Surrounding code context

## Example JSON-RPC Calls

### Initialize
```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {
      "name": "claude",
      "version": "1.0.0"
    }
  },
  "id": 1
}
```

### Spawn Instance
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "spawn_instance",
    "arguments": {
      "title": "Development Terminal",
      "working_directory": "/home/user/project",
      "command": "bash"
    }
  },
  "id": 2
}
```

### Send Keys
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "send_keys",
    "arguments": {
      "instance_id": "uuid-here",
      "keys": "ls -la Return"
    }
  },
  "id": 3
}
```

### Get Neovim Context
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "get_neovim_context",
    "arguments": {
      "instance_id": "uuid-here",
      "include_diagnostics": true,
      "include_buffers": true,
      "context_lines": 10
    }
  },
  "id": 4
}
```

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "text": "Neovim context for instance uuid-here:\n{\n  \"instance_info\": {\n    \"pid\": 12345,\n    \"socket_path\": \"/tmp/nvim.12345.0\",\n    \"version\": \"NVIM v0.11.2\",\n    \"config_path\": \"/home/user/.config/nvim\"\n  },\n  \"current_buffer\": {\n    \"file_path\": \"/home/user/project/src/main.rs\",\n    \"file_type\": \"rust\",\n    \"is_modified\": true,\n    \"line_count\": 150,\n    \"surrounding_context\": {\n      \"lines_before\": [\"fn main() {\", \"    let config = Config::new();\"],\n      \"current_line\": \"    let result = process_data(config);\",\n      \"lines_after\": [\"    println!(\\\"Result: {:?}\\\", result);\", \"}\"]\n    }\n  },\n  \"diagnostics\": [\n    {\n      \"file_path\": \"/home/user/project/src/main.rs\",\n      \"line\": 42,\n      \"column\": 15,\n      \"severity\": \"Error\",\n      \"message\": \"cannot find function `process_data` in this scope\",\n      \"source\": \"rust-analyzer\"\n    }\n  ],\n  \"cursor_position\": {\n    \"line\": 42,\n    \"column\": 28,\n    \"line_content\": \"    let result = process_data(config);\"\n  },\n  \"vim_mode\": \"n\",\n  \"lsp_status\": {\n    \"active_clients\": [\n      {\n        \"name\": \"rust-analyzer\",\n        \"file_types\": [\"rust\"],\n        \"status\": \"active\"\n      }\n    ],\n    \"diagnostics_count\": {\n      \"errors\": 1,\n      \"warnings\": 0,\n      \"info\": 0,\n      \"hints\": 2\n    }\n  }\n}",
        "type": "text"
      }
    ]
  },
  "error": null,
  "id": 4
}
```

## Testing

The project includes comprehensive test coverage:

### Unit Tests
Test core functionality and data structures:
```bash
cargo test --test unit_tests
```

### Integration Tests  
Test MCP protocol compliance and error handling:
```bash
cargo test --test integration_tests
```

### Functional Tests
Test real Alacritty interaction (requires X11 and Alacritty installed):
```bash
cargo test --test functional_tests
```

**Functional tests include:**
- ✅ **Real terminal spawning** - Actually opens Alacritty windows
- ✅ **Lifecycle management** - Spawn multiple terminals and track them
- ✅ **Custom commands** - Launch terminals with specific commands
- ✅ **Process cleanup** - Proper terminal termination
- ⚠️ **Key sending** - Requires `xdotool` (skipped if not available)
- ⚠️ **Screenshots** - Requires `xclip` (skipped if not available)

### Neovim Integration Tests
Test Neovim context extraction (requires Neovim installed):
```bash
cargo test --test neovim_integration_tests
```

**Neovim tests include:**
- ✅ **Neovim detection** - Identify Neovim instances in terminals
- ✅ **Context extraction** - Extract editing state and diagnostics
- ✅ **Real Neovim spawning** - Launch and communicate with Neovim
- ✅ **Pattern recognition** - Detect Neovim UI elements
- ✅ **LSP integration** - Access language server diagnostics

### Run All Tests
```bash
cargo test
```

**Test Results:** 29 total tests
- 11 unit tests ✅
- 8 integration tests ✅  
- 5 functional tests ✅
- 5 Neovim integration tests ✅

Note: Some functional tests require X11 environment and system tools, and may be skipped in headless CI environments.

## Architecture

- `AlacrittyManager`: Core logic for managing terminal instances
- `McpServer`: JSON-RPC server implementation  
- `NeovimContextExtractor`: Neovim-specific context extraction and LSP integration
- `types`: Data structures and type definitions

## Limitations

- Currently X11-only (no Wayland support)
- Requires external system utilities for advanced features
- Terminal text extraction relies on clipboard operations