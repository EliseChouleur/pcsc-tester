# PCSC Tester

Cross-platform PCSC tool for testing smart card readers with both GUI and CLI interfaces.

## Features

- **Cross-platform**: Works on macOS, Windows, and Linux
- **Dual interface**: GUI and command-line interfaces
- **PCSC commands**: Support for both transmit (APDU) and control commands
- **Multiple readers**: List and select from available PCSC readers
- **Command history**: Track and export command history
- **Flexible input**: Parse hex strings in various formats
- **Smart formatting**: Multiple output formats (hex, ASCII, hex dump)
- **Interactive mode**: Real-time PCSC testing
- **Script support**: Execute command sequences from files

## Installation

### Prerequisites

- PC/SC-Lite installed on your system:
  - **macOS**: Included in the system
  - **Linux**: Install `pcscd` package (`apt install pcscd` or equivalent)
  - **Windows**: Install PC/SC drivers for your reader

### Building from source

```bash
cd tools/pcsc-tester
cargo build --release
```

The executable will be available at `target/release/pcsc-tester`.

## Usage

### GUI Mode (Default)

Launch the graphical interface:

```bash
pcsc-tester
# or explicitly
pcsc-tester --gui
```

The GUI provides:
- Reader selection dropdown
- Connection controls with share mode options
- APDU command input with syntax highlighting
- Control command interface
- Response display with multiple format options
- Command history browser
- Settings panel

### CLI Mode

Use command-line interface with subcommands:

#### List readers

```bash
pcsc-tester list              # Shows ATR when card is present
pcsc-tester list --detailed   # Show detailed status and ATR
```

Example output:
```
Available PCSC readers:
  [0] My Reader [CARD - ATR: 3B AC 00 40 2A 00 12 25 00 64 80 00 03 10 00 90 00]
  [1] Another Reader
```

#### Send APDU commands

```bash
# Send APDU to first reader
pcsc-tester transmit 0 "00A40400"

# Send APDU with spaced hex
pcsc-tester transmit "Reader Name" "00 A4 04 00"

# Use exclusive mode
pcsc-tester transmit 0 "00A40400" --mode exclusive

# Different output formats
pcsc-tester transmit 0 "00A40400" --format dump
```

#### Send control commands

```bash
# Send control command (hex code)
pcsc-tester control 0 0x42000C00

# Send control with data
pcsc-tester control 0 0x42000C00 "1234ABCD"

# Use decimal code
pcsc-tester control 0 1107296256

# Direct mode for reader control
pcsc-tester control 0 0x42000C00 --mode direct
```

#### Script mode

Create a script file with commands:

```
# Example script.txt
transmit 00A40400
transmit 00B0000010
control 0x42000C00 1234
```

Execute the script:

```bash
pcsc-tester script script.txt 0
pcsc-tester script script.txt 0 --continue-on-error
```

#### Interactive mode

```bash
pcsc-tester interactive
# or specify reader
pcsc-tester interactive 0
```

Interactive commands:
- `transmit <apdu>` - Send APDU
- `control <code> [data]` - Send control command
- `history` - Show command history
- `clear` - Clear history
- `help` - Show help
- `quit` - Exit

## Input Formats

### Hex strings
The tool accepts hex strings in various formats:

```
0102030A           # Pure hex
01 02 03 0A        # Space-separated
0x01,0x02,0x03,0x0A  # 0x prefix with commas
01:02:03:0A        # Colon-separated
01-02-03-0A        # Dash-separated
```

### Control codes
Control codes can be specified as:

```
0x42000C00         # Hex with 0x prefix
42000C00           # Pure hex (if >3 chars)
1107296256         # Decimal
```

## Output Formats

### CLI Output formats
- `hex`: Raw hex (0102030A)
- `spaced`: Spaced hex (01 02 03 0A)
- `dump`: Hex dump with ASCII
- `ascii`: ASCII representation
- `all`: All formats combined

### Status word interpretation
The tool automatically interprets ISO 7816 status words:

```
90 00 → Success
6A 82 → Error: File not found
61 10 → Success, 16 bytes available
```

## Examples

### Basic APDU communication

```bash
# List readers
pcsc-tester list

# Select master file
pcsc-tester transmit 0 "00A4003C00"

# Get response
pcsc-tester transmit 0 "00B0000000"
```

### Reader control

```bash
# Get firmware version (example for specific reader)
pcsc-tester control 0 0x42000C00 --mode direct

# Get reader features
pcsc-tester control 0 0x42000D48 --mode direct
```

### Script automation

```bash
# Create test script
echo "transmit 00A40400" > test.txt
echo "transmit 00B0000010" >> test.txt

# Execute script
pcsc-tester script test.txt 0 --verbose
```

## Development

### Project structure

```
src/
├── main.rs           # Entry point (CLI vs GUI)
├── lib.rs            # Library exports
├── cli/              # CLI interface
│   ├── mod.rs
│   └── commands.rs   # Command implementations
├── gui/              # GUI interface
│   ├── mod.rs
│   └── app.rs        # egui application
└── core/             # Core PCSC logic
    ├── mod.rs
    ├── reader.rs     # Reader management
    ├── commands.rs   # Command execution
    └── utils.rs      # Utilities (hex parsing, etc.)
```

### Running tests

```bash
cargo test
```

### Building for release

```bash
cargo build --release
```

The optimized binary will be in `target/release/pcsc-tester`.

## Troubleshooting

### "No readers found"
- Check that PC/SC daemon is running (`pcscd` on Linux)
- Verify reader is properly connected and recognized by the system
- On macOS, check System Information → USB for reader presence

### "Failed to establish context"
- Ensure PC/SC is installed and running
- Check user permissions for PC/SC access
- On Linux, ensure user is in `plugdev` group if required

### "Failed to connect to reader"
- Another application might be using the reader exclusively
- Try different share modes (shared/exclusive/direct)
- Check if card is properly inserted

### GUI not starting
- Ensure graphics drivers are properly installed
- Try running with `RUST_LOG=debug` for more information

## Cross-platform builds

### macOS
```bash
cargo build --release --target x86_64-apple-darwin      # Intel
cargo build --release --target aarch64-apple-darwin     # Apple Silicon
```

### Windows
```bash
cargo build --release --target x86_64-pc-windows-gnu
```

### Linux
```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

## License

MIT License - see LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## Changelog

### v0.1.1
- Updated `list` command to display ATR when card is present
- Improved output formatting for better readability

### v0.1.0
- Initial release
- CLI and GUI interfaces
- Support for transmit and control commands
- Cross-platform compatibility
- Command history and scripting support