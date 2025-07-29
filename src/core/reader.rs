use pcsc::{Context, Scope, ShareMode, Protocols, Card};
use anyhow::{Result, Context as AnyhowContext};
use serde::{Deserialize, Serialize};
use std::ffi::CString;

/// Information about a PCSC reader
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderInfo {
    pub name: String,
    pub is_connected: bool,
    pub atr: Option<Vec<u8>>,
}

/// PCSC reader manager
pub struct PcscReader {
    context: Context,
    current_reader: Option<String>,
    current_card: Option<Card>,
}

impl PcscReader {
    /// Create a new PCSC reader manager
    pub fn new() -> Result<Self> {
        let context = Context::establish(Scope::User)
            .context("Failed to establish PCSC context")?;
        
        Ok(Self {
            context,
            current_reader: None,
            current_card: None,
        })
    }

    /// List all available readers
    pub fn list_readers(&self) -> Result<Vec<ReaderInfo>> {
        let mut readers_buf = vec![0; 2048];
        let readers = self.context.list_readers(&mut readers_buf)
            .context("Failed to list readers")?;

        let mut reader_infos = Vec::new();
        
        for reader_name in readers {
            let reader_name_str = reader_name.to_string_lossy().to_string();
            
            // Try to get reader status to check if card is present
            let (is_connected, atr) = self.get_reader_status(&reader_name_str)
                .unwrap_or((false, None));
            
            reader_infos.push(ReaderInfo {
                name: reader_name_str,
                is_connected,
                atr,
            });
        }

        Ok(reader_infos)
    }

    /// Get the status of a specific reader
    fn get_reader_status(&self, reader_name: &str) -> Result<(bool, Option<Vec<u8>>)> {
        // Convert to CString for PCSC API
        let reader_cstr = CString::new(reader_name).context("Invalid reader name")?;
        
        // Try to connect to get status
        match self.context.connect(&reader_cstr, ShareMode::Shared, Protocols::ANY) {
            Ok(card) => {
                // Get ATR if available
                match card.status2_owned() {
                    Ok(status) => Ok((true, Some(status.atr().to_vec()))),
                    Err(_) => Ok((true, None)),
                }
            }
            Err(pcsc::Error::NoSmartcard) => Ok((false, None)),
            Err(_) => Ok((false, None)),
        }
    }

    /// Connect to a specific reader
    pub fn connect(&mut self, reader_name: &str, share_mode: ShareMode) -> Result<()> {
        log::info!("Connecting to reader: {}", reader_name);
        
        let reader_cstr = CString::new(reader_name).context("Invalid reader name")?;
        let card = self.context.connect(&reader_cstr, share_mode, Protocols::ANY)
            .with_context(|| format!("Failed to connect to reader: {}", reader_name))?;
        
        self.current_reader = Some(reader_name.to_string());
        self.current_card = Some(card);
        
        log::info!("Successfully connected to reader: {}", reader_name);
        Ok(())
    }

    /// Disconnect from current reader
    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(card) = self.current_card.take() {
            let _ = card.disconnect(pcsc::Disposition::LeaveCard)
                .map_err(|_e| log::warn!("Failed to disconnect cleanly from card"));
        }
        
        self.current_reader = None;
        log::info!("Disconnected from reader");
        Ok(())
    }

    /// Get the currently connected reader name
    pub fn current_reader(&self) -> Option<&str> {
        self.current_reader.as_deref()
    }

    /// Check if connected to a reader
    pub fn is_connected(&self) -> bool {
        self.current_card.is_some()
    }

    /// Get the current card handle (for commands)
    pub fn card(&self) -> Option<&Card> {
        self.current_card.as_ref()
    }

    /// Get mutable card handle (for commands)
    pub fn card_mut(&mut self) -> Option<&mut Card> {
        self.current_card.as_mut()
    }

    /// Get reader information for the currently connected reader
    pub fn current_reader_info(&self) -> Result<Option<ReaderInfo>> {
        if let Some(reader_name) = &self.current_reader {
            let (is_connected, atr) = self.get_reader_status(reader_name)?;
            Ok(Some(ReaderInfo {
                name: reader_name.clone(),
                is_connected,
                atr,
            }))
        } else {
            Ok(None)
        }
    }
}

impl Drop for PcscReader {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_info_creation() {
        let reader_info = ReaderInfo {
            name: "Test Reader".to_string(),
            is_connected: true,
            atr: Some(vec![0x3B, 0x75, 0x13, 0x00]),
        };

        assert_eq!(reader_info.name, "Test Reader");
        assert!(reader_info.is_connected);
        assert_eq!(reader_info.atr, Some(vec![0x3B, 0x75, 0x13, 0x00]));
    }

    #[test]
    fn test_reader_info_serialization() {
        let reader_info = ReaderInfo {
            name: "Test Reader".to_string(),
            is_connected: false,
            atr: None,
        };

        // Test JSON serialization
        let json = serde_json::to_string(&reader_info).unwrap();
        assert!(json.contains("Test Reader"));
        assert!(json.contains("false"));

        // Test deserialization
        let deserialized: ReaderInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, reader_info.name);
        assert_eq!(deserialized.is_connected, reader_info.is_connected);
        assert_eq!(deserialized.atr, reader_info.atr);
    }

    #[test]
    fn test_reader_info_with_atr() {
        let atr = vec![0x3B, 0xAC, 0x00, 0x40, 0x2A, 0x00, 0x12, 0x25, 0x00, 0x64, 0x80, 0x00, 0x03, 0x10, 0x00, 0x90, 0x00];
        let reader_info = ReaderInfo {
            name: "Smart Card Reader".to_string(), 
            is_connected: true,
            atr: Some(atr.clone()),
        };

        assert!(reader_info.is_connected);
        assert_eq!(reader_info.atr.unwrap(), atr);
    }

    // Note: Testing PcscReader methods that interact with actual PCSC hardware
    // would require either:
    // 1. Real hardware (not practical for unit tests)
    // 2. Mocking the PCSC library (would require significant refactoring)
    // 3. A test-only PCSC implementation
    //
    // For now, we test the data structures and would rely on integration tests
    // for actual PCSC functionality.

    #[test] 
    fn test_reader_info_clone() {
        let original = ReaderInfo {
            name: "Original Reader".to_string(),
            is_connected: true,
            atr: Some(vec![0x3B, 0x00]),
        };

        let cloned = original.clone();
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.is_connected, cloned.is_connected);
        assert_eq!(original.atr, cloned.atr);
    }

    #[test]
    fn test_reader_info_debug() {
        let reader_info = ReaderInfo {
            name: "Debug Reader".to_string(),
            is_connected: false,
            atr: None,
        };

        let debug_str = format!("{:?}", reader_info);
        assert!(debug_str.contains("Debug Reader"));
        assert!(debug_str.contains("false"));
        assert!(debug_str.contains("None"));
    }

    // Test edge cases
    #[test]
    fn test_reader_info_empty_name() {
        let reader_info = ReaderInfo {
            name: "".to_string(),
            is_connected: false,
            atr: None,
        };

        assert_eq!(reader_info.name, "");
        assert!(!reader_info.is_connected);
        assert!(reader_info.atr.is_none());
    }

    #[test]
    fn test_reader_info_long_atr() {
        // Test with a long ATR (maximum is typically 33 bytes)
        let long_atr: Vec<u8> = (0..33).collect();
        let reader_info = ReaderInfo {
            name: "Long ATR Reader".to_string(),
            is_connected: true,
            atr: Some(long_atr.clone()),
        };

        assert_eq!(reader_info.atr.unwrap().len(), 33);
    }

    #[test]
    fn test_reader_info_empty_atr() {
        let reader_info = ReaderInfo {
            name: "Empty ATR Reader".to_string(),
            is_connected: true,
            atr: Some(vec![]),
        };

        assert!(reader_info.is_connected);
        assert_eq!(reader_info.atr.unwrap().len(), 0);
    }
}