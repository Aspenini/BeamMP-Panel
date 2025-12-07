use egui::{ScrollArea, Ui};

pub enum ControlAction {
    None,
    SendCommand(String),
    RefreshPlayers,
}

pub fn show(
    ui: &mut Ui,
    is_server_running: bool,
    player_list: &mut Vec<String>,
    kick_player_name: &mut String,
    kick_reason: &mut String,
    broadcast_message: &mut String,
) -> ControlAction {
    if !is_server_running {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Server Control");
            ui.add_space(20.0);
            ui.label("Start the server to use control features");
        });
        return ControlAction::None;
    }

    let mut action = ControlAction::None;

    ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Server Control Panel");
        ui.separator();

        // Player Management Section
        ui.group(|ui| {
            ui.heading("Player Management");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("🔄 Refresh Player List").clicked() {
                    action = ControlAction::RefreshPlayers;
                }
                
                ui.label(format!("Players: {}", player_list.len()));
            });

            ui.add_space(5.0);

            if player_list.is_empty() {
                ui.label("Click 'Refresh Player List' to see connected players");
            } else {
                ui.label("Connected Players:");
                ui.indent("player_list", |ui| {
                    for player in player_list.iter() {
                        ui.label(format!("• {}", player));
                    }
                });
            }
        });

        ui.add_space(10.0);

        // Kick Player Section
        ui.group(|ui| {
            ui.heading("Kick Player");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Player Name:");
                ui.text_edit_singleline(kick_player_name);
            });

            ui.horizontal(|ui| {
                ui.label("Reason (optional):");
                ui.text_edit_singleline(kick_reason);
            });

            ui.horizontal(|ui| {
                if ui.button("⚠ Kick Player").clicked() {
                    if !kick_player_name.is_empty() {
                        let cmd = if kick_reason.is_empty() {
                            format!("kick {}", kick_player_name)
                        } else {
                            format!("kick {} {}", kick_player_name, kick_reason)
                        };
                        action = ControlAction::SendCommand(cmd);
                        kick_player_name.clear();
                        kick_reason.clear();
                    }
                }

                if ui.button("Clear").clicked() {
                    kick_player_name.clear();
                    kick_reason.clear();
                }
            });
        });

        ui.add_space(10.0);

        // Chat/Broadcast Section
        ui.group(|ui| {
            ui.heading("Broadcast Message");
            ui.add_space(5.0);

            ui.label("Message to all players:");
            ui.text_edit_singleline(broadcast_message);

            if ui.button("📢 Send Message").clicked() {
                if !broadcast_message.is_empty() {
                    action = ControlAction::SendCommand(format!("say {}", broadcast_message));
                    broadcast_message.clear();
                }
            }
        });

        ui.add_space(10.0);

        // Server Commands Section
        ui.group(|ui| {
            ui.heading("Server Commands");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("📊 Status").clicked() {
                    action = ControlAction::SendCommand("status".to_string());
                }

                if ui.button("ℹ Version").clicked() {
                    action = ControlAction::SendCommand("version".to_string());
                }

                if ui.button("🔄 Reload Mods").clicked() {
                    action = ControlAction::SendCommand("reloadmods".to_string());
                }
            });

            ui.horizontal(|ui| {
                if ui.button("🧹 Clear Console").clicked() {
                    action = ControlAction::SendCommand("clear".to_string());
                }

                if ui.button("❓ Help").clicked() {
                    action = ControlAction::SendCommand("help".to_string());
                }
            });
        });

        ui.add_space(10.0);

        // Lua Console Section
        ui.group(|ui| {
            ui.heading("Advanced");
            ui.add_space(5.0);

            if ui.button("🔧 Lua Console").clicked() {
                action = ControlAction::SendCommand("lua".to_string());
            }

            ui.label("Opens the Lua interactive console");
        });

        ui.add_space(10.0);

        // Info Section
        ui.group(|ui| {
            ui.heading("ℹ Command Information");
            ui.add_space(5.0);
            
            ui.label("All commands are executed in the server console.");
            ui.label("Output will appear in the Server Console panel below.");
            ui.label("Note: Player list parsing is basic - check console for full details.");
        });
    });
    
    action
}

