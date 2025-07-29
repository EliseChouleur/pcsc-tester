/// Unit tests for GUI components that don't require window creation

use pcsc_tester::core::{
    reader::ReaderInfo,
    commands::{CommandExecutor, CommandResult, CommandType},
    utils::{format_hex_spaced, parse_hex, validate_hex_string},
};
use chrono::Utc;

// Test the core logic that the GUI uses
#[test]
fn test_gui_reader_info_display() {
    let reader_with_card = ReaderInfo {
        name: "Test Reader 1".to_string(),
        is_connected: true,
        atr: Some(vec![0x3B, 0x75, 0x13, 0x00]),
    };

    let reader_without_card = ReaderInfo {
        name: "Test Reader 2".to_string(),
        is_connected: false,
        atr: None,
    };

    // Test display formatting for GUI
    assert_eq!(reader_with_card.name, "Test Reader 1");
    assert!(reader_with_card.is_connected);
    assert!(reader_with_card.atr.is_some());

    assert_eq!(reader_without_card.name, "Test Reader 2");
    assert!(!reader_without_card.is_connected);
    assert!(reader_without_card.atr.is_none());
}

#[test]
fn test_gui_command_history_formatting() {
    let mut executor = CommandExecutor::new();

    // Add test commands like GUI would
    executor.add_to_history(CommandResult {
        timestamp: Utc::now(),
        command_type: CommandType::Transmit,
        input: vec![0x00, 0xA4, 0x04, 0x00],
        output: vec![0x90, 0x00],
        success: true,
        error: None,
        duration_ms: 25,
    });

    executor.add_to_history(CommandResult {
        timestamp: Utc::now(),
        command_type: CommandType::Control { code: 0x42000C00 },
        input: vec![0x01, 0x02],
        output: vec![0x03, 0x04],
        success: false,
        error: Some("Connection failed".to_string()),
        duration_ms: 15,
    });

    let history = executor.history();
    assert_eq!(history.len(), 2);

    // Test first command (success)
    let cmd1 = &history[0];
    assert!(cmd1.success);
    assert!(cmd1.error.is_none());
    assert_eq!(cmd1.duration_ms, 25);
    assert!(matches!(cmd1.command_type, CommandType::Transmit));

    // Test second command (failure)
    let cmd2 = &history[1];
    assert!(!cmd2.success);
    assert!(cmd2.error.is_some());
    assert_eq!(cmd2.error.as_ref().unwrap(), "Connection failed");
    assert_eq!(cmd2.duration_ms, 15);
    if let CommandType::Control { code } = cmd2.command_type {
        assert_eq!(code, 0x42000C00);
    } else {
        panic!("Expected Control command type");
    }
}

#[test]
fn test_gui_hex_input_validation() {
    // Test valid hex inputs that GUI would accept
    assert!(validate_hex_string("00A40400").is_ok());
    assert!(validate_hex_string("00 A4 04 00").is_ok());
    assert!(validate_hex_string("0x00,0xA4,0x04,0x00").is_ok());
    assert!(validate_hex_string("").is_ok()); // Empty should be OK

    // Test invalid inputs that GUI should reject
    assert!(validate_hex_string("00A4040").is_err()); // Odd length
    assert!(validate_hex_string("00G40400").is_err()); // Invalid hex char
    assert!(validate_hex_string("Hello").is_err()); // Not hex at all
}

#[test]
fn test_gui_response_formatting() {
    let test_responses = vec![
        vec![], // Empty response
        vec![0x90, 0x00], // Success response
        vec![0x61, 0x10], // Response available
        vec![0x6A, 0x82], // File not found
        b"Hello World".to_vec(), // ASCII data
        (0..32).collect::<Vec<u8>>(), // Long response
    ];

    for response in test_responses {
        // Test hex formatting (what GUI would display)
        let hex_output = format_hex_spaced(&response);
        
        if response.is_empty() {
            assert_eq!(hex_output, "");
        } else {
            assert!(hex_output.len() > 0);
            assert!(!hex_output.contains("x")); // Should not contain 0x prefix
            
            // Should be space-separated pairs
            if response.len() > 1 {
                assert!(hex_output.contains(" "));
            }
        }

        // Test that we can parse back what we format
        if !response.is_empty() {
            let parsed = parse_hex(&hex_output).unwrap();
            assert_eq!(parsed, response);
        }
    }
}

#[test]
fn test_gui_statistics_display() {
    let mut executor = CommandExecutor::new();

    // Add various commands for statistics
    for i in 0..10 {
        executor.add_to_history(CommandResult {
            timestamp: Utc::now(),
            command_type: if i % 2 == 0 { CommandType::Transmit } else { CommandType::Control { code: 0x1234 } },
            input: vec![i as u8],
            output: if i < 7 { vec![0x90, 0x00] } else { vec![] },
            success: i < 7,
            error: if i >= 7 { Some(format!("Error {}", i)) } else { None },
            duration_ms: (i + 1) * 10,
        });
    }

    let stats = executor.get_statistics();
    assert_eq!(stats.total_commands, 10);
    assert_eq!(stats.successful_commands, 7);
    assert_eq!(stats.failed_commands, 3);
    assert_eq!(stats.average_duration_ms, 55); // (10+20+...+100)/10 = 55
}

#[test]
fn test_gui_export_import_functionality() {
    let mut original_executor = CommandExecutor::new();

    // Add test data
    original_executor.add_to_history(CommandResult {
        timestamp: Utc::now(),
        command_type: CommandType::Transmit,
        input: vec![0x00, 0xA4, 0x04, 0x00],
        output: vec![0x90, 0x00],
        success: true,
        error: None,
        duration_ms: 42,
    });

    // Export (like GUI save function would do)
    let json_export = original_executor.export_history().unwrap();
    assert!(!json_export.is_empty());
    assert!(json_export.contains("Transmit") || json_export.contains("transmit"));

    // Import to new executor (like GUI load function would do)
    let mut new_executor = CommandExecutor::new();
    new_executor.import_history(&json_export).unwrap();

    assert_eq!(new_executor.history().len(), 1);
    let imported_cmd = &new_executor.history()[0];
    assert_eq!(imported_cmd.input, vec![0x00, 0xA4, 0x04, 0x00]);
    assert_eq!(imported_cmd.output, vec![0x90, 0x00]);
    assert!(imported_cmd.success);
    assert_eq!(imported_cmd.duration_ms, 42);
}

#[test]  
fn test_gui_control_code_parsing() {
    // Test control codes that GUI might receive
    use pcsc_tester::core::utils::parse_control_code;

    assert_eq!(parse_control_code("0x42000C00").unwrap(), 0x42000C00);
    assert_eq!(parse_control_code("42000C00").unwrap(), 0x42000C00);
    assert_eq!(parse_control_code("123").unwrap(), 123); // Valid decimal value (< 4 chars)
    assert_eq!(parse_control_code("0").unwrap(), 0);

    // Test invalid codes that GUI should handle gracefully
    assert!(parse_control_code("").is_err());
    assert!(parse_control_code("not-a-number").is_err());
    assert!(parse_control_code("0xZZZZ").is_err());
}

#[test]
fn test_gui_reader_selection_logic() {
    let readers = vec![
        ReaderInfo {
            name: "Reader 1".to_string(),
            is_connected: false,
            atr: None,
        },
        ReaderInfo {
            name: "Reader 2".to_string(),
            is_connected: true,
            atr: Some(vec![0x3B, 0xAC, 0x00]),
        },
        ReaderInfo {
            name: "Reader 3".to_string(),
            is_connected: true,
            atr: Some(vec![0x3B, 0x75, 0x13, 0x00]),
        },
    ];

    // Test GUI display names
    let display_names: Vec<String> = readers
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let status = if r.is_connected { " [CARD]" } else { "" };
            format!("[{}] {}{}", i, r.name, status)
        })
        .collect();

    assert_eq!(display_names[0], "[0] Reader 1");
    assert_eq!(display_names[1], "[1] Reader 2 [CARD]");
    assert_eq!(display_names[2], "[2] Reader 3 [CARD]");

    // Test finding readers with cards
    let readers_with_cards: Vec<&ReaderInfo> = readers
        .iter()
        .filter(|r| r.is_connected)
        .collect();

    assert_eq!(readers_with_cards.len(), 2);
    assert_eq!(readers_with_cards[0].name, "Reader 2");
    assert_eq!(readers_with_cards[1].name, "Reader 3");
}

#[test]
fn test_gui_error_handling() {
    let mut executor = CommandExecutor::new();

    // Test adding error results (what GUI would do on failed commands)
    executor.add_to_history(CommandResult {
        timestamp: Utc::now(),
        command_type: CommandType::Transmit,
        input: vec![0x00, 0xA4, 0x04, 0x00],
        output: vec![],
        success: false,
        error: Some("Card not present".to_string()),
        duration_ms: 5,
    });

    executor.add_to_history(CommandResult {
        timestamp: Utc::now(),
        command_type: CommandType::Control { code: 0x42000C00 },
        input: vec![0x01],
        output: vec![],
        success: false,
        error: Some("Reader not connected".to_string()),
        duration_ms: 1,
    });

    let stats = executor.get_statistics();
    assert_eq!(stats.total_commands, 2);
    assert_eq!(stats.successful_commands, 0);
    assert_eq!(stats.failed_commands, 2);

    // Test that errors are properly stored
    let history = executor.history();
    assert!(history[0].error.as_ref().unwrap().contains("Card not present"));
    assert!(history[1].error.as_ref().unwrap().contains("Reader not connected"));
}

// Test performance characteristics that GUI cares about
#[test]
fn test_gui_performance_characteristics() {
    let mut executor = CommandExecutor::new();

    // Add a large number of commands to test GUI performance
    let num_commands = 1000;
    let start = std::time::Instant::now();

    for i in 0..num_commands {
        executor.add_to_history(CommandResult {
            timestamp: Utc::now(),
            command_type: CommandType::Transmit,
            input: vec![i as u8, (i >> 8) as u8],
            output: vec![0x90, 0x00],
            success: true,
            error: None,
            duration_ms: i as u64,
        });
    }

    let insert_duration = start.elapsed();
    assert!(insert_duration.as_millis() < 100, "Adding commands took too long");

    // Test statistics calculation performance
    let stats_start = std::time::Instant::now();
    let stats = executor.get_statistics();
    let stats_duration = stats_start.elapsed();
    
    assert_eq!(stats.total_commands, num_commands);
    assert!(stats_duration.as_millis() < 10, "Statistics calculation took too long");

    // Test export performance
    let export_start = std::time::Instant::now();
    let _json = executor.export_history().unwrap();
    let export_duration = export_start.elapsed();
    
    assert!(export_duration.as_millis() < 1000, "Export took too long");
}