use std::env;
use anyhow::Result;

mod cli;
mod gui;
mod core;

use cli::commands::run_cli;
use gui::app::run_gui;

fn main() -> Result<()> {
    // Check if we have command line arguments (excluding program name)
    let args: Vec<String> = env::args().collect();
    
    // If no arguments provided or only "--gui" flag, start GUI
    if args.len() == 1 || (args.len() == 2 && args[1] == "--gui") {
        // GUI mode
        println!("Starting PCSC Tester GUI...");
        run_gui()
    } else {
        // CLI mode - pass arguments to CLI parser
        run_cli()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::utils::*;

    #[test]
    fn test_hex_parsing() {
        assert_eq!(parse_hex("0102030A").unwrap(), vec![0x01, 0x02, 0x03, 0x0A]);
        assert_eq!(parse_hex("01 02 03 0A").unwrap(), vec![0x01, 0x02, 0x03, 0x0A]);
        assert_eq!(parse_hex("").unwrap(), vec![]);
    }

    #[test]
    fn test_control_code_parsing() {
        assert_eq!(parse_control_code("0x1234").unwrap(), 0x1234);
        assert_eq!(parse_control_code("1234").unwrap(), 1234);
    }

    #[test]
    fn test_format_functions() {
        let bytes = vec![0x01, 0x02, 0x03, 0x0A];
        assert_eq!(format_hex(&bytes), "0102030A");
        assert_eq!(format_hex_spaced(&bytes), "01 02 03 0A");
    }
    
    #[test]
    fn test_status_word_descriptions() {
        assert_eq!(describe_status_word(0x90, 0x00), "Success");
        assert_eq!(describe_status_word(0x6A, 0x82), "Error: File not found");
    }
}