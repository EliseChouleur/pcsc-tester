/// Integration tests for the CLI interface
use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::io::Write;
use std::process::Command as StdCommand;
use tempfile::NamedTempFile;

/// Helper function to create a command for testing
fn pcsc_cmd() -> Command {
    Command::cargo_bin("pcsc-tester").expect("Failed to find pcsc-tester binary")
}

#[test]
fn test_help_command() {
    let mut cmd = pcsc_cmd();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cross-platform PCSC tool"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("transmit"))
        .stdout(predicate::str::contains("control"));
}

#[test]
fn test_version_command() {
    let mut cmd = pcsc_cmd();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("pcsc-tester"));
}

#[test]
fn test_list_command_basic() {
    let mut cmd = pcsc_cmd();
    cmd.arg("list").assert().success().stdout(
        predicate::str::contains("readers").or(predicate::str::contains("No PCSC readers found")),
    );
}

#[test]
fn test_list_command_detailed() {
    let mut cmd = pcsc_cmd();
    cmd.arg("list").arg("--detailed").assert().success();
}

#[test]
fn test_invalid_command() {
    let mut cmd = pcsc_cmd();
    cmd.arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_transmit_without_args() {
    let mut cmd = pcsc_cmd();
    cmd.arg("transmit")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_transmit_invalid_reader() {
    let mut cmd = pcsc_cmd();
    cmd.arg("transmit")
        .arg("999") // Invalid reader index
        .arg("00A40400")
        .assert()
        .failure();
}

#[test]
fn test_transmit_invalid_hex() {
    let mut cmd = pcsc_cmd();
    cmd.arg("transmit")
        .arg("0") // First reader (may not exist)
        .arg("invalid-hex")
        .assert()
        .failure();
}

#[test]
fn test_control_without_args() {
    let mut cmd = pcsc_cmd();
    cmd.arg("control")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_control_invalid_code() {
    let mut cmd = pcsc_cmd();
    cmd.arg("control")
        .arg("0")
        .arg("invalid-code")
        .assert()
        .failure();
}

#[test]
fn test_script_nonexistent_file() {
    let mut cmd = pcsc_cmd();
    cmd.arg("script")
        .arg("nonexistent-file.txt")
        .arg("0")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to open"));
}

#[test]
fn test_script_valid_file() {
    // Create a temporary script file
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(temp_file, "# Test script").expect("Failed to write to temp file");
    writeln!(temp_file, "transmit 00A40400").expect("Failed to write to temp file");

    let mut cmd = pcsc_cmd();
    cmd.arg("script")
        .arg(temp_file.path())
        .arg("0") // May fail if no reader exists, but should parse the file
        .assert()
        .code(predicate::in_iter([0, 1])); // Allow both success and failure
}

#[test]
fn test_script_empty_file() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");

    let mut cmd = pcsc_cmd();
    cmd.arg("script")
        .arg(temp_file.path())
        .arg("0")
        .assert()
        .success()
        .stdout(predicate::str::contains("Script execution completed"));
}

#[test]
fn test_verbose_flag() {
    let mut cmd = pcsc_cmd();
    cmd.arg("--verbose").arg("list").assert().success();
}

#[test]
fn test_debug_flag() {
    let mut cmd = pcsc_cmd();
    cmd.arg("--debug").arg("list").assert().success();
}

#[test]
fn test_transmit_different_formats() {
    let mut cmd = pcsc_cmd();
    cmd.arg("transmit")
        .arg("0")
        .arg("00A40400")
        .arg("--format")
        .arg("hex")
        .assert()
        .code(predicate::in_iter([0, 1])); // May fail if no reader

    let mut cmd2 = pcsc_cmd();
    cmd2.arg("transmit")
        .arg("0")
        .arg("00A40400")
        .arg("--format")
        .arg("dump")
        .assert()
        .code(predicate::in_iter([0, 1]));
}

#[test]
fn test_transmit_different_modes() {
    let mut cmd = pcsc_cmd();
    cmd.arg("transmit")
        .arg("0")
        .arg("00A40400")
        .arg("--mode")
        .arg("shared")
        .assert()
        .code(predicate::in_iter([0, 1]));

    let mut cmd2 = pcsc_cmd();
    cmd2.arg("transmit")
        .arg("0")
        .arg("00A40400")
        .arg("--mode")
        .arg("exclusive")
        .assert()
        .code(predicate::in_iter([0, 1]));
}

#[test]
#[ignore] // Requires real PCSC hardware - will panic without physical reader
fn test_control_different_modes() {
    // This test requires actual PCSC hardware with a connected card reader
    // It will fail in CI environments or systems without PCSC hardware
    let mut cmd = pcsc_cmd();
    cmd.arg("control")
        .arg("0")
        .arg("0x42000C00")
        .arg("--mode")
        .arg("direct")
        .assert()
        .code(predicate::in_iter([0, 1]));
}

#[test]
#[ignore] // Requires real PCSC hardware and interactive input - will timeout/panic
fn test_interactive_help() {
    // This test requires actual PCSC hardware with a connected card reader
    // AND interactive input from stdin, making it unsuitable for automated testing
    // It will timeout or panic in CI environments or systems without PCSC hardware
    let mut cmd = pcsc_cmd();
    cmd.arg("interactive")
        .arg("0")
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .code(predicate::in_iter([0, 1, 124])); // 124 is timeout code
}

#[test]
fn test_complex_script() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(temp_file, "# Complex test script").expect("Failed to write");
    writeln!(temp_file, "transmit 00A40400").expect("Failed to write");
    writeln!(temp_file, "transmit 00B0000010").expect("Failed to write");
    writeln!(temp_file, "control 0x42000C00").expect("Failed to write");
    writeln!(temp_file, "control 1234 ABCD").expect("Failed to write");

    let mut cmd = pcsc_cmd();
    cmd.arg("script")
        .arg(temp_file.path())
        .arg("0")
        .arg("--continue-on-error")
        .assert()
        .code(predicate::in_iter([0, 1]));
}

#[test]
fn test_invalid_format() {
    let mut cmd = pcsc_cmd();
    cmd.arg("transmit")
        .arg("0")
        .arg("00A40400")
        .arg("--format")
        .arg("invalid-format")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn test_invalid_mode() {
    let mut cmd = pcsc_cmd();
    cmd.arg("transmit")
        .arg("0")
        .arg("00A40400")
        .arg("--mode")
        .arg("invalid-mode")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

// Helper function for checking if PCSC is available
fn is_pcsc_available() -> bool {
    StdCommand::new("pcscd").arg("--version").output().is_ok()
}

/// Test that requires actual PCSC readers (conditional)
#[test]
#[serial]
fn test_with_real_readers() {
    if !is_pcsc_available() {
        println!("Skipping PCSC integration test - no PCSC daemon available");
        return;
    }

    // This test will only run if PCSC is available
    let mut cmd = pcsc_cmd();
    let output = cmd.arg("list").assert().success();

    // If we have readers, try a basic command
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    if stdout.contains("[0]") && stdout.contains("CARD") {
        // We have a reader with a card, try a simple command
        let mut transmit_cmd = pcsc_cmd();
        transmit_cmd
            .arg("transmit")
            .arg("0")
            .arg("00A40400") // SELECT command
            .timeout(std::time::Duration::from_secs(5))
            .assert()
            .code(predicate::in_iter([0, 1])); // Allow success or failure
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_help_performance() {
        let start = Instant::now();
        let mut cmd = pcsc_cmd();
        cmd.arg("--help").assert().success();
        let duration = start.elapsed();

        // Help should be fast (less than 1 second)
        assert!(
            duration.as_secs() < 1,
            "Help command took too long: {duration:?}"
        );
    }

    #[test]
    fn test_list_performance() {
        let start = Instant::now();
        let mut cmd = pcsc_cmd();
        cmd.arg("list").assert().success();
        let duration = start.elapsed();

        // List should complete within reasonable time (less than 5 seconds)
        assert!(
            duration.as_secs() < 5,
            "List command took too long: {duration:?}"
        );
    }
}
