use pcsc::MAX_BUFFER_SIZE;
use anyhow::{Result, Context as AnyhowContext, bail};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::core::utils::{parse_hex, format_hex};
use crate::core::reader::PcscReader;

/// Result of any command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub timestamp: DateTime<Utc>,
    pub command_type: CommandType,
    pub input: Vec<u8>,
    pub output: Vec<u8>,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Types of commands supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandType {
    Transmit,
    Control { code: u32 },
}

/// Result specifically for transmit commands
#[derive(Debug, Clone)]
pub struct TransmitResult {
    pub apdu: Vec<u8>,
    pub response: Vec<u8>,
    pub sw1: u8,
    pub sw2: u8,
    pub duration_ms: u64,
}

/// Result specifically for control commands  
#[derive(Debug, Clone)]
pub struct ControlResult {
    pub code: u32,
    pub input: Vec<u8>,
    pub output: Vec<u8>,
    pub duration_ms: u64,
}

/// Command executor for PCSC operations
pub struct CommandExecutor {
    history: Vec<CommandResult>,
}

impl CommandExecutor {
    /// Create a new command executor
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    /// Execute a transmit command (APDU)
    pub fn transmit(&mut self, reader: &mut PcscReader, apdu_hex: &str) -> Result<TransmitResult> {
        let start_time = std::time::Instant::now();
        let timestamp = Utc::now();
        
        let apdu = parse_hex(apdu_hex)
            .context("Failed to parse APDU hex string")?;
        
        if apdu.is_empty() {
            bail!("APDU cannot be empty");
        }

        log::info!("Transmitting APDU: {}", format_hex(&apdu));
        
        let card = reader.card()
            .ok_or_else(|| anyhow::anyhow!("No card connected"))?;

        let mut response_buf = [0; MAX_BUFFER_SIZE];
        
        let result = card.transmit(&apdu, &mut response_buf);
        let duration = start_time.elapsed();
        
        match result {
            Ok(response) => {
                let response_vec = response.to_vec();
                log::info!("Received response: {}", format_hex(&response_vec));
                
                // Extract SW1 and SW2 (last 2 bytes)
                let (sw1, sw2) = if response_vec.len() >= 2 {
                    let len = response_vec.len();
                    (response_vec[len - 2], response_vec[len - 1])
                } else {
                    (0, 0)
                };
                
                // Record successful command
                let command_result = CommandResult {
                    timestamp,
                    command_type: CommandType::Transmit,
                    input: apdu.clone(),
                    output: response_vec.clone(),
                    success: true,
                    error: None,
                    duration_ms: duration.as_millis() as u64,
                };
                self.history.push(command_result);
                
                Ok(TransmitResult {
                    apdu,
                    response: response_vec,
                    sw1,
                    sw2,
                    duration_ms: duration.as_millis() as u64,
                })
            }
            Err(e) => {
                let error_msg = format!("Transmit failed: {}", e);
                log::error!("{}", error_msg);
                
                // Record failed command
                let command_result = CommandResult {
                    timestamp,
                    command_type: CommandType::Transmit,
                    input: apdu.clone(),
                    output: Vec::new(),
                    success: false,
                    error: Some(error_msg.clone()),
                    duration_ms: duration.as_millis() as u64,
                };
                self.history.push(command_result);
                
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    /// Execute a control command
    pub fn control(&mut self, reader: &mut PcscReader, code: u32, data_hex: &str) -> Result<ControlResult> {
        let start_time = std::time::Instant::now();
        let timestamp = Utc::now();
        
        let data = if data_hex.trim().is_empty() {
            Vec::new()
        } else {
            parse_hex(data_hex)
                .context("Failed to parse control data hex string")?
        };
        
        log::info!("Sending control command: code=0x{:X}, data={}", code, format_hex(&data));
        
        let card = reader.card()
            .ok_or_else(|| anyhow::anyhow!("No card connected"))?;

        let mut response_buf = [0; MAX_BUFFER_SIZE];
        
        let result = card.control(code, &data, &mut response_buf);
        let duration = start_time.elapsed();
        
        match result {
            Ok(response) => {
                let response_vec = response.to_vec();
                log::info!("Control response: {}", format_hex(&response_vec));
                
                // Record successful command
                let command_result = CommandResult {
                    timestamp,
                    command_type: CommandType::Control { code },
                    input: data.clone(),
                    output: response_vec.clone(),
                    success: true,
                    error: None,
                    duration_ms: duration.as_millis() as u64,
                };
                self.history.push(command_result);
                
                Ok(ControlResult {
                    code,
                    input: data,
                    output: response_vec,
                    duration_ms: duration.as_millis() as u64,
                })
            }
            Err(e) => {
                let error_msg = format!("Control command failed: {}", e);
                log::error!("{}", error_msg);
                
                // Record failed command
                let command_result = CommandResult {
                    timestamp,
                    command_type: CommandType::Control { code },
                    input: data.clone(),
                    output: Vec::new(),
                    success: false,
                    error: Some(error_msg.clone()),
                    duration_ms: duration.as_millis() as u64,
                };
                self.history.push(command_result);
                
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    /// Get command history
    pub fn history(&self) -> &[CommandResult] {
        &self.history
    }

    /// Clear command history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Export history to JSON
    pub fn export_history(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.history)
            .context("Failed to serialize command history")
    }

    /// Import history from JSON
    pub fn import_history(&mut self, json: &str) -> Result<()> {
        let imported: Vec<CommandResult> = serde_json::from_str(json)
            .context("Failed to deserialize command history")?;
        
        self.history.extend(imported);
        Ok(())
    }

    /// Get statistics about command history
    pub fn get_statistics(&self) -> CommandStatistics {
        let total_commands = self.history.len();
        let successful_commands = self.history.iter()
            .filter(|cmd| cmd.success)
            .count();
        let failed_commands = total_commands - successful_commands;
        
        let avg_duration = if !self.history.is_empty() {
            self.history.iter()
                .map(|cmd| cmd.duration_ms)
                .sum::<u64>() / total_commands as u64
        } else {
            0
        };

        CommandStatistics {
            total_commands,
            successful_commands,
            failed_commands,
            average_duration_ms: avg_duration,
        }
    }
}

/// Statistics about executed commands
#[derive(Debug, Clone)]
pub struct CommandStatistics {
    pub total_commands: usize,
    pub successful_commands: usize, 
    pub failed_commands: usize,
    pub average_duration_ms: u64,
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}