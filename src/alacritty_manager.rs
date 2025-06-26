use std::collections::HashMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Result, anyhow};
use uuid::Uuid;

use crate::types::{AlacrittyInstance, SpawnParams, SendKeysParams, ScreenshotParams};

pub struct AlacrittyManager {
    instances: HashMap<String, AlacrittyInstance>,
}

impl AlacrittyManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    pub async fn list_instances(&mut self) -> Result<Vec<AlacrittyInstance>> {
        self.refresh_instances().await?;
        Ok(self.instances.values().cloned().collect())
    }

    pub async fn spawn_instance(&mut self, params: SpawnParams) -> Result<AlacrittyInstance> {
        let instance_id = Uuid::new_v4().to_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut cmd = Command::new("alacritty");
        
        // Set title if provided
        if let Some(title) = &params.title {
            cmd.args(&["--title", title]);
        } else {
            cmd.args(&["--title", &format!("alacritty-mcp-{}", &instance_id[..8])]);
        }

        // Set working directory if provided
        if let Some(wd) = &params.working_directory {
            cmd.args(&["--working-directory", wd]);
        }

        // Set command if provided
        if let Some(command) = &params.command {
            cmd.args(&["--command"]);
            cmd.arg(command);
            if let Some(args) = &params.args {
                cmd.args(args);
            }
        }

        // Add class for identification
        cmd.args(&["--class", &format!("alacritty-mcp-{}", instance_id)]);

        let child = cmd.spawn()?;
        let pid = child.id();

        let title = params.title.unwrap_or_else(|| format!("alacritty-mcp-{}", &instance_id[..8]));
        let command_str = params.command.unwrap_or_else(|| "shell".to_string());

        let instance = AlacrittyInstance {
            id: instance_id.clone(),
            pid,
            window_id: None,
            title,
            command: command_str,
            created_at: timestamp,
        };

        self.instances.insert(instance_id.clone(), instance.clone());

        // Give the window time to appear
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Try to get the window ID
        if let Ok(window_id) = self.get_window_id_for_instance(&instance_id).await {
            if let Some(inst) = self.instances.get_mut(&instance_id) {
                inst.window_id = Some(window_id);
            }
        }

        Ok(instance)
    }

    pub async fn send_keys(&self, params: SendKeysParams) -> Result<()> {
        let instance = self.instances.get(&params.instance_id)
            .ok_or_else(|| anyhow!("Instance not found: {}", params.instance_id))?;

        if let Some(window_id) = instance.window_id {
            // Use xdotool to send keys to the specific window
            let output = Command::new("xdotool")
                .args(&["key", "--window", &window_id.to_string()])
                .arg(&params.keys)
                .output()?;

            if !output.status.success() {
                return Err(anyhow!("Failed to send keys: {}", 
                    String::from_utf8_lossy(&output.stderr)));
            }
        } else {
            // Fallback: try to find window and send keys
            let window_id = self.get_window_id_for_instance(&params.instance_id).await?;
            let output = Command::new("xdotool")
                .args(&["key", "--window", &window_id.to_string()])
                .arg(&params.keys)
                .output()?;

            if !output.status.success() {
                return Err(anyhow!("Failed to send keys: {}", 
                    String::from_utf8_lossy(&output.stderr)));
            }
        }

        Ok(())
    }

    pub async fn screenshot_instance(&self, params: ScreenshotParams) -> Result<String> {
        let instance = self.instances.get(&params.instance_id)
            .ok_or_else(|| anyhow!("Instance not found: {}", params.instance_id))?;

        let window_id = if let Some(wid) = instance.window_id {
            wid
        } else {
            self.get_window_id_for_instance(&params.instance_id).await?
        };

        let format = params.format.as_deref().unwrap_or("text");

        match format {
            "text" => self.screenshot_text(window_id).await,
            "image" => self.screenshot_image(window_id).await,
            _ => Err(anyhow!("Unsupported format: {}", format)),
        }
    }

    async fn screenshot_text(&self, window_id: u32) -> Result<String> {
        // Use xdotool to get text content from the terminal
        let output = Command::new("xdotool")
            .args(&["getwindowgeometry", &window_id.to_string()])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get window geometry"));
        }

        // Get the window content using xwininfo and xwd
        let output = Command::new("xwininfo")
            .args(&["-id", &window_id.to_string(), "-tree"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get window info"));
        }

        // For text extraction, we'll use a different approach
        // Copy all text from the terminal using xsel or xclip
        let _select_output = Command::new("xdotool")
            .args(&["windowactivate", &window_id.to_string()])
            .output()?;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Select all text
        let _select_output = Command::new("xdotool")
            .args(&["key", "--window", &window_id.to_string(), "ctrl+shift+a"])
            .output()?;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Copy to clipboard
        let _copy_output = Command::new("xdotool")
            .args(&["key", "--window", &window_id.to_string(), "ctrl+shift+c"])
            .output()?;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get clipboard content
        let clipboard_output = Command::new("xclip")
            .args(&["-o", "-selection", "clipboard"])
            .output()?;

        if clipboard_output.status.success() {
            Ok(String::from_utf8_lossy(&clipboard_output.stdout).to_string())
        } else {
            Err(anyhow!("Failed to get clipboard content"))
        }
    }

    async fn screenshot_image(&self, window_id: u32) -> Result<String> {
        // Take a screenshot of the window using imagemagick
        let temp_file = format!("/tmp/alacritty_screenshot_{}.png", window_id);
        
        let output = Command::new("import")
            .args(&["-window", &window_id.to_string(), &temp_file])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to take screenshot: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }

        // Read the file and encode as base64
        let image_data = std::fs::read(&temp_file)?;
        let base64_data = base64::encode(&image_data);
        
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        Ok(format!("data:image/png;base64,{}", base64_data))
    }

    async fn refresh_instances(&mut self) -> Result<()> {
        // Get all alacritty processes
        let output = Command::new("pgrep")
            .args(&["-f", "alacritty"])
            .output()?;

        if !output.status.success() {
            // No alacritty processes running
            self.instances.clear();
            return Ok(());
        }

        let pids_str = String::from_utf8_lossy(&output.stdout);
        let running_pids: Vec<u32> = pids_str
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect();

        // Remove instances that are no longer running
        self.instances.retain(|_, instance| running_pids.contains(&instance.pid));

        // Add new instances that we haven't seen before
        for pid in running_pids {
            if !self.instances.values().any(|inst| inst.pid == pid) {
                if let Ok(instance) = self.create_instance_from_pid(pid).await {
                    self.instances.insert(instance.id.clone(), instance);
                }
            }
        }

        Ok(())
    }

    async fn create_instance_from_pid(&self, pid: u32) -> Result<AlacrittyInstance> {
        // Get process info
        let cmdline_path = format!("/proc/{}/cmdline", pid);
        let cmdline = std::fs::read_to_string(cmdline_path)?;
        let args: Vec<&str> = cmdline.split('\0').filter(|s| !s.is_empty()).collect();

        let mut title = format!("alacritty-{}", pid);
        let mut command = "shell".to_string();

        // Parse command line arguments
        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--title" | "-t" => {
                    if i + 1 < args.len() {
                        title = args[i + 1].to_string();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--command" | "-e" => {
                    if i + 1 < args.len() {
                        command = args[i + 1].to_string();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                _ => i += 1,
            }
        }

        let instance_id = Uuid::new_v4().to_string();
        let window_id = self.get_window_id_for_pid(pid).await.ok();

        Ok(AlacrittyInstance {
            id: instance_id,
            pid,
            window_id,
            title,
            command,
            created_at: 0, // We don't know the actual creation time
        })
    }

    async fn get_window_id_for_pid(&self, pid: u32) -> Result<u32> {
        let output = Command::new("xdotool")
            .args(&["search", "--pid", &pid.to_string(), "--class", "Alacritty"])
            .output()?;

        if output.status.success() {
            let window_ids = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = window_ids.lines().next() {
                if let Ok(window_id) = first_line.trim().parse::<u32>() {
                    return Ok(window_id);
                }
            }
        }

        Err(anyhow!("Could not find window ID for PID {}", pid))
    }

    async fn get_window_id_for_instance(&self, instance_id: &str) -> Result<u32> {
        let instance = self.instances.get(instance_id)
            .ok_or_else(|| anyhow!("Instance not found: {}", instance_id))?;

        if let Some(window_id) = instance.window_id {
            return Ok(window_id);
        }

        self.get_window_id_for_pid(instance.pid).await
    }
}

// Add base64 encoding since we're using it
pub mod base64 {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(input: &[u8]) -> String {
        let mut result = String::new();
        let mut i = 0;
        
        while i < input.len() {
            let b1 = input[i];
            let b2 = if i + 1 < input.len() { input[i + 1] } else { 0 };
            let b3 = if i + 2 < input.len() { input[i + 2] } else { 0 };
            
            let bitmap = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);
            
            result.push(CHARS[((bitmap >> 18) & 63) as usize] as char);
            result.push(CHARS[((bitmap >> 12) & 63) as usize] as char);
            result.push(if i + 1 < input.len() { CHARS[((bitmap >> 6) & 63) as usize] as char } else { '=' });
            result.push(if i + 2 < input.len() { CHARS[(bitmap & 63) as usize] as char } else { '=' });
            
            i += 3;
        }
        
        result
    }
}