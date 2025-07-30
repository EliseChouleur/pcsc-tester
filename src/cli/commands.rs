use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use pcsc::ShareMode;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

use crate::core::{
    commands::CommandExecutor,
    reader::PcscReader,
    utils::{
        describe_status_word, format_ascii, format_hex_dump, format_hex_spaced, parse_control_code,
    },
};

#[derive(Parser)]
#[command(name = "pcsc-tester")]
#[command(about = "Cross-platform PCSC tool for testing smart card readers")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Enable debug logging  
    #[arg(short, long, global = true)]
    pub debug: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available PCSC readers
    List {
        /// Show detailed information about readers
        #[arg(short = 'l', long)]
        detailed: bool,
    },

    /// Send APDU command via transmit
    Transmit {
        /// Reader name or index (use 'list' to see available readers)
        reader: String,

        /// APDU command in hex format (e.g., "00A40400")
        apdu: String,

        /// Connection share mode
        #[arg(short, long, default_value = "shared")]
        mode: ShareModeArg,

        /// Show response in different formats
        #[arg(short, long)]
        format: Option<ResponseFormat>,
    },

    /// Send control command to reader
    Control {
        /// Reader name or index  
        reader: String,

        /// Control code (decimal or hex with 0x prefix)
        code: String,

        /// Data to send in hex format (optional)
        #[arg(default_value = "")]
        data: String,

        /// Connection share mode (use 'direct' for reader control)
        #[arg(short, long, default_value = "direct")]
        mode: ShareModeArg,

        /// Show response in different formats
        #[arg(short, long)]
        format: Option<ResponseFormat>,
    },

    /// Execute commands from a script file
    Script {
        /// Script file path
        file: String,

        /// Reader name or index to use for all commands
        reader: String,

        /// Connection share mode
        #[arg(short, long, default_value = "shared")]
        mode: ShareModeArg,

        /// Continue on errors
        #[arg(short, long)]
        continue_on_error: bool,
    },

    /// Interactive mode
    Interactive {
        /// Reader name or index (optional, can select interactively)
        reader: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub enum ShareModeArg {
    Shared,
    Exclusive,
    Direct,
}

impl std::str::FromStr for ShareModeArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "shared" => Ok(ShareModeArg::Shared),
            "exclusive" => Ok(ShareModeArg::Exclusive),
            "direct" => Ok(ShareModeArg::Direct),
            _ => Err(format!("Invalid share mode: {s}")),
        }
    }
}

impl From<ShareModeArg> for ShareMode {
    fn from(mode: ShareModeArg) -> Self {
        match mode {
            ShareModeArg::Shared => ShareMode::Shared,
            ShareModeArg::Exclusive => ShareMode::Exclusive,
            ShareModeArg::Direct => ShareMode::Direct,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ResponseFormat {
    Hex,
    HexSpaced,
    HexDump,
    Ascii,
    All,
}

impl std::str::FromStr for ResponseFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hex" => Ok(ResponseFormat::Hex),
            "spaced" => Ok(ResponseFormat::HexSpaced),
            "dump" => Ok(ResponseFormat::HexDump),
            "ascii" => Ok(ResponseFormat::Ascii),
            "all" => Ok(ResponseFormat::All),
            _ => Err(format!("Invalid format: {s}")),
        }
    }
}

pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging
    let log_level = if cli.debug {
        log::LevelFilter::Debug
    } else if cli.verbose {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };

    env_logger::Builder::from_default_env()
        .filter_level(log_level)
        .init();

    match cli.command {
        Commands::List { detailed } => cmd_list(detailed),
        Commands::Transmit {
            reader,
            apdu,
            mode,
            format,
        } => cmd_transmit(&reader, &apdu, mode, format),
        Commands::Control {
            reader,
            code,
            data,
            mode,
            format,
        } => cmd_control(&reader, &code, &data, mode, format),
        Commands::Script {
            file,
            reader,
            mode,
            continue_on_error,
        } => cmd_script(&file, &reader, mode, continue_on_error),
        Commands::Interactive { reader } => cmd_interactive(reader.as_deref()),
    }
}

fn cmd_list(detailed: bool) -> Result<()> {
    let reader = PcscReader::new().context("Failed to initialize PCSC")?;

    let readers = reader.list_readers().context("Failed to list readers")?;

    if readers.is_empty() {
        println!("No PCSC readers found.");
        return Ok(());
    }

    println!("Available PCSC readers:");
    for (i, reader_info) in readers.iter().enumerate() {
        if detailed {
            println!("  [{}] {}", i, reader_info.name);
            println!(
                "      Status: {}",
                if reader_info.is_connected {
                    "Card present"
                } else {
                    "No card"
                }
            );
            if let Some(ref atr) = reader_info.atr {
                println!("      ATR: {}", format_hex_spaced(atr));
            }
        } else if reader_info.is_connected {
            if let Some(ref atr) = reader_info.atr {
                println!(
                    "  [{}] {} [CARD - ATR: {}]",
                    i,
                    reader_info.name,
                    format_hex_spaced(atr)
                );
            } else {
                println!("  [{}] {} [CARD]", i, reader_info.name);
            }
        } else {
            println!("  [{}] {}", i, reader_info.name);
        }
    }

    Ok(())
}

fn cmd_transmit(
    reader_name: &str,
    apdu_hex: &str,
    mode: ShareModeArg,
    format: Option<ResponseFormat>,
) -> Result<()> {
    let mut reader = PcscReader::new().context("Failed to initialize PCSC")?;

    let reader_name = resolve_reader_name(&reader, reader_name)?;
    reader
        .connect(&reader_name, mode.into())
        .with_context(|| format!("Failed to connect to reader: {reader_name}"))?;

    let mut executor = CommandExecutor::new();
    let result = executor
        .transmit(&mut reader, apdu_hex)
        .context("Failed to execute transmit command")?;

    println!("APDU: {}", format_hex_spaced(&result.apdu));

    let format = format.unwrap_or(ResponseFormat::HexSpaced);
    print_response(&result.response, &format);

    // Show status word interpretation
    println!(
        "Status: {:02X} {:02X} ({})",
        result.sw1,
        result.sw2,
        describe_status_word(result.sw1, result.sw2)
    );
    println!("Duration: {}ms", result.duration_ms);

    Ok(())
}

fn cmd_control(
    reader_name: &str,
    code_str: &str,
    data_hex: &str,
    mode: ShareModeArg,
    format: Option<ResponseFormat>,
) -> Result<()> {
    let mut reader = PcscReader::new().context("Failed to initialize PCSC")?;

    let reader_name = resolve_reader_name(&reader, reader_name)?;
    reader
        .connect(&reader_name, mode.into())
        .with_context(|| format!("Failed to connect to reader: {reader_name}"))?;

    let code = parse_control_code(code_str).context("Failed to parse control code")?;

    let mut executor = CommandExecutor::new();
    let result = executor
        .control(&mut reader, code, data_hex)
        .context("Failed to execute control command")?;

    println!("Control Code: 0x{:X} ({})", result.code, result.code);
    if !result.input.is_empty() {
        println!("Input: {}", format_hex_spaced(&result.input));
    }

    let format = format.unwrap_or(ResponseFormat::HexSpaced);
    print_response(&result.output, &format);

    println!("Duration: {}ms", result.duration_ms);

    Ok(())
}

fn cmd_script(
    file_path: &str,
    reader_name: &str,
    mode: ShareModeArg,
    continue_on_error: bool,
) -> Result<()> {
    let mut reader = PcscReader::new().context("Failed to initialize PCSC")?;

    let reader_name = resolve_reader_name(&reader, reader_name)?;
    reader
        .connect(&reader_name, mode.into())
        .with_context(|| format!("Failed to connect to reader: {reader_name}"))?;

    let file = File::open(file_path)
        .with_context(|| format!("Failed to open script file: {file_path}"))?;

    let reader_buf = BufReader::new(file);
    let mut executor = CommandExecutor::new();
    let mut line_number = 0;
    let mut errors = 0;

    println!("Executing script: {file_path}");
    println!("Reader: {reader_name}");
    println!();

    for line in reader_buf.lines() {
        line_number += 1;
        let line = line.context("Failed to read line from script")?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        println!("Line {line_number}: {line}");

        // Parse command (simple format: "transmit <apdu>" or "control <code> <data>")
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let result = match parts[0].to_lowercase().as_str() {
            "transmit" => {
                if parts.len() < 2 {
                    Err(anyhow::anyhow!("Missing APDU for transmit command"))
                } else {
                    executor
                        .transmit(&mut reader, parts[1])
                        .map(|r| (r.response, r.duration_ms))
                }
            }
            "control" => {
                if parts.len() < 2 {
                    Err(anyhow::anyhow!("Missing control code"))
                } else {
                    let code = parse_control_code(parts[1])?;
                    let data = parts.get(2).unwrap_or(&"");
                    executor
                        .control(&mut reader, code, data)
                        .map(|r| (r.output, r.duration_ms))
                }
            }
            _ => Err(anyhow::anyhow!("Unknown command: {}", parts[0])),
        };

        match result {
            Ok((response, duration)) => {
                println!(
                    "  Response: {} ({}ms)",
                    format_hex_spaced(&response),
                    duration
                );
            }
            Err(e) => {
                errors += 1;
                println!("  ERROR: {e}");
                if !continue_on_error {
                    bail!(
                        "Script execution stopped due to error on line {}",
                        line_number
                    );
                }
            }
        }
        println!();
    }

    println!("Script execution completed.");
    println!("Total lines processed: {line_number}");
    if errors > 0 {
        println!("Errors encountered: {errors}");
    }

    Ok(())
}

fn cmd_interactive(reader_name: Option<&str>) -> Result<()> {
    let mut reader = PcscReader::new().context("Failed to initialize PCSC")?;

    let reader_name = if let Some(name) = reader_name {
        resolve_reader_name(&reader, name)?
    } else {
        // Let user select reader interactively
        select_reader_interactive(&reader)?
    };

    reader
        .connect(&reader_name, ShareMode::Shared)
        .with_context(|| format!("Failed to connect to reader: {reader_name}"))?;

    let mut executor = CommandExecutor::new();

    println!("PCSC Tester - Interactive Mode");
    println!("Connected to: {reader_name}");
    println!("Commands: transmit <apdu>, control <code> [data], history, clear, help, quit");
    println!();

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::Write::flush(&mut io::stdout()).unwrap();

        let mut input = String::new();
        stdin
            .read_line(&mut input)
            .context("Failed to read input")?;

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        match parts[0].to_lowercase().as_str() {
            "quit" | "exit" | "q" => break,
            "help" | "h" => {
                println!("Available commands:");
                println!("  transmit <apdu>     - Send APDU command");
                println!("  control <code> [data] - Send control command");
                println!("  history             - Show command history");
                println!("  clear               - Clear command history");
                println!("  help                - Show this help");
                println!("  quit                - Exit interactive mode");
            }
            "transmit" | "t" => {
                if parts.len() < 2 {
                    println!("Error: Missing APDU");
                    continue;
                }

                match executor.transmit(&mut reader, parts[1]) {
                    Ok(result) => {
                        println!("Response: {}", format_hex_spaced(&result.response));
                        println!(
                            "Status: {:02X} {:02X} ({})",
                            result.sw1,
                            result.sw2,
                            describe_status_word(result.sw1, result.sw2)
                        );
                        println!("Duration: {}ms", result.duration_ms);
                    }
                    Err(e) => println!("Error: {e}"),
                }
            }
            "control" | "c" => {
                if parts.len() < 2 {
                    println!("Error: Missing control code");
                    continue;
                }

                match parse_control_code(parts[1]) {
                    Ok(code) => {
                        let data = parts.get(2).unwrap_or(&"");
                        match executor.control(&mut reader, code, data) {
                            Ok(result) => {
                                println!("Response: {}", format_hex_spaced(&result.output));
                                println!("Duration: {}ms", result.duration_ms);
                            }
                            Err(e) => println!("Error: {e}"),
                        }
                    }
                    Err(e) => println!("Error: {e}"),
                }
            }
            "history" => {
                let history = executor.history();
                if history.is_empty() {
                    println!("No commands in history");
                } else {
                    println!("Command history:");
                    for (i, cmd) in history.iter().enumerate() {
                        let status = if cmd.success { "OK" } else { "ERROR" };
                        let cmd_type = match &cmd.command_type {
                            crate::core::commands::CommandType::Transmit => "TRANSMIT",
                            crate::core::commands::CommandType::Control { code } => {
                                &format!("CONTROL(0x{code:X})")
                            }
                        };
                        println!(
                            "  [{}] {} {} - {} ({}ms)",
                            i + 1,
                            cmd.timestamp.format("%H:%M:%S"),
                            cmd_type,
                            status,
                            cmd.duration_ms
                        );
                    }
                }
            }
            "clear" => {
                executor.clear_history();
                println!("Command history cleared");
            }
            _ => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    parts[0]
                );
            }
        }
        println!();
    }

    println!("Goodbye!");
    Ok(())
}

fn resolve_reader_name(reader: &PcscReader, name_or_index: &str) -> Result<String> {
    // Try to parse as index first
    if let Ok(index) = name_or_index.parse::<usize>() {
        let readers = reader.list_readers()?;
        if index < readers.len() {
            return Ok(readers[index].name.clone());
        } else {
            bail!(
                "Reader index {} out of range (0-{})",
                index,
                readers.len() - 1
            );
        }
    }

    // Use as reader name directly
    Ok(name_or_index.to_string())
}

fn select_reader_interactive(reader: &PcscReader) -> Result<String> {
    let readers = reader.list_readers().context("Failed to list readers")?;

    if readers.is_empty() {
        bail!("No PCSC readers found");
    }

    println!("Available readers:");
    for (i, reader_info) in readers.iter().enumerate() {
        let status = if reader_info.is_connected {
            " [CARD]"
        } else {
            ""
        };
        println!("  [{}] {}{}", i, reader_info.name, status);
    }

    print!("Select reader [0-{}]: ", readers.len() - 1);
    io::Write::flush(&mut io::stdout()).unwrap();

    let stdin = io::stdin();
    let mut input = String::new();
    stdin
        .read_line(&mut input)
        .context("Failed to read input")?;

    let index: usize = input.trim().parse().context("Invalid reader index")?;

    if index >= readers.len() {
        bail!("Reader index {} out of range", index);
    }

    Ok(readers[index].name.clone())
}

fn print_response(data: &[u8], format: &ResponseFormat) {
    if data.is_empty() {
        println!("Response: (empty)");
        return;
    }

    match format {
        ResponseFormat::Hex => {
            println!("Response: {}", hex::encode_upper(data));
        }
        ResponseFormat::HexSpaced => {
            println!("Response: {}", format_hex_spaced(data));
        }
        ResponseFormat::HexDump => {
            println!("Response:");
            println!("{}", format_hex_dump(data));
        }
        ResponseFormat::Ascii => {
            println!("Response (ASCII): {}", format_ascii(data));
        }
        ResponseFormat::All => {
            println!("Response (Hex): {}", format_hex_spaced(data));
            println!("Response (ASCII): {}", format_ascii(data));
            if data.len() > 16 {
                println!("Response (Dump):");
                println!("{}", format_hex_dump(data));
            }
        }
    }
}
