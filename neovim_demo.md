# Neovim Integration Demo

This demonstrates how Claude can read and understand your Neovim editing context through the Alacritty MCP server.

## Example Workflow: Code Assistance with Context

### 1. Spawn Neovim in a new terminal
```bash
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"spawn_instance","arguments":{"title":"Neovim-Coding","command":"nvim","args":["src/main.rs"],"working_directory":"/home/user/project"}},"id":1}' | ./target/release/alacritty-mcp
```

### 2. Extract current editing context
```bash
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_neovim_context","arguments":{"instance_id":"your-instance-id","include_diagnostics":true,"context_lines":15}},"id":2}' | ./target/release/alacritty-mcp
```

### 3. Claude receives structured context like:
```json
{
  "current_buffer": {
    "file_path": "/home/user/project/src/main.rs",
    "file_type": "rust",
    "is_modified": true,
    "surrounding_context": {
      "current_line": "    let result = process_data(config);",
      "lines_before": ["fn main() {", "    let config = Config::new();"],
      "lines_after": ["    println!(\"Result: {:?}\", result);", "}"]
    }
  },
  "diagnostics": [
    {
      "line": 42,
      "message": "cannot find function `process_data` in this scope",
      "severity": "Error",
      "source": "rust-analyzer"
    }
  ],
  "cursor_position": {
    "line": 42,
    "column": 28
  },
  "lsp_status": {
    "active_clients": [{"name": "rust-analyzer", "status": "active"}],
    "diagnostics_count": {"errors": 1, "warnings": 0}
  }
}
```

## What Claude Can Do With This Context

### 🔍 **Understand Your Current Situation**
- **File location**: You're editing `src/main.rs` 
- **Current problem**: Missing `process_data` function causing a compilation error
- **Cursor position**: Line 42, column 28 - right where the error is
- **LSP feedback**: rust-analyzer is active and reporting the issue

### 💡 **Provide Contextual Assistance**
Claude can now give you precise help:

> "I see you're on line 42 of `src/main.rs` with a rust-analyzer error about the missing `process_data` function. Based on your code context, it looks like you're trying to process a `Config` object. Here are a few solutions:
> 
> 1. **Define the function** above line 42:
> ```rust
> fn process_data(config: Config) -> SomeResultType {
>     // Implementation here
> }
> ```
> 
> 2. **Import it** if it exists in another module:
> ```rust
> use crate::data_processor::process_data;
> ```
> 
> Would you like me to help you implement the `process_data` function based on how you're using it?"

### 🚀 **Intelligent Code Completion**
- **Context-aware suggestions** based on surrounding code
- **Error-driven development** - fix issues as they appear in diagnostics
- **Multi-buffer awareness** - understand relationships between open files
- **LSP integration** - leverage your existing language server setup

## Advanced Use Cases

### Real-time Code Review
```bash
# Get diagnostics across all buffers
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_neovim_context","arguments":{"instance_id":"uuid","include_diagnostics":true,"include_buffers":true}},"id":3}' | ./target/release/alacritty-mcp
```

### Debugging Session Support
```bash
# Extract context while debugging
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_neovim_context","arguments":{"instance_id":"uuid","context_lines":20}},"id":4}' | ./target/release/alacritty-mcp
```

### Multi-file Project Understanding
The context includes all open buffers, so Claude can understand:
- Which files you're working on simultaneously
- Cross-file dependencies and relationships
- Project structure and organization

## Benefits for Development Workflow

1. **No more copy-pasting code** - Claude sees exactly what you're working on
2. **Precise error fixing** - Direct access to LSP diagnostics and error locations
3. **Contextual suggestions** - Recommendations based on your actual code
4. **Efficient debugging** - Understanding of your current debugging context
5. **Project awareness** - Knowledge of your file structure and open buffers

This transforms Claude from a general coding assistant into a **context-aware pair programmer** that understands exactly what you're working on! 🎉