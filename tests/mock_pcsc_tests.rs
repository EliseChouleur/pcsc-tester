/// Mock PCSC tests for testing without real hardware

use pcsc_tester::core::{
    reader::ReaderInfo,
    commands::{CommandExecutor, CommandResult, CommandType},
    utils::{parse_hex, format_hex_spaced},
};
use chrono::Utc;

/// Mock PCSC reader for testing
#[derive(Debug, Clone)]
struct MockPcscReader {
    readers: Vec<ReaderInfo>,
    current_reader: Option<String>,
    is_connected: bool,
    should_fail: bool,
    mock_responses: std::collections::HashMap<Vec<u8>, Vec<u8>>,
}

impl MockPcscReader {
    fn new() -> Self {
        Self {
            readers: vec![
                ReaderInfo {
                    name: "Mock Reader 1".to_string(),
                    is_connected: true,
                    atr: Some(vec![0x3B, 0xAC, 0x00, 0x40, 0x2A, 0x00, 0x12, 0x25, 0x00, 0x64, 0x80, 0x00, 0x03, 0x10, 0x00, 0x90, 0x00]),
                },
                ReaderInfo {
                    name: "Mock Reader 2".to_string(),
                    is_connected: false,
                    atr: None,
                },
                ReaderInfo {
                    name: "Mock Reader 3".to_string(),
                    is_connected: true,
                    atr: Some(vec![0x3B, 0x75, 0x13, 0x00, 0x00, 0x47, 0x09, 0xEA, 0x90, 0x00]),
                },
            ],
            current_reader: None,
            is_connected: false,
            should_fail: false,
            mock_responses: std::collections::HashMap::new(),
        }
    }

    fn with_failure(&mut self) -> &mut Self {
        self.should_fail = true;
        self
    }

    fn add_mock_response(&mut self, command: Vec<u8>, response: Vec<u8>) {
        self.mock_responses.insert(command, response);
    }

    fn list_readers(&self) -> Result<Vec<ReaderInfo>, String> {
        if self.should_fail {
            return Err("Mock PCSC error: Failed to list readers".to_string());
        }
        Ok(self.readers.clone())
    }

    fn connect(&mut self, reader_name: &str) -> Result<(), String> {
        if self.should_fail {
            return Err("Mock PCSC error: Failed to connect".to_string());
        }

        if let Some(reader) = self.readers.iter().find(|r| r.name == reader_name) {
            if !reader.is_connected {
                return Err("Mock PCSC error: No card present".to_string());
            }
            self.current_reader = Some(reader_name.to_string());
            self.is_connected = true;
            Ok(())
        } else {
            Err("Mock PCSC error: Reader not found".to_string())
        }
    }

    fn disconnect(&mut self) -> Result<(), String> {
        self.current_reader = None;
        self.is_connected = false;
        Ok(())
    }

    fn transmit(&self, apdu: &[u8]) -> Result<Vec<u8>, String> {
        if self.should_fail {
            return Err("Mock PCSC error: Transmit failed".to_string());
        }

        if !self.is_connected {
            return Err("Mock PCSC error: Not connected to reader".to_string());
        }

        // Check for mock responses
        if let Some(response) = self.mock_responses.get(apdu) {
            return Ok(response.clone());
        }

        // Default responses for common commands
        match apdu {
            [0x00, 0xA4, 0x04, 0x00] => Ok(vec![0x90, 0x00]), // SELECT success
            [0x00, 0xA4, 0x04, 0x00, ..] => Ok(vec![0x90, 0x00]), // SELECT with data
            [0x00, 0xB0, 0x00, 0x00, ..] => Ok(vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x90, 0x00]), // READ BINARY
            [0x00, 0xC0, 0x00, 0x00, ..] => Ok(vec![0x61, 0x10]), // GET RESPONSE
            [0x80, 0xCA, 0x9F, 0x7F, 0x00] => Ok(vec![0x6A, 0x88]), // Unsupported
            _ => Ok(vec![0x6D, 0x00]), // INS not supported
        }
    }

    fn control(&self, code: u32, data: &[u8]) -> Result<Vec<u8>, String> {
        if self.should_fail {
            return Err("Mock PCSC error: Control failed".to_string());
        }

        if !self.is_connected {
            return Err("Mock PCSC error: Not connected to reader".to_string());
        }

        // Mock responses for common control codes
        match code {
            0x42000C00 => Ok(vec![0x01, 0x02, 0x03, 0x04]), // Firmware version
            0x42000D48 => Ok(vec![0xFF, 0xFF, 0xFF, 0xFF]), // Reader features
            0x42000E01 => Ok(data.to_vec()), // Echo command
            _ => Ok(vec![]), // Empty response for unknown codes
        }
    }
}

#[test]
fn test_mock_reader_creation() {
    let mock_reader = MockPcscReader::new();
    assert_eq!(mock_reader.readers.len(), 3);
    assert!(!mock_reader.is_connected);
    assert!(mock_reader.current_reader.is_none());
}

#[test]
fn test_mock_list_readers() {
    let mock_reader = MockPcscReader::new();
    let readers = mock_reader.list_readers().unwrap();
    
    assert_eq!(readers.len(), 3);
    assert_eq!(readers[0].name, "Mock Reader 1");
    assert!(readers[0].is_connected);
    assert!(readers[0].atr.is_some());
    
    assert_eq!(readers[1].name, "Mock Reader 2");
    assert!(!readers[1].is_connected);
    assert!(readers[1].atr.is_none());
}

#[test]
fn test_mock_list_readers_failure() {
    let mut mock_reader = MockPcscReader::new();
    mock_reader.with_failure();
    
    let result = mock_reader.list_readers();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Failed to list readers"));
}

#[test]
fn test_mock_connect_success() {
    let mut mock_reader = MockPcscReader::new();
    let result = mock_reader.connect("Mock Reader 1");
    
    assert!(result.is_ok());
    assert!(mock_reader.is_connected);
    assert_eq!(mock_reader.current_reader.as_ref().unwrap(), "Mock Reader 1");
}

#[test]
fn test_mock_connect_no_card() {
    let mut mock_reader = MockPcscReader::new();
    let result = mock_reader.connect("Mock Reader 2"); // This reader has no card
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No card present"));
    assert!(!mock_reader.is_connected);
}

#[test]
fn test_mock_connect_reader_not_found() {
    let mut mock_reader = MockPcscReader::new();
    let result = mock_reader.connect("Nonexistent Reader");
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Reader not found"));
    assert!(!mock_reader.is_connected);
}

#[test]
fn test_mock_transmit_success() {
    let mut mock_reader = MockPcscReader::new();
    mock_reader.connect("Mock Reader 1").unwrap();
    
    // Test SELECT command
    let apdu = vec![0x00, 0xA4, 0x04, 0x00];
    let response = mock_reader.transmit(&apdu).unwrap();
    assert_eq!(response, vec![0x90, 0x00]);
    
    // Test READ BINARY command
    let read_apdu = vec![0x00, 0xB0, 0x00, 0x00, 0x05];
    let read_response = mock_reader.transmit(&read_apdu).unwrap();
    assert_eq!(read_response, vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x90, 0x00]); // "Hello" + SW
}

#[test]
fn test_mock_transmit_not_connected() {
    let mock_reader = MockPcscReader::new();
    let apdu = vec![0x00, 0xA4, 0x04, 0x00];
    let result = mock_reader.transmit(&apdu);
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Not connected"));
}

#[test]
fn test_mock_transmit_with_custom_response() {
    let mut mock_reader = MockPcscReader::new();
    mock_reader.connect("Mock Reader 1").unwrap();
    
    // Add custom mock response
    let custom_command = vec![0x80, 0x12, 0x34, 0x56];
    let custom_response = vec![0xAB, 0xCD, 0xEF, 0x90, 0x00];
    mock_reader.add_mock_response(custom_command.clone(), custom_response.clone());
    
    let response = mock_reader.transmit(&custom_command).unwrap();
    assert_eq!(response, custom_response);
}

#[test]
fn test_mock_control_success() {
    let mut mock_reader = MockPcscReader::new();
    mock_reader.connect("Mock Reader 1").unwrap();
    
    // Test firmware version command
    let response = mock_reader.control(0x42000C00, &[]).unwrap();
    assert_eq!(response, vec![0x01, 0x02, 0x03, 0x04]);
    
    // Test echo command
    let test_data = vec![0xAA, 0xBB, 0xCC];
    let echo_response = mock_reader.control(0x42000E01, &test_data).unwrap();
    assert_eq!(echo_response, test_data);
}

#[test]
fn test_mock_control_failure() {
    let mut mock_reader = MockPcscReader::new();
    mock_reader.with_failure();
    mock_reader.connect("Mock Reader 1").unwrap_err(); // Connection will fail
    
    let result = mock_reader.control(0x42000C00, &[]);
    assert!(result.is_err());
}

#[test]
fn test_mock_disconnect() {
    let mut mock_reader = MockPcscReader::new();
    mock_reader.connect("Mock Reader 1").unwrap();
    assert!(mock_reader.is_connected);
    
    mock_reader.disconnect().unwrap();
    assert!(!mock_reader.is_connected);
    assert!(mock_reader.current_reader.is_none());
}

#[test]
fn test_mock_command_executor_integration() {
    let mut mock_reader = MockPcscReader::new();
    let mut executor = CommandExecutor::new();
    
    // Mock a successful transmit operation
    mock_reader.connect("Mock Reader 1").unwrap();
    let apdu = vec![0x00, 0xA4, 0x04, 0x00];
    let response = mock_reader.transmit(&apdu).unwrap();
    
    // Simulate what CommandExecutor would do
    let start_time = std::time::Instant::now();
    let duration = start_time.elapsed();
    
    executor.add_to_history(CommandResult {
        timestamp: Utc::now(),
        command_type: CommandType::Transmit,
        input: apdu,
        output: response.clone(),
        success: true,
        error: None,
        duration_ms: duration.as_millis() as u64,
    });
    
    let history = executor.history();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].output, vec![0x90, 0x00]);
    assert!(history[0].success);
}

#[test]
fn test_mock_error_scenarios() {
    let mut mock_reader = MockPcscReader::new();
    
    // Test connection to reader without card
    let connect_result = mock_reader.connect("Mock Reader 2");
    assert!(connect_result.is_err());
    
    // Test transmit without connection
    let transmit_result = mock_reader.transmit(&[0x00, 0xA4, 0x04, 0x00]);
    assert!(transmit_result.is_err());
    
    // Test control without connection
    let control_result = mock_reader.control(0x42000C00, &[]);
    assert!(control_result.is_err());
}

#[test]
fn test_mock_atr_parsing() {
    let mock_reader = MockPcscReader::new();
    let readers = mock_reader.list_readers().unwrap();
    
    // Test ATR formatting for display
    for reader in &readers {
        if let Some(ref atr) = reader.atr {
            let atr_string = format_hex_spaced(atr);
            assert!(!atr_string.is_empty());
            
            // Verify we can parse it back
            let parsed_atr = parse_hex(&atr_string).unwrap();
            assert_eq!(parsed_atr, *atr);
        }
    }
}

#[test]
fn test_mock_performance() {
    let mut mock_reader = MockPcscReader::new();
    mock_reader.connect("Mock Reader 1").unwrap();
    
    let start = std::time::Instant::now();
    
    // Perform multiple operations
    for i in 0..100 {
        let apdu = vec![0x00, 0xA4, 0x04, 0x00, i as u8];
        let _response = mock_reader.transmit(&apdu).unwrap();
    }
    
    let duration = start.elapsed();
    assert!(duration.as_millis() < 100, "Mock operations should be fast");
}

// Integration test with mock CLI-like workflow
#[test]
fn test_mock_cli_workflow() {
    let mut mock_reader = MockPcscReader::new();
    let mut executor = CommandExecutor::new();
    
    // Step 1: List readers (like CLI list command)
    let readers = mock_reader.list_readers().unwrap();
    assert_eq!(readers.len(), 3);
    
    // Step 2: Connect to first reader with card
    let reader_with_card = readers.iter().find(|r| r.is_connected).unwrap();
    mock_reader.connect(&reader_with_card.name).unwrap();
    
    // Step 3: Send transmit command (like CLI transmit command)
    let apdu_hex = "00A40400";
    let apdu = parse_hex(apdu_hex).unwrap();
    let response = mock_reader.transmit(&apdu).unwrap();
    
    // Record in history
    executor.add_to_history(CommandResult {
        timestamp: Utc::now(),
        command_type: CommandType::Transmit,
        input: apdu,
        output: response.clone(),
        success: true,
        error: None,
        duration_ms: 25,
    });
    
    // Step 4: Send control command
    let control_response = mock_reader.control(0x42000C00, &[]).unwrap();
    executor.add_to_history(CommandResult {
        timestamp: Utc::now(),
        command_type: CommandType::Control { code: 0x42000C00 },
        input: vec![],
        output: control_response,
        success: true,
        error: None,
        duration_ms: 15,
    });
    
    // Verify workflow
    let stats = executor.get_statistics();
    assert_eq!(stats.total_commands, 2);
    assert_eq!(stats.successful_commands, 2);
    assert_eq!(stats.failed_commands, 0);
}