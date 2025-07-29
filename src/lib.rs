/// PCSC Tester - Cross-platform tool for testing smart card readers
/// 
/// This library provides both CLI and GUI interfaces for interacting with 
/// PCSC-compatible smart card readers.

pub mod cli;
pub mod gui;
pub mod core;

// Re-export commonly used types
pub use core::{
    reader::{PcscReader, ReaderInfo},
    commands::{CommandResult, TransmitResult, ControlResult},
    utils::{parse_hex, format_hex},
};

// Common error type
pub type Result<T> = anyhow::Result<T>;