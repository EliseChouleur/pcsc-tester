use crate::core::reader::PcscReader;
use crate::core::utils::{format_hex, parse_hex};
use anyhow::{bail, Context as AnyhowContext, Result};
use chrono::{DateTime, Utc};
use pcsc::MAX_BUFFER_SIZE;
use serde::{Deserialize, Serialize};

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

        let apdu = parse_hex(apdu_hex).context("Failed to parse APDU hex string")?;

        if apdu.is_empty() {
            bail!("APDU cannot be empty");
        }

        log::info!("Transmitting APDU: {}", format_hex(&apdu));

        let card = reader
            .card()
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
                let error_msg = format!("Transmit failed: {e}");
                log::error!("{error_msg}");

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
    pub fn control(
        &mut self,
        reader: &mut PcscReader,
        code: u32,
        data_hex: &str,
    ) -> Result<ControlResult> {
        let start_time = std::time::Instant::now();
        let timestamp = Utc::now();

        let data = if data_hex.trim().is_empty() {
            Vec::new()
        } else {
            parse_hex(data_hex).context("Failed to parse control data hex string")?
        };

        log::info!(
            "Sending control command: code=0x{:X}, data={}",
            code,
            format_hex(&data)
        );

        let card = reader
            .card()
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
                let error_msg = format!("Control command failed: {e}");
                log::error!("{error_msg}");

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
        serde_json::to_string_pretty(&self.history).context("Failed to serialize command history")
    }

    /// Import history from JSON
    #[allow(dead_code)]
    pub fn import_history(&mut self, json: &str) -> Result<()> {
        let imported: Vec<CommandResult> =
            serde_json::from_str(json).context("Failed to deserialize command history")?;

        self.history.extend(imported);
        Ok(())
    }

    /// Add a command result to history (for testing)
    #[allow(dead_code)]
    pub fn add_to_history(&mut self, result: CommandResult) {
        self.history.push(result);
    }

    /// Get statistics about command history
    pub fn get_statistics(&self) -> CommandStatistics {
        let total_commands = self.history.len();
        let successful_commands = self.history.iter().filter(|cmd| cmd.success).count();
        let failed_commands = total_commands - successful_commands;

        let avg_duration = if !self.history.is_empty() {
            self.history.iter().map(|cmd| cmd.duration_ms).sum::<u64>() / total_commands as u64
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_executor_new() {
        let executor = CommandExecutor::new();
        assert_eq!(executor.history().len(), 0);
    }

    #[test]
    fn test_command_executor_clear_history() {
        let mut executor = CommandExecutor::new();

        // Add some dummy history entries
        executor.history.push(CommandResult {
            timestamp: chrono::Utc::now(),
            command_type: CommandType::Transmit,
            input: vec![0x00, 0x01],
            output: vec![0x90, 0x00],
            success: true,
            error: None,
            duration_ms: 10,
        });

        assert_eq!(executor.history().len(), 1);
        executor.clear_history();
        assert_eq!(executor.history().len(), 0);
    }

    #[test]
    fn test_export_import_history() {
        let mut executor = CommandExecutor::new();

        // Add test command
        executor.history.push(CommandResult {
            timestamp: chrono::Utc::now(),
            command_type: CommandType::Transmit,
            input: vec![0x00, 0xA4, 0x04, 0x00],
            output: vec![0x90, 0x00],
            success: true,
            error: None,
            duration_ms: 25,
        });

        executor.history.push(CommandResult {
            timestamp: chrono::Utc::now(),
            command_type: CommandType::Control { code: 0x42000C00 },
            input: vec![0x01, 0x02],
            output: vec![0x03, 0x04],
            success: true,
            error: None,
            duration_ms: 15,
        });

        // Test export
        let json = executor.export_history().unwrap();
        assert!(json.contains("transmit") || json.contains("Transmit"));
        assert!(json.contains("Control")); // Just check the control type is there

        // Test import
        let mut new_executor = CommandExecutor::new();
        new_executor.import_history(&json).unwrap();
        assert_eq!(new_executor.history().len(), 2);
    }

    #[test]
    fn test_export_import_empty_history() {
        let executor = CommandExecutor::new();
        let json = executor.export_history().unwrap();
        assert_eq!(json.trim(), "[]");

        let mut new_executor = CommandExecutor::new();
        new_executor.import_history(&json).unwrap();
        assert_eq!(new_executor.history().len(), 0);
    }

    #[test]
    fn test_import_invalid_json() {
        let mut executor = CommandExecutor::new();
        assert!(executor.import_history("invalid json").is_err());
        assert!(executor.import_history("{}").is_err());
        assert!(executor
            .import_history("[{\"invalid\": \"structure\"}]")
            .is_err());
    }

    #[test]
    fn test_get_statistics_empty() {
        let executor = CommandExecutor::new();
        let stats = executor.get_statistics();

        assert_eq!(stats.total_commands, 0);
        assert_eq!(stats.successful_commands, 0);
        assert_eq!(stats.failed_commands, 0);
        assert_eq!(stats.average_duration_ms, 0);
    }

    #[test]
    fn test_get_statistics_with_commands() {
        let mut executor = CommandExecutor::new();

        // Add successful command
        executor.history.push(CommandResult {
            timestamp: chrono::Utc::now(),
            command_type: CommandType::Transmit,
            input: vec![0x00, 0xA4],
            output: vec![0x90, 0x00],
            success: true,
            error: None,
            duration_ms: 20,
        });

        // Add failed command
        executor.history.push(CommandResult {
            timestamp: chrono::Utc::now(),
            command_type: CommandType::Control { code: 0x1234 },
            input: vec![0x01],
            output: vec![],
            success: false,
            error: Some("Test error".to_string()),
            duration_ms: 30,
        });

        // Add another successful command
        executor.history.push(CommandResult {
            timestamp: chrono::Utc::now(),
            command_type: CommandType::Transmit,
            input: vec![0x00, 0xB0],
            output: vec![0x61, 0x10],
            success: true,
            error: None,
            duration_ms: 10,
        });

        let stats = executor.get_statistics();
        assert_eq!(stats.total_commands, 3);
        assert_eq!(stats.successful_commands, 2);
        assert_eq!(stats.failed_commands, 1);
        assert_eq!(stats.average_duration_ms, 20); // (20 + 30 + 10) / 3 = 20
    }

    #[test]
    fn test_command_result_serialization() {
        let command = CommandResult {
            timestamp: chrono::Utc::now(),
            command_type: CommandType::Transmit,
            input: vec![0x00, 0xA4, 0x04, 0x00],
            output: vec![0x90, 0x00],
            success: true,
            error: None,
            duration_ms: 42,
        };

        // Test JSON serialization
        let json = serde_json::to_string(&command).unwrap();
        assert!(json.contains("00A40400") || json.contains("[0,164,4,0]"));
        assert!(json.contains("9000") || json.contains("[144,0]"));
        assert!(json.contains("42"));

        // Test deserialization
        let deserialized: CommandResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.input, command.input);
        assert_eq!(deserialized.output, command.output);
        assert_eq!(deserialized.success, command.success);
        assert_eq!(deserialized.duration_ms, command.duration_ms);
    }

    #[test]
    fn test_command_type_serialization() {
        let transmit = CommandType::Transmit;
        let control = CommandType::Control { code: 0x42000C00 };

        let transmit_json = serde_json::to_string(&transmit).unwrap();
        let control_json = serde_json::to_string(&control).unwrap();

        assert!(transmit_json.contains("Transmit"));
        assert!(control_json.contains("Control"));
        // Just check that the JSON is valid, don't check exact format
        assert!(!control_json.is_empty());

        // Test deserialization
        let _: CommandType = serde_json::from_str(&transmit_json).unwrap();
        let deserialized_control: CommandType = serde_json::from_str(&control_json).unwrap();

        match deserialized_control {
            CommandType::Control { code } => assert_eq!(code, 0x42000C00),
            _ => panic!("Expected Control command type"),
        }
    }

    #[test]
    fn test_transmit_result() {
        let result = TransmitResult {
            apdu: vec![0x00, 0xA4, 0x04, 0x00],
            response: vec![0x61, 0x10],
            sw1: 0x61,
            sw2: 0x10,
            duration_ms: 15,
        };

        assert_eq!(result.apdu, vec![0x00, 0xA4, 0x04, 0x00]);
        assert_eq!(result.response, vec![0x61, 0x10]);
        assert_eq!(result.sw1, 0x61);
        assert_eq!(result.sw2, 0x10);
        assert_eq!(result.duration_ms, 15);
    }

    #[test]
    fn test_control_result() {
        let result = ControlResult {
            code: 0x42000C00,
            input: vec![0x01, 0x02],
            output: vec![0x03, 0x04, 0x05],
            duration_ms: 25,
        };

        assert_eq!(result.code, 0x42000C00);
        assert_eq!(result.input, vec![0x01, 0x02]);
        assert_eq!(result.output, vec![0x03, 0x04, 0x05]);
        assert_eq!(result.duration_ms, 25);
    }

    // Note: Testing actual transmit/control would require either real PCSC hardware
    // or mocking the PCSC layer, which would require significant refactoring.
    // For integration tests, we'll test those with the CLI interface.
}
