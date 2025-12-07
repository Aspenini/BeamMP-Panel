use crate::settings::AppSettings;
use crate::StatusMessage;
use egui::{ScrollArea, Ui};

pub fn show(ui: &mut Ui, settings: &mut AppSettings, status: &mut Option<StatusMessage>) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Application Settings");
        ui.separator();

        ui.add_space(10.0);

        ui.label("System Tray");
        ui.indent("tray_settings", |ui| {
            let minimize_changed = ui.checkbox(
                &mut settings.minimize_to_tray,
                "Minimize to system tray when closing window",
            ).changed();

            ui.label("When enabled, clicking the X button will minimize the app to the system tray instead of exiting.");
            
            ui.add_space(5.0);

            let start_changed = ui.checkbox(
                &mut settings.start_minimized,
                "Start minimized to tray",
            ).changed();

            ui.label("When enabled, the app will start in the system tray without showing the window.");

            if minimize_changed || start_changed {
                // Auto-save settings when changed
                if let Err(e) = settings.save() {
                    *status = Some(StatusMessage {
                        text: format!("Failed to save settings: {}", e),
                        is_error: true,
                    });
                } else {
                    *status = Some(StatusMessage {
                        text: "Settings saved".to_string(),
                        is_error: false,
                    });
                }
            }
        });

        ui.add_space(20.0);

        ui.label("System Tray Usage:");
        ui.indent("tray_usage", |ui| {
            ui.label("• Right-click the tray icon to access the menu");
            ui.label("• Select 'Show' to restore the window");
            ui.label("• Select 'Quit' to completely exit the application");
        });

        ui.add_space(20.0);

        ui.separator();
        
        if ui.button("Save Settings").clicked() {
            match settings.save() {
                Ok(_) => {
                    *status = Some(StatusMessage {
                        text: "Settings saved successfully!".to_string(),
                        is_error: false,
                    });
                }
                Err(e) => {
                    *status = Some(StatusMessage {
                        text: format!("Failed to save settings: {}", e),
                        is_error: true,
                    });
                }
            }
        }
    });
}

