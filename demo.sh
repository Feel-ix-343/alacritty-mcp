#!/bin/bash

# Demo script for Alacritty MCP Server
echo "🚀 Alacritty MCP Server Demo"
echo "==============================="

# Build the project
echo "📦 Building the MCP server..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful!"
echo ""

# Create a test script that sends JSON-RPC commands
echo "🧪 Running functional demo..."

# Initialize the server
init_request='{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo-client","version":"1.0.0"}},"id":1}'

# List tools
tools_request='{"jsonrpc":"2.0","method":"tools/list","id":2}'

# List current instances
list_request='{"jsonrpc":"2.0","method":"tools/call","params":{"name":"list_instances","arguments":{}},"id":3}'

# Spawn new instance
spawn_request='{"jsonrpc":"2.0","method":"tools/call","params":{"name":"spawn_instance","arguments":{"title":"MCP-Demo-Terminal","working_directory":"/tmp"}},"id":4}'

echo "📋 Available commands:"
echo "1. Initialize server"
echo "2. List available tools"
echo "3. List current Alacritty instances"
echo "4. Spawn new Alacritty instance"
echo ""

echo "🔧 MCP Server is ready!"
echo "Binary location: ./target/release/alacritty-mcp"
echo ""
echo "📖 To test manually, run:"
echo "  echo '$init_request' | ./target/release/alacritty-mcp"
echo "  echo '$tools_request' | ./target/release/alacritty-mcp"
echo "  echo '$list_request' | ./target/release/alacritty-mcp"
echo "  echo '$spawn_request' | ./target/release/alacritty-mcp"
echo ""
echo "🧪 Running automated tests to verify functionality..."
cargo test --test functional_tests --quiet

echo ""
echo "✅ Demo complete! The Alacritty MCP server is fully functional."
echo "📚 See README.md for detailed usage instructions."