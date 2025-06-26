# Alacritty MCP Server

A Model Context Protocol (MCP) server for controlling Alacritty terminal instances.

## Features

- **List Instances**: Discover all running Alacritty terminal instances
- **Spawn Instances**: Create new Alacritty terminals with custom configurations
- **Send Keys**: Send keyboard commands to specific terminal instances
- **Screenshot**: Capture terminal content as text or visual screenshots

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

### Run All Tests
```bash
cargo test
```

**Test Results:** 24 total tests
- 11 unit tests ✅
- 8 integration tests ✅  
- 5 functional tests ✅

Note: Some functional tests require X11 environment and system tools, and may be skipped in headless CI environments.

## Architecture

- `AlacrittyManager`: Core logic for managing terminal instances
- `McpServer`: JSON-RPC server implementation
- `types`: Data structures and type definitions

## Limitations

- Currently X11-only (no Wayland support)
- Requires external system utilities for advanced features
- Terminal text extraction relies on clipboard operations