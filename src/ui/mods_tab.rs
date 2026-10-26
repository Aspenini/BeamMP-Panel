use super::theme::{self, palette};
use crate::app::{App, Dialog, ModFilter};
use crate::mods::{self, ModType};
use crate::util;
use egui::{Margin, RichText, Stroke, Ui};
use std::path::PathBuf;

enum Action {
    SetType(ModType),
    Refresh,
    AddFiles(Vec<PathBuf>),
    Toggle(usize, bool),
    Details(PathBuf),
    Delete(PathBuf, String),
    OpenFolder(PathBuf),
}

const ROW_HEIGHT: f32 = 40.0;

pub fn show(app: &mut App, ui: &mut Ui, id: &str) {
    app.ensure_mods_loaded();
    let Some(server) = app.servers.get(id) else {
        return;
    };
    let p = palette(ui);
    let view = &mut app.mods;
    let mut mod_type = view.mod_type;
    let mut action = None;

    // Toolbar: type, filter, search, actions
    ui.horizontal(|ui| {
        if theme::segmented(
            ui,
            &mut mod_type,
            &[
                (ModType::Client, "Client mods"),
                (ModType::Server, "Server plugins"),
            ],
        ) {
            action = Some(Action::SetType(mod_type));
        }
        if mod_type == ModType::Client {
            ui.add_space(4.0);
            theme::segmented(
                ui,
                &mut view.filter,
                &[
                    (ModFilter::All, "All"),
                    (ModFilter::Levels, "Levels"),
                    (ModFilter::Vehicles, "Vehicles"),
                ],
            );
        }
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut view.search)
                .hint_text("Search")
                .desired_width(170.0),
        );
        if view.loading || view.copying > 0 {
            ui.spinner();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if mod_type == ModType::Client
                && ui
                    .add(theme::primary_button(ui, "Add mods"))
                    .on_hover_text(
                        "Copy .zip files into the Client folder, or drop them on the window",
                    )
                    .clicked()
                && let Some(files) = rfd::FileDialog::new()
                    .add_filter("BeamNG mods", &["zip"])
                    .pick_files()
            {
                action = Some(Action::AddFiles(files));
            }
            let folder = mods::enabled_root(&server.path, server.resource_folder(), mod_type);
            if ui
                .add(theme::link_button(ui, "Open folder"))
                .on_hover_text(folder.display().to_string())
                .clicked()
            {
                action = Some(Action::OpenFolder(folder));
            }
            if ui.add(theme::link_button(ui, "Rescan")).clicked() {
                action = Some(Action::Refresh);
            }
        });
    });
    ui.add_space(10.0);

    // Filtered view (indices into view.mods)
    let search = view.search.trim().to_lowercase();
    let visible: Vec<usize> = view
        .mods
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            let type_ok = mod_type == ModType::Server
                || match view.filter {
                    ModFilter::All => true,
                    ModFilter::Levels => m.is_level,
                    ModFilter::Vehicles => m.is_vehicle,
                };
            type_ok && (search.is_empty() || m.name_lower.contains(&search))
        })
        .map(|(i, _)| i)
        .collect();

    if view.mods.is_empty() {
        theme::card(ui, "", |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(16.0);
                if view.loading {
                    ui.label(RichText::new("Scanning mods...").color(p.muted));
                } else {
                    ui.label(RichText::new("No mods found").color(p.text_strong));
                    ui.label(
                        RichText::new(match mod_type {
                            ModType::Client => "Drop .zip files onto the window or click Add mods.",
                            ModType::Server => {
                                "Server plugins are Lua folders in Resources/Server. Copy a plugin folder there."
                            }
                        })
                        .small()
                        .color(p.muted),
                    );
                }
                ui.add_space(16.0);
            });
        });
    } else {
        let enabled = view.mods.iter().filter(|m| m.enabled).count();
        ui.label(
            RichText::new(format!(
                "{} shown  ·  {} total  ·  {} enabled",
                visible.len(),
                view.mods.len(),
                enabled
            ))
            .small()
            .color(p.muted),
        );
        ui.add_space(6.0);

        egui::Frame::new()
            .fill(p.surface)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(10)
            .inner_margin(Margin::symmetric(6, 4))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show_rows(ui, ROW_HEIGHT, visible.len(), |ui, rows| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (n, &index) in visible[rows.clone()].iter().enumerate() {
                            let m = &view.mods[index];
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), ROW_HEIGHT),
                                egui::Sense::hover(),
                            );
                            if rows.start + n > 0 {
                                ui.painter().hline(
                                    rect.x_range().shrink(8.0),
                                    rect.top(),
                                    Stroke::new(1.0, p.border),
                                );
                            }
                            let mut row = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(rect.shrink2(egui::vec2(8.0, 0.0)))
                                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                            );
                            mod_row(&mut row, m, index, mod_type, &mut action);
                        }
                    });
            });
    }

    // Drag & drop .zip files onto the window to add them as client mods
    let dropped: Vec<PathBuf> = ui.ctx().input(|i| {
        i.raw
            .dropped_files
            .iter()
            .map(|f| f.path().to_path_buf())
            .filter(|p| !p.as_os_str().is_empty())
            .collect()
    });
    if !dropped.is_empty() {
        action = Some(Action::AddFiles(dropped));
    }
    if ui.ctx().input(|i| !i.raw.hovered_files.is_empty()) {
        drop_overlay(ui);
    }

    match action {
        Some(Action::SetType(t)) => {
            app.mods.mod_type = t;
            app.ensure_mods_loaded();
        }
        Some(Action::Refresh) => app.reload_mods(),
        Some(Action::AddFiles(files)) => app.add_client_mods(files),
        Some(Action::Toggle(index, on)) => app.set_mod_enabled(index, on),
        Some(Action::Details(path)) => app.show_mod_details(path),
        Some(Action::Delete(path, name)) => app.dialog = Some(Dialog::DeleteMod { path, name }),
        Some(Action::OpenFolder(path)) => {
            let _ = std::fs::create_dir_all(&path);
            if let Err(e) = util::open_in_file_manager(&path) {
                app.set_status(format!("Failed to open folder: {e}"), true);
            }
        }
        None => {}
    }
}

fn mod_row(
    ui: &mut Ui,
    m: &mods::ModEntry,
    index: usize,
    mod_type: ModType,
    action: &mut Option<Action>,
) {
    let p = palette(ui);
    let mut on = m.enabled;
    if ui
        .checkbox(&mut on, "")
        .on_hover_text(if m.enabled {
            "Enabled - click to disable"
        } else {
            "Disabled - click to enable"
        })
        .changed()
    {
        *action = Some(Action::Toggle(index, on));
    }
    let name = RichText::new(&m.name).color(if m.enabled { p.text_strong } else { p.muted });
    // Leave room for the tags and the right-aligned actions
    let name_width = (ui.available_width() - 330.0).max(80.0);
    ui.allocate_ui(egui::vec2(name_width, ROW_HEIGHT), |ui| {
        ui.add(egui::Label::new(name).truncate());
    });
    if m.is_level {
        theme::pill(ui, "LEVEL", p.info);
    }
    if m.is_vehicle {
        theme::pill(ui, "VEHICLE", p.vehicle);
    }
    if !m.enabled {
        theme::pill(ui, "DISABLED", p.muted);
    }

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui
            .add(egui::Button::new(RichText::new("Delete").color(p.danger)).frame(false))
            .on_hover_text("Delete from disk")
            .clicked()
        {
            *action = Some(Action::Delete(m.full_path.clone(), m.name.clone()));
        }
        if mod_type == ModType::Client && ui.add(theme::link_button(ui, "Details")).clicked() {
            *action = Some(Action::Details(m.full_path.clone()));
        }
        if let Some(size) = m.size {
            ui.label(
                RichText::new(util::format_bytes(size))
                    .small()
                    .color(p.muted),
            );
        }
    });
}

fn drop_overlay(ui: &Ui) {
    let p = palette(ui);
    let rect = ui.ctx().content_rect();
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::unique("mod_drop_overlay"),
    ));
    painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(170));
    painter.rect_stroke(
        rect.shrink(24.0),
        16.0,
        Stroke::new(2.0, p.accent),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Drop .zip files to add them as client mods",
        egui::FontId::new(20.0, theme::heading_family()),
        egui::Color32::WHITE,
    );
}
