use super::theme::{self, palette};
use crate::app::{App, Dialog};
use crate::util;
use egui::{Context, Id, Margin, Modal, RichText, ScrollArea, Stroke, Ui};

enum Outcome {
    Keep,
    Close,
    RemoveServer(String),
    DeleteMod(std::path::PathBuf, String),
    Quit,
}

fn modal(ctx: &Context, id: &str) -> Modal {
    let style = ctx.global_style();
    let p = if style.visuals.dark_mode { &theme::DARK } else { &theme::LIGHT };
    Modal::new(Id::unique(id))
        .backdrop_color(egui::Color32::from_black_alpha(140))
        .frame(
            egui::Frame::new()
                .fill(p.surface)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(12)
                .shadow(style.visuals.window_shadow)
                .inner_margin(Margin::same(22)),
        )
}

pub fn show(app: &mut App, ctx: &Context) {
    if app.stopping_for_quit() {
        quitting(app, ctx);
        return;
    }
    let Some(dialog) = &app.dialog else {
        return;
    };
    let running = app.running_count();

    let mut outcome = Outcome::Keep;
    let response = modal(ctx, "dialog").show(ctx, |ui| {
        let p = palette(ui);
        ui.set_width(420.0);
        match dialog {
            Dialog::RemoveServer { id, name } => {
                title(ui, "Remove server?");
                ui.label(format!("\"{name}\" will be removed from the panel."));
                note(ui, "No files are deleted. You can add the folder again later.");
                if buttons(ui, "Remove", false) {
                    outcome = Outcome::RemoveServer(id.clone());
                }
            }
            Dialog::DeleteMod { path, name } => {
                title(ui, "Delete mod?");
                ui.label(format!("\"{name}\" will be permanently deleted from disk."));
                ui.label(RichText::new("This cannot be undone.").small().color(p.danger));
                if buttons(ui, "Delete", true) {
                    outcome = Outcome::DeleteMod(path.clone(), name.clone());
                }
            }
            Dialog::ConfirmQuit => {
                title(ui, "Quit BeamMP Panel?");
                ui.label(format!(
                    "{running} server{} still running and will be stopped.",
                    if running == 1 { " is" } else { "s are" }
                ));
                if buttons(ui, "Stop servers and quit", true) {
                    outcome = Outcome::Quit;
                }
            }
            Dialog::ModDetails(None) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Reading mod...");
                });
            }
            Dialog::ModDetails(Some(details)) => {
                title(ui, &details.name);
                ui.horizontal(|ui| {
                    if !details.level_names.is_empty() {
                        theme::pill(ui, "LEVEL", p.info);
                    }
                    if !details.vehicle_names.is_empty() {
                        theme::pill(ui, "VEHICLE", p.vehicle);
                    }
                    if details.level_names.is_empty() && details.vehicle_names.is_empty() {
                        theme::pill(ui, "NO LEVELS OR VEHICLES", p.muted);
                    }
                });
                ui.add_space(10.0);
                egui::Grid::new("mod_details")
                    .num_columns(2)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        let row = |ui: &mut Ui, k: &str, v: String| {
                            ui.label(RichText::new(k).color(p.muted));
                            ui.label(v);
                            ui.end_row();
                        };
                        row(ui, "Files", details.total_files.to_string());
                        row(ui, "Archive size", util::format_bytes(details.archive_size));
                        if let Some(size) = details.uncompressed_size {
                            row(
                                ui,
                                "Uncompressed",
                                util::format_bytes(u64::try_from(size).unwrap_or(u64::MAX)),
                            );
                        }
                    });
                for (heading, names) in [
                    ("Levels", &details.level_names),
                    ("Vehicles", &details.vehicle_names),
                ] {
                    if names.is_empty() {
                        continue;
                    }
                    ui.add_space(12.0);
                    theme::section_title(ui, &format!("{heading} ({})", names.len()));
                    ui.add_space(2.0);
                    ScrollArea::vertical()
                        .id_salt(heading)
                        .max_height(140.0)
                        .show(ui, |ui| {
                            for name in names {
                                ui.label(RichText::new(name).monospace());
                            }
                        });
                }
                ui.add_space(14.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        outcome = Outcome::Close;
                    }
                });
            }
        }
    });

    if matches!(outcome, Outcome::Keep) && response.should_close() {
        outcome = Outcome::Close;
    }
    match outcome {
        Outcome::Keep => {}
        Outcome::Close => app.dialog = None,
        Outcome::RemoveServer(id) => {
            app.dialog = None;
            app.remove_server(&id);
        }
        Outcome::DeleteMod(path, name) => {
            app.dialog = None;
            app.delete_mod(&path, &name);
        }
        Outcome::Quit => {
            app.dialog = None;
            app.begin_quit();
        }
    }
}

fn title(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(egui::FontId::new(18.0, theme::heading_family()))
            .color(palette(ui).text_strong),
    );
    ui.add_space(6.0);
}

fn note(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().color(palette(ui).muted));
}

/// Cancel + confirm buttons, right aligned; returns true if confirmed.
fn buttons(ui: &mut Ui, confirm: &str, destructive: bool) -> bool {
    ui.add_space(18.0);
    let mut confirmed = false;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let button = if destructive {
            theme::danger_button(ui, confirm)
        } else {
            theme::primary_button(ui, confirm)
        };
        confirmed = ui.add(button).clicked();
        if ui.button("Cancel").clicked() {
            ui.close();
        }
    });
    confirmed
}

fn quitting(app: &mut App, ctx: &Context) {
    let running = app.running_count();
    let mut force = false;
    modal(ctx, "quitting").show(ctx, |ui| {
        ui.set_width(320.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(format!(
                "Stopping {running} server{}...",
                if running == 1 { "" } else { "s" }
            ));
        });
        ui.add_space(14.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            force = ui
                .add(theme::danger_button(ui, "Quit now"))
                .on_hover_text("Kill the servers immediately")
                .clicked();
        });
    });
    if force {
        app.close_now();
    }
}
