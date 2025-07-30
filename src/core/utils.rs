use anyhow::{bail, Context, Result};

/// Parse a hex string into bytes
/// Supports various formats:
/// - "0102030A" (pure hex)
/// - "01 02 03 0A" (space-separated)
/// - "0x01,0x02,0x03,0x0A" (0x prefix with commas)
/// - "01:02:03:0A" (colon-separated)
pub fn parse_hex(hex_str: &str) -> Result<Vec<u8>> {
    let cleaned = clean_hex_string(hex_str);

    if cleaned.is_empty() {
        return Ok(Vec::new());
    }

    if cleaned.len() % 2 != 0 {
        bail!(
            "Hex string must have even number of characters: '{}'",
            hex_str
        );
    }

    hex::decode(&cleaned).with_context(|| format!("Invalid hex string: '{hex_str}'"))
}

/// Clean a hex string by removing common separators and prefixes
fn clean_hex_string(hex_str: &str) -> String {
    hex_str
        .trim()
        .replace("0x", "")
        .replace("0X", "")
        .replace(" ", "")
        .replace(",", "")
        .replace(":", "")
        .replace("-", "")
        .replace("\t", "")
        .replace("\n", "")
        .replace("\r", "")
        .to_uppercase()
}

/// Format bytes as a hex string
pub fn format_hex(bytes: &[u8]) -> String {
    hex::encode_upper(bytes)
}

/// Format bytes as a hex string with spaces
pub fn format_hex_spaced(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format bytes as a hex string with 0x prefix
#[allow(dead_code)]
pub fn format_hex_prefixed(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    bytes
        .iter()
        .map(|b| format!("0x{b:02X}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format bytes as ASCII, replacing non-printable chars with '.'
pub fn format_ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

/// Format bytes in a hex dump style (both hex and ASCII)
pub fn format_hex_dump(bytes: &[u8]) -> String {
    const BYTES_PER_LINE: usize = 16;

    if bytes.is_empty() {
        return String::from("(empty)");
    }

    let mut result = String::new();

    for (i, chunk) in bytes.chunks(BYTES_PER_LINE).enumerate() {
        // Address
        result.push_str(&format!("{:08X}: ", i * BYTES_PER_LINE));

        // Hex bytes
        for (j, &byte) in chunk.iter().enumerate() {
            result.push_str(&format!("{byte:02X} "));
            if j == 7 {
                result.push(' '); // Extra space in the middle
            }
        }

        // Padding for incomplete lines
        let padding_needed = (BYTES_PER_LINE - chunk.len()) * 3;
        if chunk.len() <= 8 {
            result.push(' '); // Account for middle space
        }
        result.push_str(&" ".repeat(padding_needed));

        // ASCII representation
        result.push_str(" |");
        for &byte in chunk {
            if byte.is_ascii_graphic() || byte == b' ' {
                result.push(byte as char);
            } else {
                result.push('.');
            }
        }
        result.push('|');
        result.push('\n');
    }

    result.trim_end().to_string()
}

/// Parse a control code from various formats
pub fn parse_control_code(code_str: &str) -> Result<u32> {
    let cleaned = code_str.trim();

    if cleaned.starts_with("0x") || cleaned.starts_with("0X") {
        u32::from_str_radix(&cleaned[2..], 16)
            .with_context(|| format!("Invalid hex control code: '{code_str}'"))
    } else if cleaned.chars().all(|c| c.is_ascii_hexdigit()) && cleaned.len() > 3 {
        // Assume hex if it looks like hex
        u32::from_str_radix(cleaned, 16)
            .with_context(|| format!("Invalid hex control code: '{code_str}'"))
    } else {
        cleaned
            .parse::<u32>()
            .with_context(|| format!("Invalid decimal control code: '{code_str}'"))
    }
}

/// Validate that a hex string is properly formatted
pub fn validate_hex_string(hex_str: &str) -> Result<()> {
    let cleaned = clean_hex_string(hex_str);

    if cleaned.len() % 2 != 0 {
        bail!("Hex string must have even number of characters");
    }

    for c in cleaned.chars() {
        if !c.is_ascii_hexdigit() {
            bail!("Invalid hex character: '{}'", c);
        }
    }

    Ok(())
}

/// Check if a string looks like a hex string
#[allow(dead_code)]
pub fn is_hex_like(s: &str) -> bool {
    let cleaned = clean_hex_string(s);
    !cleaned.is_empty() && cleaned.chars().all(|c| c.is_ascii_hexdigit()) && cleaned.len() % 2 == 0
}

/// Get a human-readable description of SW1/SW2 status words
pub fn describe_status_word(sw1: u8, sw2: u8) -> String {
    match (sw1, sw2) {
        (0x90, 0x00) => "Success".to_string(),
        (0x61, n) => format!("Success, {n} bytes available"),
        (0x62, 0x00) => "Warning: No information given".to_string(),
        (0x62, 0x81) => "Warning: Part of returned data may be corrupted".to_string(),
        (0x62, 0x82) => "Warning: End of file reached".to_string(),
        (0x62, 0x83) => "Warning: Selected file invalidated".to_string(),
        (0x62, 0x84) => "Warning: FCI not formatted".to_string(),
        (0x63, 0x00) => "Warning: No information given".to_string(),
        (0x63, n) if n & 0xF0 == 0xC0 => format!("Warning: Counter = {}", n & 0x0F),
        (0x64, 0x00) => "Error: Execution error".to_string(),
        (0x65, 0x00) => "Error: No precise diagnosis".to_string(),
        (0x65, 0x81) => "Error: Memory failure".to_string(),
        (0x66, 0x00) => "Error: Reserved".to_string(),
        (0x67, 0x00) => "Error: Wrong length".to_string(),
        (0x68, 0x00) => "Error: Functions in CLA not supported".to_string(),
        (0x68, 0x81) => "Error: Logical channel not supported".to_string(),
        (0x68, 0x82) => "Error: Secure messaging not supported".to_string(),
        (0x69, 0x00) => "Error: Command not allowed".to_string(),
        (0x69, 0x81) => "Error: Command incompatible with file structure".to_string(),
        (0x69, 0x82) => "Error: Security status not satisfied".to_string(),
        (0x69, 0x83) => "Error: Authentication method blocked".to_string(),
        (0x69, 0x84) => "Error: Referenced data invalidated".to_string(),
        (0x69, 0x85) => "Error: Conditions of use not satisfied".to_string(),
        (0x69, 0x86) => "Error: Command not allowed (no current EF)".to_string(),
        (0x69, 0x87) => "Error: Expected SM data objects missing".to_string(),
        (0x69, 0x88) => "Error: SM data objects incorrect".to_string(),
        (0x6A, 0x00) => "Error: Wrong parameter(s) P1-P2".to_string(),
        (0x6A, 0x80) => "Error: Incorrect parameters in data field".to_string(),
        (0x6A, 0x81) => "Error: Function not supported".to_string(),
        (0x6A, 0x82) => "Error: File not found".to_string(),
        (0x6A, 0x83) => "Error: Record not found".to_string(),
        (0x6A, 0x84) => "Error: Not enough memory space in file".to_string(),
        (0x6A, 0x85) => "Error: Lc inconsistent with TLV structure".to_string(),
        (0x6A, 0x86) => "Error: Incorrect parameters P1-P2".to_string(),
        (0x6A, 0x87) => "Error: Lc inconsistent with P1-P2".to_string(),
        (0x6A, 0x88) => "Error: Referenced data not found".to_string(),
        (0x6B, 0x00) => "Error: Wrong parameter(s) P1-P2".to_string(),
        (0x6C, n) => format!("Error: Wrong Le field, exact length: {n}"),
        (0x6D, 0x00) => "Error: Instruction code not supported or invalid".to_string(),
        (0x6E, 0x00) => "Error: Class not supported".to_string(),
        (0x6F, 0x00) => "Error: No precise diagnosis".to_string(),
        _ => format!("Unknown status: {sw1:02X} {sw2:02X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_various_formats() {
        assert_eq!(parse_hex("0102030A").unwrap(), vec![0x01, 0x02, 0x03, 0x0A]);
        assert_eq!(
            parse_hex("01 02 03 0A").unwrap(),
            vec![0x01, 0x02, 0x03, 0x0A]
        );
        assert_eq!(
            parse_hex("0x01,0x02,0x03,0x0A").unwrap(),
            vec![0x01, 0x02, 0x03, 0x0A]
        );
        assert_eq!(
            parse_hex("01:02:03:0A").unwrap(),
            vec![0x01, 0x02, 0x03, 0x0A]
        );
        assert_eq!(
            parse_hex("01-02-03-0A").unwrap(),
            vec![0x01, 0x02, 0x03, 0x0A]
        );
        assert_eq!(parse_hex("").unwrap(), Vec::<u8>::new());
        assert_eq!(parse_hex("   ").unwrap(), Vec::<u8>::new());
        assert_eq!(
            parse_hex("\t01\n02\r03\n0A\t").unwrap(),
            vec![0x01, 0x02, 0x03, 0x0A]
        );
    }

    #[test]
    fn test_parse_hex_case_insensitive() {
        assert_eq!(parse_hex("abcdef").unwrap(), vec![0xAB, 0xCD, 0xEF]);
        assert_eq!(parse_hex("ABCDEF").unwrap(), vec![0xAB, 0xCD, 0xEF]);
        assert_eq!(parse_hex("aBcDeF").unwrap(), vec![0xAB, 0xCD, 0xEF]);
    }

    #[test]
    fn test_parse_hex_invalid() {
        assert!(parse_hex("0102030").is_err()); // Odd length
        assert!(parse_hex("0102G30A").is_err()); // Invalid hex character
        assert!(parse_hex("01@02").is_err()); // Invalid character
        assert!(parse_hex("Z").is_err()); // Invalid hex
    }

    #[test]
    fn test_format_functions() {
        let bytes = vec![0x01, 0x02, 0x03, 0x0A];
        assert_eq!(format_hex(&bytes), "0102030A");
        assert_eq!(format_hex_spaced(&bytes), "01 02 03 0A");
        assert_eq!(format_hex_prefixed(&bytes), "0x01, 0x02, 0x03, 0x0A");

        // Test empty bytes
        assert_eq!(format_hex(&[]), "");
        assert_eq!(format_hex_spaced(&[]), "");
        assert_eq!(format_hex_prefixed(&[]), "");

        // Test single byte
        assert_eq!(format_hex(&[0xFF]), "FF");
        assert_eq!(format_hex_spaced(&[0xFF]), "FF");
        assert_eq!(format_hex_prefixed(&[0xFF]), "0xFF");
    }

    #[test]
    fn test_format_ascii() {
        assert_eq!(format_ascii(b"Hello"), "Hello");
        assert_eq!(format_ascii(&[0x48, 0x65, 0x6C, 0x6C, 0x6F]), "Hello");
        assert_eq!(format_ascii(&[0x00, 0x01, 0x02, 0x20, 0x7F]), "... .");
        assert_eq!(format_ascii(&[]), "");
    }

    #[test]
    fn test_format_hex_dump() {
        let bytes = vec![
            0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64,
        ];
        let dump = format_hex_dump(&bytes);
        assert!(dump.contains("48 65 6C 6C 6F 20 57 6F"));
        assert!(dump.contains("|Hello World|"));

        // Test empty
        assert_eq!(format_hex_dump(&[]), "(empty)");

        // Test multi-line
        let long_bytes: Vec<u8> = (0..32).collect();
        let long_dump = format_hex_dump(&long_bytes);
        assert!(long_dump.lines().count() >= 2);
    }

    #[test]
    fn test_parse_control_code() {
        assert_eq!(parse_control_code("0x1234").unwrap(), 0x1234);
        assert_eq!(parse_control_code("0X1234").unwrap(), 0x1234);
        assert_eq!(parse_control_code("1234").unwrap(), 0x1234); // 4+ chars treated as hex
        assert_eq!(parse_control_code("ABCD").unwrap(), 0xABCD);
        assert_eq!(parse_control_code("abcd").unwrap(), 0xABCD);
        assert_eq!(parse_control_code("42000C00").unwrap(), 0x42000C00);

        // Test decimal
        assert_eq!(parse_control_code("123").unwrap(), 123);
        assert_eq!(parse_control_code("0").unwrap(), 0);

        // Test invalid
        assert!(parse_control_code("").is_err());
        assert!(parse_control_code("invalid").is_err());
        assert!(parse_control_code("0xZZZZ").is_err());
    }

    #[test]
    fn test_validate_hex_string() {
        assert!(validate_hex_string("0102030A").is_ok());
        assert!(validate_hex_string("01 02 03 0A").is_ok());
        assert!(validate_hex_string("").is_ok());

        assert!(validate_hex_string("0102030").is_err()); // Odd length
        assert!(validate_hex_string("0102G30A").is_err()); // Invalid char
    }

    #[test]
    fn test_is_hex_like() {
        assert!(is_hex_like("0102030A"));
        assert!(is_hex_like("01 02 03 0A"));
        assert!(is_hex_like("ABCDEF"));
        assert!(!is_hex_like("")); // Empty string is not hex-like

        assert!(!is_hex_like("0102030")); // Odd length
        assert!(!is_hex_like("Hello"));
        assert!(!is_hex_like("0102G30A"));
    }

    #[test]
    fn test_describe_status_word() {
        assert_eq!(describe_status_word(0x90, 0x00), "Success");
        assert_eq!(
            describe_status_word(0x61, 0x10),
            "Success, 16 bytes available"
        );
        assert_eq!(describe_status_word(0x6A, 0x82), "Error: File not found");
        assert_eq!(
            describe_status_word(0x6F, 0x00),
            "Error: No precise diagnosis"
        );
        assert_eq!(describe_status_word(0x67, 0x00), "Error: Wrong length");
        assert_eq!(
            describe_status_word(0x6C, 0x08),
            "Error: Wrong Le field, exact length: 8"
        );
        assert_eq!(describe_status_word(0x63, 0xC3), "Warning: Counter = 3");

        // Unknown status
        assert_eq!(describe_status_word(0x12, 0x34), "Unknown status: 12 34");
    }

    #[test]
    fn test_clean_hex_string() {
        assert_eq!(clean_hex_string("0x01,0x02"), "0102");
        assert_eq!(clean_hex_string("01 02 03"), "010203");
        assert_eq!(clean_hex_string("01:02:03"), "010203");
        assert_eq!(clean_hex_string("01-02-03"), "010203");
        assert_eq!(clean_hex_string("\t01\n02\r"), "0102");
        assert_eq!(clean_hex_string("  0X01  "), "01");
    }

    #[test]
    fn test_edge_cases() {
        // Test very long hex strings
        let long_hex = "01".repeat(1000);
        let parsed = parse_hex(&long_hex).unwrap();
        assert_eq!(parsed.len(), 1000);
        assert!(parsed.iter().all(|&b| b == 0x01));

        // Test large control code (but not max which overflows)
        assert_eq!(parse_control_code("42000C00").unwrap(), 0x42000C00);

        // Test format with special bytes
        let special_bytes = vec![0x00, 0xFF, 0x7F, 0x80];
        assert_eq!(format_hex(&special_bytes), "00FF7F80");
        assert_eq!(format_hex_spaced(&special_bytes), "00 FF 7F 80");
    }
}
