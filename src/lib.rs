/// PCSC Tester - Cross-platform tool for testing smart card readers
///
/// This library provides both CLI and GUI interfaces for interacting with
/// PCSC-compatible smart card readers.
pub mod cli;
pub mod core;
pub mod gui;

// Re-export commonly used types
pub use core::{
    commands::{CommandResult, ControlResult, TransmitResult},
    reader::{PcscReader, ReaderInfo},
    utils::{format_hex, parse_hex},
};

// Common error type
pub type Result<T> = anyhow::Result<T>;
