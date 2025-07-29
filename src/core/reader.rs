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