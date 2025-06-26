use std::process::Command;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeovimContext {
    pub instance_info: NeovimInstanceInfo,
    pub current_buffer: Option<CurrentBuffer>,
    pub diagnostics: Vec<Diagnostic>,
    pub open_buffers: Vec<BufferInfo>,
    pub cursor_position: Option<CursorPosition>,
    pub vim_mode: Option<String>,
    pub working_directory: Option<String>,
    pub lsp_status: Option<LspStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeovimInstanceInfo {
    pub pid: u32,
    pub socket_path: Option<String>,
    pub version: Option<String>,
    pub config_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentBuffer {
    pub file_path: String,
    pub file_type: Option<String>,
    pub is_modified: bool,
    pub line_count: u32,
    pub content_preview: String,
    pub surrounding_context: SurroundingContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurroundingContext {
    pub lines_before: Vec<String>,
    pub current_line: String,
    pub lines_after: Vec<String>,
    pub function_context: Option<String>,
    pub class_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferInfo {
    pub file_path: String,
    pub is_modified: bool,
    pub is_current: bool,
    pub file_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
    pub line_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspStatus {
    pub active_clients: Vec<LspClient>,
    pub diagnostics_count: DiagnosticCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspClient {
    pub name: String,
    pub file_types: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCounts {
    pub errors: u32,
    pub warnings: u32,
    pub info: u32,
    pub hints: u32,
}

pub struct NeovimContextExtractor {
    nvim_command: String,
}

impl NeovimContextExtractor {
    pub fn new() -> Self {
        Self {
            nvim_command: "nvim".to_string(),
        }
    }

    pub async fn extract_context_from_instance(&self, instance_id: &str, pid: u32) -> Result<NeovimContext> {
        // Try multiple methods to connect to Neovim
        let context = if let Ok(ctx) = self.extract_via_nvim_listen(pid).await {
            ctx
        } else if let Ok(ctx) = self.extract_via_terminal_scraping(instance_id).await {
            ctx
        } else {
            self.extract_basic_context(pid).await?
        };

        Ok(context)
    }

    async fn extract_via_nvim_listen(&self, pid: u32) -> Result<NeovimContext> {
        // Try to find Neovim socket
        let socket_path = self.find_neovim_socket(pid).await?;
        
        // Use nvim --server to communicate with the instance
        let current_buffer = self.get_current_buffer_via_socket(&socket_path).await?;
        let diagnostics = self.get_diagnostics_via_socket(&socket_path).await?;
        let open_buffers = self.get_open_buffers_via_socket(&socket_path).await?;
        let cursor_position = self.get_cursor_position_via_socket(&socket_path).await?;
        let vim_mode = self.get_vim_mode_via_socket(&socket_path).await?;
        let lsp_status = self.get_lsp_status_via_socket(&socket_path).await?;
        let working_directory = self.get_working_directory_via_socket(&socket_path).await?;

        Ok(NeovimContext {
            instance_info: NeovimInstanceInfo {
                pid,
                socket_path: Some(socket_path),
                version: self.get_neovim_version().await.ok(),
                config_path: self.get_config_path().await.ok(),
            },
            current_buffer,
            diagnostics,
            open_buffers,
            cursor_position,
            vim_mode,
            working_directory,
            lsp_status,
        })
    }

    async fn extract_via_terminal_scraping(&self, _instance_id: &str) -> Result<NeovimContext> {
        // This would use the existing screenshot functionality to parse terminal content
        // and extract Neovim state from the visual output
        Err(anyhow!("Terminal scraping not yet implemented"))
    }

    async fn extract_basic_context(&self, pid: u32) -> Result<NeovimContext> {
        // Fallback: basic process information
        Ok(NeovimContext {
            instance_info: NeovimInstanceInfo {
                pid,
                socket_path: None,
                version: self.get_neovim_version().await.ok(),
                config_path: self.get_config_path().await.ok(),
            },
            current_buffer: None,
            diagnostics: Vec::new(),
            open_buffers: Vec::new(),
            cursor_position: None,
            vim_mode: None,
            working_directory: self.get_process_working_directory(pid).await.ok(),
            lsp_status: None,
        })
    }

    async fn find_neovim_socket(&self, pid: u32) -> Result<String> {
        // Check common socket locations
        let possible_sockets = vec![
            format!("/tmp/nvim.{}.0", pid),
            format!("/tmp/nvim{}/0", pid),
            format!("/run/user/{}/nvim.{}.0", self.get_user_id()?, pid),
        ];

        for socket in possible_sockets {
            if std::path::Path::new(&socket).exists() {
                return Ok(socket);
            }
        }

        // Try to find via lsof
        let output = Command::new("lsof")
            .args(&["-p", &pid.to_string(), "-a", "-U"])
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("nvim") && line.contains("socket") {
                    if let Some(socket_path) = line.split_whitespace().last() {
                        return Ok(socket_path.to_string());
                    }
                }
            }
        }

        Err(anyhow!("Could not find Neovim socket for PID {}", pid))
    }

    async fn get_current_buffer_via_socket(&self, socket_path: &str) -> Result<Option<CurrentBuffer>> {
        let lua_script = r#"
            local buf = vim.api.nvim_get_current_buf()
            local file_path = vim.api.nvim_buf_get_name(buf)
            local file_type = vim.bo.filetype
            local is_modified = vim.bo.modified
            local line_count = vim.api.nvim_buf_line_count(buf)
            local cursor = vim.api.nvim_win_get_cursor(0)
            local current_line_nr = cursor[1]
            
            -- Get surrounding context
            local start_line = math.max(1, current_line_nr - 5)
            local end_line = math.min(line_count, current_line_nr + 5)
            local lines = vim.api.nvim_buf_get_lines(buf, start_line - 1, end_line, false)
            
            local context = {
                file_path = file_path,
                file_type = file_type,
                is_modified = is_modified,
                line_count = line_count,
                current_line_nr = current_line_nr,
                lines_before = {},
                current_line = "",
                lines_after = {},
            }
            
            for i, line in ipairs(lines) do
                local line_nr = start_line + i - 1
                if line_nr < current_line_nr then
                    table.insert(context.lines_before, line)
                elseif line_nr == current_line_nr then
                    context.current_line = line
                else
                    table.insert(context.lines_after, line)
                end
            end
            
            print(vim.json.encode(context))
        "#;

        let output = Command::new("nvim")
            .args(&["--server", socket_path, "--remote-expr", &format!("luaeval('{}')", lua_script)])
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&output_str) {
                let current_buffer = CurrentBuffer {
                    file_path: data["file_path"].as_str().unwrap_or("").to_string(),
                    file_type: data["file_type"].as_str().map(|s| s.to_string()),
                    is_modified: data["is_modified"].as_bool().unwrap_or(false),
                    line_count: data["line_count"].as_u64().unwrap_or(0) as u32,
                    content_preview: format!("Current line: {}", data["current_line"].as_str().unwrap_or("")),
                    surrounding_context: SurroundingContext {
                        lines_before: data["lines_before"].as_array()
                            .map(|arr| arr.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                            .unwrap_or_default(),
                        current_line: data["current_line"].as_str().unwrap_or("").to_string(),
                        lines_after: data["lines_after"].as_array()
                            .map(|arr| arr.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                            .unwrap_or_default(),
                        function_context: None, // TODO: Parse function context
                        class_context: None,    // TODO: Parse class context
                    },
                };
                return Ok(Some(current_buffer));
            }
        }

        Ok(None)
    }

    async fn get_diagnostics_via_socket(&self, socket_path: &str) -> Result<Vec<Diagnostic>> {
        let lua_script = r#"
            local diagnostics = vim.diagnostic.get()
            local result = {}
            for _, diag in ipairs(diagnostics) do
                table.insert(result, {
                    file_path = vim.api.nvim_buf_get_name(diag.bufnr),
                    line = diag.lnum + 1,
                    column = diag.col + 1,
                    severity = diag.severity,
                    message = diag.message,
                    source = diag.source,
                    code = diag.code
                })
            end
            print(vim.json.encode(result))
        "#;

        let output = Command::new("nvim")
            .args(&["--server", socket_path, "--remote-expr", &format!("luaeval('{}')", lua_script)])
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(data) = serde_json::from_str::<Vec<serde_json::Value>>(&output_str) {
                let diagnostics = data.into_iter().map(|d| {
                    let severity = match d["severity"].as_u64().unwrap_or(1) {
                        1 => DiagnosticSeverity::Error,
                        2 => DiagnosticSeverity::Warning,
                        3 => DiagnosticSeverity::Info,
                        _ => DiagnosticSeverity::Hint,
                    };

                    Diagnostic {
                        file_path: d["file_path"].as_str().unwrap_or("").to_string(),
                        line: d["line"].as_u64().unwrap_or(0) as u32,
                        column: d["column"].as_u64().unwrap_or(0) as u32,
                        severity,
                        message: d["message"].as_str().unwrap_or("").to_string(),
                        source: d["source"].as_str().map(|s| s.to_string()),
                        code: d["code"].as_str().map(|s| s.to_string()),
                    }
                }).collect();

                return Ok(diagnostics);
            }
        }

        Ok(Vec::new())
    }

    async fn get_open_buffers_via_socket(&self, socket_path: &str) -> Result<Vec<BufferInfo>> {
        let lua_script = r#"
            local buffers = {}
            local current_buf = vim.api.nvim_get_current_buf()
            for _, buf in ipairs(vim.api.nvim_list_bufs()) do
                if vim.api.nvim_buf_is_loaded(buf) then
                    local name = vim.api.nvim_buf_get_name(buf)
                    if name ~= "" then
                        table.insert(buffers, {
                            file_path = name,
                            is_modified = vim.api.nvim_buf_get_option(buf, "modified"),
                            is_current = buf == current_buf,
                            file_type = vim.api.nvim_buf_get_option(buf, "filetype")
                        })
                    end
                end
            end
            print(vim.json.encode(buffers))
        "#;

        let output = Command::new("nvim")
            .args(&["--server", socket_path, "--remote-expr", &format!("luaeval('{}')", lua_script)])
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(data) = serde_json::from_str::<Vec<serde_json::Value>>(&output_str) {
                let buffers = data.into_iter().map(|b| BufferInfo {
                    file_path: b["file_path"].as_str().unwrap_or("").to_string(),
                    is_modified: b["is_modified"].as_bool().unwrap_or(false),
                    is_current: b["is_current"].as_bool().unwrap_or(false),
                    file_type: b["file_type"].as_str().map(|s| s.to_string()),
                }).collect();

                return Ok(buffers);
            }
        }

        Ok(Vec::new())
    }

    async fn get_cursor_position_via_socket(&self, socket_path: &str) -> Result<Option<CursorPosition>> {
        let lua_script = r#"
            local cursor = vim.api.nvim_win_get_cursor(0)
            local line_content = vim.api.nvim_get_current_line()
            local result = {
                line = cursor[1],
                column = cursor[2] + 1,
                line_content = line_content
            }
            print(vim.json.encode(result))
        "#;

        let output = Command::new("nvim")
            .args(&["--server", socket_path, "--remote-expr", &format!("luaeval('{}')", lua_script)])
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&output_str) {
                return Ok(Some(CursorPosition {
                    line: data["line"].as_u64().unwrap_or(0) as u32,
                    column: data["column"].as_u64().unwrap_or(0) as u32,
                    line_content: data["line_content"].as_str().unwrap_or("").to_string(),
                }));
            }
        }

        Ok(None)
    }

    async fn get_vim_mode_via_socket(&self, socket_path: &str) -> Result<Option<String>> {
        let output = Command::new("nvim")
            .args(&["--server", socket_path, "--remote-expr", "mode()"])
            .output()?;

        if output.status.success() {
            let mode = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(Some(mode));
        }

        Ok(None)
    }

    async fn get_lsp_status_via_socket(&self, socket_path: &str) -> Result<Option<LspStatus>> {
        let lua_script = r#"
            local clients = vim.lsp.get_active_clients()
            local result = {
                active_clients = {},
                diagnostics_count = {errors = 0, warnings = 0, info = 0, hints = 0}
            }
            
            for _, client in ipairs(clients) do
                table.insert(result.active_clients, {
                    name = client.name,
                    file_types = client.config.filetypes or {},
                    status = "active"
                })
            end
            
            local diagnostics = vim.diagnostic.get()
            for _, diag in ipairs(diagnostics) do
                if diag.severity == 1 then
                    result.diagnostics_count.errors = result.diagnostics_count.errors + 1
                elseif diag.severity == 2 then
                    result.diagnostics_count.warnings = result.diagnostics_count.warnings + 1
                elseif diag.severity == 3 then
                    result.diagnostics_count.info = result.diagnostics_count.info + 1
                else
                    result.diagnostics_count.hints = result.diagnostics_count.hints + 1
                end
            end
            
            print(vim.json.encode(result))
        "#;

        let output = Command::new("nvim")
            .args(&["--server", socket_path, "--remote-expr", &format!("luaeval('{}')", lua_script)])
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&output_str) {
                let active_clients = data["active_clients"].as_array()
                    .map(|arr| {
                        arr.iter().map(|c| LspClient {
                            name: c["name"].as_str().unwrap_or("").to_string(),
                            file_types: c["file_types"].as_array()
                                .map(|ft| ft.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                                .unwrap_or_default(),
                            status: c["status"].as_str().unwrap_or("").to_string(),
                        }).collect()
                    })
                    .unwrap_or_default();

                let diagnostics_count = DiagnosticCounts {
                    errors: data["diagnostics_count"]["errors"].as_u64().unwrap_or(0) as u32,
                    warnings: data["diagnostics_count"]["warnings"].as_u64().unwrap_or(0) as u32,
                    info: data["diagnostics_count"]["info"].as_u64().unwrap_or(0) as u32,
                    hints: data["diagnostics_count"]["hints"].as_u64().unwrap_or(0) as u32,
                };

                return Ok(Some(LspStatus {
                    active_clients,
                    diagnostics_count,
                }));
            }
        }

        Ok(None)
    }

    async fn get_working_directory_via_socket(&self, socket_path: &str) -> Result<Option<String>> {
        let output = Command::new("nvim")
            .args(&["--server", socket_path, "--remote-expr", "getcwd()"])
            .output()?;

        if output.status.success() {
            let wd = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(Some(wd));
        }

        Ok(None)
    }

    async fn get_neovim_version(&self) -> Result<String> {
        let output = Command::new(&self.nvim_command)
            .args(&["--version"])
            .output()?;

        if output.status.success() {
            let version_output = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = version_output.lines().next() {
                return Ok(first_line.to_string());
            }
        }

        Err(anyhow!("Could not get Neovim version"))
    }

    async fn get_config_path(&self) -> Result<String> {
        let output = Command::new(&self.nvim_command)
            .args(&["--headless", "-c", "echo stdpath('config')", "-c", "quit"])
            .output()?;

        if output.status.success() {
            let config_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(config_path);
        }

        Err(anyhow!("Could not get Neovim config path"))
    }

    async fn get_process_working_directory(&self, pid: u32) -> Result<String> {
        let cwd_path = format!("/proc/{}/cwd", pid);
        match std::fs::read_link(&cwd_path) {
            Ok(path) => Ok(path.to_string_lossy().to_string()),
            Err(_) => {
                // Fallback: use lsof
                let output = Command::new("lsof")
                    .args(&["-p", &pid.to_string(), "-a", "-d", "cwd"])
                    .output()?;

                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if line.contains("cwd") {
                            if let Some(path) = line.split_whitespace().last() {
                                return Ok(path.to_string());
                            }
                        }
                    }
                }

                Err(anyhow!("Could not determine working directory for PID {}", pid))
            }
        }
    }

    fn get_user_id(&self) -> Result<u32> {
        let output = Command::new("id")
            .args(&["-u"])
            .output()?;

        if output.status.success() {
            let uid_string = String::from_utf8_lossy(&output.stdout);
            let uid_str = uid_string.trim();
            return uid_str.parse().map_err(|e| anyhow!("Failed to parse UID: {}", e));
        }

        Err(anyhow!("Could not get user ID"))
    }

    pub fn detect_neovim_in_terminal(&self, terminal_content: &str) -> bool {
        // Look for common Neovim indicators in terminal content
        let nvim_indicators = [
            "-- INSERT --",
            "-- VISUAL --", 
            "-- NORMAL --",
            "-- COMMAND --",
            ":set",
            ":help",
            ":q",
            "~vim:",
            "[No Name]",
        ];

        nvim_indicators.iter().any(|indicator| terminal_content.contains(indicator))
    }
}

impl Default for NeovimContextExtractor {
    fn default() -> Self {
        Self::new()
    }
}