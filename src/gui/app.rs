use eframe::egui;
use pcsc::ShareMode;
use anyhow::Result;
use std::sync::Arc;

use crate::core::{
    reader::{PcscReader, ReaderInfo},
    commands::{CommandExecutor, CommandType},
    utils::{format_hex_spaced, format_hex_dump, format_ascii, parse_control_code, validate_hex_string},
};

#[derive(Default)]
pub struct PcscTesterApp {
    // PCSC components
    pcsc_reader: Option<PcscReader>,
    command_executor: CommandExecutor,
    
    // Reader state
    available_readers: Vec<ReaderInfo>,
    selected_reader_idx: Option<usize>,
    connected_reader: Option<String>,
    connection_status: ConnectionStatus,
    
    // Command input
    command_input: String,
    control_code_input: String,
    control_data_input: String,
    share_mode: ShareModeGui,
    
    // Response display
    last_response: Vec<u8>,
    response_format: ResponseFormatGui,
    
    // UI state
    show_history: bool,
    show_settings: bool,
    auto_scroll_history: bool,
    max_history_items: usize,
    
    // Status messages
    status_message: String,
    error_message: String,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum ShareModeGui {
    #[default]
    Shared,
    Exclusive,
    Direct,
}

impl From<ShareModeGui> for ShareMode {
    fn from(mode: ShareModeGui) -> Self {
        match mode {
            ShareModeGui::Shared => ShareMode::Shared,
            ShareModeGui::Exclusive => ShareMode::Exclusive,
            ShareModeGui::Direct => ShareMode::Direct,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
enum ResponseFormatGui {
    #[default]
    HexSpaced,
    Hex,
    HexDump,
    Ascii,
}

impl PcscTesterApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Use default fonts (egui has good monospace support built-in)
        let _ctx = &cc.egui_ctx;
        
        let mut app = Self {
            max_history_items: 1000,
            auto_scroll_history: true,
            ..Default::default()
        };
        
        // Initialize PCSC
        app.initialize_pcsc();
        
        app
    }
    
    fn initialize_pcsc(&mut self) {
        match PcscReader::new() {
            Ok(reader) => {
                self.pcsc_reader = Some(reader);
                self.refresh_readers();
                self.status_message = "PCSC initialized".to_string();
            }
            Err(e) => {
                self.error_message = format!("Failed to initialize PCSC: {}", e);
                self.connection_status = ConnectionStatus::Error;
            }
        }
    }
    
    fn refresh_readers(&mut self) {
        if let Some(ref reader) = self.pcsc_reader {
            match reader.list_readers() {
                Ok(readers) => {
                    self.available_readers = readers;
                    if self.available_readers.is_empty() {
                        self.status_message = "No readers found".to_string();
                    } else {
                        self.status_message = format!("Found {} reader(s)", self.available_readers.len());
                    }
                }
                Err(e) => {
                    self.error_message = format!("Failed to list readers: {}", e);
                }
            }
        }
    }
    
    fn connect_to_reader(&mut self) {
        if let (Some(ref mut reader), Some(idx)) = (&mut self.pcsc_reader, self.selected_reader_idx) {
            if idx < self.available_readers.len() {
                let reader_name = &self.available_readers[idx].name;
                self.connection_status = ConnectionStatus::Connecting;
                
                match reader.connect(reader_name, self.share_mode.into()) {
                    Ok(()) => {
                        self.connected_reader = Some(reader_name.clone());
                        self.connection_status = ConnectionStatus::Connected;
                        self.status_message = format!("Connected to {}", reader_name);
                        self.error_message.clear();
                    }
                    Err(e) => {
                        self.connection_status = ConnectionStatus::Error;
                        self.error_message = format!("Failed to connect: {}", e);
                    }
                }
            }
        }
    }
    
    fn disconnect_from_reader(&mut self) {
        if let Some(ref mut reader) = &mut self.pcsc_reader {
            match reader.disconnect() {
                Ok(()) => {
                    self.connected_reader = None;
                    self.connection_status = ConnectionStatus::Disconnected;
                    self.status_message = "Disconnected".to_string();
                }
                Err(e) => {
                    self.error_message = format!("Disconnect error: {}", e);
                }
            }
        }
    }
    
    fn send_transmit_command(&mut self) {
        if let Some(ref mut reader) = &mut self.pcsc_reader {
            if self.connection_status == ConnectionStatus::Connected {
                // Validate hex input
                if let Err(e) = validate_hex_string(&self.command_input) {
                    self.error_message = format!("Invalid hex string: {}", e);
                    return;
                }
                
                match self.command_executor.transmit(reader, &self.command_input) {
                    Ok(result) => {
                        self.last_response = result.response.clone();
                        self.status_message = format!("Transmit successful - SW: {:02X} {:02X} ({}ms)", 
                                                       result.sw1, result.sw2, result.duration_ms);
                        self.error_message.clear();
                    }
                    Err(e) => {
                        self.error_message = format!("Transmit failed: {}", e);
                    }
                }
            } else {
                self.error_message = "Not connected to a reader".to_string();
            }
        }
    }
    
    fn send_control_command(&mut self) {
        if let Some(ref mut reader) = &mut self.pcsc_reader {
            if self.connection_status == ConnectionStatus::Connected {
                // Parse control code
                let code = match parse_control_code(&self.control_code_input) {
                    Ok(code) => code,
                    Err(e) => {
                        self.error_message = format!("Invalid control code: {}", e);
                        return;
                    }
                };
                
                // Validate data hex input if provided
                if !self.control_data_input.trim().is_empty() {
                    if let Err(e) = validate_hex_string(&self.control_data_input) {
                        self.error_message = format!("Invalid data hex string: {}", e);
                        return;
                    }
                }
                
                match self.command_executor.control(reader, code, &self.control_data_input) {
                    Ok(result) => {
                        self.last_response = result.output.clone();
                        self.status_message = format!("Control successful - Code: 0x{:X} ({}ms)", 
                                                       result.code, result.duration_ms);
                        self.error_message.clear();
                    }
                    Err(e) => {
                        self.error_message = format!("Control failed: {}", e);
                    }
                }
            } else {
                self.error_message = "Not connected to a reader".to_string();
            }
        }
    }
    
    fn format_response(&self, data: &[u8]) -> String {
        if data.is_empty() {
            return "(empty)".to_string();
        }
        
        match self.response_format {
            ResponseFormatGui::Hex => hex::encode_upper(data),
            ResponseFormatGui::HexSpaced => format_hex_spaced(data),
            ResponseFormatGui::HexDump => format_hex_dump(data),
            ResponseFormatGui::Ascii => format_ascii(data),
        }
    }
}

impl eframe::App for PcscTesterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Refresh Readers").clicked() {
                        self.refresh_readers();
                        ui.close_menu();
                    }
                    if ui.button("Export History").clicked() {
                        if let Ok(json) = self.command_executor.export_history() {
                            // In a real app, you'd use a file dialog here
                            log::info!("History exported: {} bytes", json.len());
                            self.status_message = "History exported to log".to_string();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_history, "Show History");
                    ui.checkbox(&mut self.show_settings, "Show Settings");
                });
                
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        log::info!("PCSC Tester - Cross-platform smart card reader testing tool");
                        ui.close_menu();
                    }
                });
            });
        });
        
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Status:");
                if !self.status_message.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(0, 150, 0), &self.status_message);
                }
                if !self.error_message.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(200, 0, 0), &self.error_message);
                }
            });
        });
        
        // Settings panel
        if self.show_settings {
            egui::Window::new("Settings")
                .open(&mut self.show_settings)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Max history items:");
                        ui.add(egui::DragValue::new(&mut self.max_history_items).range(10..=10000));
                    });
                    
                    ui.checkbox(&mut self.auto_scroll_history, "Auto-scroll history");
                    
                    if ui.button("Clear History").clicked() {
                        self.command_executor.clear_history();
                        self.status_message = "History cleared".to_string();
                    }
                });
        }
        
        // History panel
        if self.show_history {
            egui::Window::new("Command History")
                .open(&mut self.show_history)
                .default_width(600.0)
                .show(ctx, |ui| {
                    let history = self.command_executor.history();
                    
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for (i, cmd) in history.iter().enumerate() {
                                let status_color = if cmd.success {
                                    egui::Color32::from_rgb(0, 150, 0)
                                } else {
                                    egui::Color32::from_rgb(200, 0, 0)
                                };
                                
                                let cmd_type = match &cmd.command_type {
                                    CommandType::Transmit => "TRANSMIT".to_string(),
                                    CommandType::Control { code } => format!("CONTROL(0x{:X})", code),
                                };
                                
                                ui.horizontal(|ui| {
                                    ui.colored_label(status_color, format!("[{}]", i + 1));
                                    ui.label(cmd.timestamp.format("%H:%M:%S").to_string());
                                    ui.label(cmd_type);
                                    ui.label(format!("{}ms", cmd.duration_ms));
                                });
                                
                                ui.horizontal(|ui| {
                                    ui.label("  In:");
                                    ui.code(format_hex_spaced(&cmd.input));
                                });
                                
                                if !cmd.output.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.label(" Out:");
                                        ui.code(format_hex_spaced(&cmd.output));
                                    });
                                }
                                
                                if let Some(ref error) = cmd.error {
                                    ui.horizontal(|ui| {
                                        ui.label("Error:");
                                        ui.colored_label(egui::Color32::from_rgb(200, 0, 0), error);
                                    });
                                }
                                
                                ui.separator();
                            }
                            
                            if self.auto_scroll_history && !history.is_empty() {
                                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                            }
                        });
                    
                    let stats = self.command_executor.get_statistics();
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(format!("Total: {}", stats.total_commands));
                        ui.label(format!("Success: {}", stats.successful_commands));
                        ui.label(format!("Failed: {}", stats.failed_commands));
                        ui.label(format!("Avg: {}ms", stats.average_duration_ms));
                    });
                });
        }
        
        // Main panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("PCSC Tester");
            
            // Reader selection
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Reader:");
                    
                    let reader_names: Vec<String> = self.available_readers
                        .iter()
                        .enumerate()
                        .map(|(i, r)| {
                            let status = if r.is_connected { " [CARD]" } else { "" };
                            format!("[{}] {}{}", i, r.name, status)
                        })
                        .collect();
                    
                    egui::ComboBox::from_label("")
                        .selected_text(
                            self.selected_reader_idx
                                .and_then(|i| reader_names.get(i))
                                .unwrap_or(&"Select reader...".to_string())
                        )
                        .show_ui(ui, |ui| {
                            for (i, name) in reader_names.iter().enumerate() {
                                ui.selectable_value(&mut self.selected_reader_idx, Some(i), name);
                            }
                        });
                    
                    if ui.button("Refresh").clicked() {
                        self.refresh_readers();
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Share Mode:");
                    ui.radio_value(&mut self.share_mode, ShareModeGui::Shared, "Shared");
                    ui.radio_value(&mut self.share_mode, ShareModeGui::Exclusive, "Exclusive");
                    ui.radio_value(&mut self.share_mode, ShareModeGui::Direct, "Direct");
                });
                
                ui.horizontal(|ui| {
                    let connection_text = match self.connection_status {
                        ConnectionStatus::Disconnected => "Connect",
                        ConnectionStatus::Connecting => "Connecting...",
                        ConnectionStatus::Connected => "Disconnect",
                        ConnectionStatus::Error => "Connect",
                    };
                    
                    let connect_button = ui.add_enabled(
                        self.selected_reader_idx.is_some() && 
                        self.connection_status != ConnectionStatus::Connecting,
                        egui::Button::new(connection_text)
                    );
                    
                    if connect_button.clicked() {
                        match self.connection_status {
                            ConnectionStatus::Connected => self.disconnect_from_reader(),
                            _ => self.connect_to_reader(),
                        }
                    }
                    
                    if let Some(ref reader_name) = self.connected_reader {
                        ui.colored_label(egui::Color32::from_rgb(0, 150, 0), 
                                         format!("Connected to: {}", reader_name));
                    }
                });
            });
            
            ui.separator();
            
            // Command input
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("APDU:");
                            ui.add(egui::TextEdit::singleline(&mut self.command_input)
                                .hint_text("e.g., 00A40400 or 00 A4 04 00")
                                .font(egui::TextStyle::Monospace));
                        });
                        
                        if ui.add_enabled(
                            self.connection_status == ConnectionStatus::Connected && 
                            !self.command_input.trim().is_empty(),
                            egui::Button::new("Send Transmit")
                        ).clicked() {
                            self.send_transmit_command();
                        }
                    });
                    
                    ui.separator();
                    
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Control Code:");
                            ui.add(egui::TextEdit::singleline(&mut self.control_code_input)
                                .hint_text("e.g., 0x42000C00 or 1107296256")
                                .font(egui::TextStyle::Monospace));
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Data (optional):");
                            ui.add(egui::TextEdit::singleline(&mut self.control_data_input)
                                .hint_text("e.g., 1234ABCD")
                                .font(egui::TextStyle::Monospace));
                        });
                        
                        if ui.add_enabled(
                            self.connection_status == ConnectionStatus::Connected && 
                            !self.control_code_input.trim().is_empty(),
                            egui::Button::new("Send Control")
                        ).clicked() {
                            self.send_control_command();
                        }
                    });
                });
            });
            
            ui.separator();
            
            // Response display
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Response Format:");
                    ui.radio_value(&mut self.response_format, ResponseFormatGui::HexSpaced, "Hex Spaced");
                    ui.radio_value(&mut self.response_format, ResponseFormatGui::Hex, "Hex");
                    ui.radio_value(&mut self.response_format, ResponseFormatGui::HexDump, "Hex Dump");
                    ui.radio_value(&mut self.response_format, ResponseFormatGui::Ascii, "ASCII");
                });
                
                ui.label("Response:");
                let response_text = self.format_response(&self.last_response);
                
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut response_text.as_str())
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                        );
                    });
            });
            
            ui.separator();
            
            // Quick actions
            ui.horizontal(|ui| {
                if ui.button("Show History").clicked() {
                    self.show_history = true;
                }
                
                if ui.button("Settings").clicked() {
                    self.show_settings = true;
                }
                
                if ui.button("Clear Response").clicked() {
                    self.last_response.clear();
                }
            });
        });
    }
}

pub fn run_gui() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(Arc::new(egui::IconData::default())),
        ..Default::default()
    };
    
    eframe::run_native(
        "PCSC Tester",  
        options,
        Box::new(|cc| Ok(Box::new(PcscTesterApp::new(cc)))),
    ).map_err(|e| anyhow::anyhow!("GUI error: {}", e))
}