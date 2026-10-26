use super::theme::{self, palette};
use crate::app::{App, Dialog};
use crate::session::State;
use crate::util;
use egui::{Align2, CornerRadius, FontId, Margin, RichText, Sense, TextStyle, Ui, vec2};

enum Action {
    Select(String),
    Start(String),
    Stop(String),
    OpenFolder(std::path::PathBuf),
    Remove(String, String),
}

const ROW_HEIGHT: f32 = 46.0;

pub fn show(app: &mut App, ui: &mut Ui) {
    let p = palette(ui);
    let mut action = None;

    egui::Panel::left("servers_panel")
        .resizable(true)
        .min_size(210.0)
        .max_size(420.0)
        .default_size(250.0)
        .show_separator_line(false)
        .frame(
            egui::Frame::new()
                .fill(p.sidebar)
                .stroke(egui::Stroke::new(1.0, p.border))
                .inner_margin(Margin::symmetric(10, 14)),
        )
        .show(ui, |ui| {
            // Brand
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                ui.label(
                    RichText::new("BeamMP")
                        .font(FontId::new(17.0, theme::heading_family()))
                        .color(p.accent),
                );
                ui.label(
                    RichText::new("Panel")
                        .font(FontId::new(17.0, theme::heading_family()))
                        .color(p.text_strong),
                );
            });
            ui.add_space(14.0);

            ui.horizontal(|ui| {
                ui.add_space(6.0);
                theme::section_title(ui, "Servers");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(theme::link_button(ui, "+ Add"))
                        .on_hover_text("Add a server folder")
                        .clicked()
                    {
                        app.add_server();
                    }
                });
            });
            ui.add_space(4.0);

            // Footer first so the list takes the remaining height
            egui::Panel::bottom("servers_footer")
                .show_separator_line(false)
                .frame(egui::Frame::new().inner_margin(Margin::symmetric(0, 6)))
                .show(ui, |ui| {
                    let settings = ui.add_sized(
                        [ui.available_width(), 30.0],
                        egui::Button::new(RichText::new("Settings").color(p.text)),
                    );
                    if settings.clicked() {
                        app.settings_open = true;
                    }
                });

            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for server in &app.servers.servers {
                        let state = app.state_of(&server.id);
                        let selected = app.selected.as_deref() == Some(server.id.as_str());
                        let response = server_row(ui, &server.name, state, selected)
                            .on_hover_text(server.path.display().to_string());

                        if response.clicked() {
                            action = Some(Action::Select(server.id.clone()));
                        }
                        response.context_menu(|ui| {
                            let running = state != State::Stopped;
                            if !running && ui.button("Start").clicked() {
                                action = Some(Action::Start(server.id.clone()));
                            }
                            if running && ui.button("Stop").clicked() {
                                action = Some(Action::Stop(server.id.clone()));
                            }
                            if ui.button("Open folder").clicked() {
                                action = Some(Action::OpenFolder(server.path.clone()));
                            }
                            ui.separator();
                            if ui
                                .add_enabled(!running, egui::Button::new("Remove from panel"))
                                .on_disabled_hover_text("Stop the server first")
                                .clicked()
                            {
                                action =
                                    Some(Action::Remove(server.id.clone(), server.name.clone()));
                            }
                        });
                    }
                    if app.servers.servers.is_empty() {
                        ui.add_space(8.0);
                        ui.label(RichText::new("  No servers added").small().color(p.muted));
                    }
                });
        });

    match action {
        Some(Action::Select(id)) => app.selected = Some(id),
        Some(Action::Start(id)) => app.start_server(&id),
        Some(Action::Stop(id)) => app.stop_server(&id),
        Some(Action::OpenFolder(path)) => {
            if let Err(e) = util::open_in_file_manager(&path) {
                app.set_status(format!("Failed to open folder: {e}"), true);
            }
        }
        Some(Action::Remove(id, name)) => app.dialog = Some(Dialog::RemoveServer { id, name }),
        None => {}
    }
}

/// A two-line, custom-painted list row: status dot, name and state.
fn server_row(ui: &mut Ui, name: &str, state: State, selected: bool) -> egui::Response {
    let p = palette(ui);
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();

    let bg = if selected {
        Some(p.row_selected)
    } else if response.hovered() {
        Some(p.row_hover)
    } else {
        None
    };
    if let Some(bg) = bg {
        painter.rect_filled(rect, CornerRadius::same(8), bg);
    }
    if selected {
        // Accent marker on the left edge
        let marker = egui::Rect::from_min_size(
            rect.left_top() + vec2(0.0, 12.0),
            vec2(3.0, rect.height() - 24.0),
        );
        painter.rect_filled(marker, CornerRadius::same(2), p.accent);
    }

    let left = rect.left() + 14.0;
    painter.circle_filled(
        egui::pos2(left + 4.0, rect.center().y),
        4.0,
        super::state_color(ui, state),
    );

    let text_left = left + 18.0;
    let max_width = rect.right() - text_left - 8.0;
    let body = TextStyle::Body.resolve(ui.style());
    let name_color = if selected { p.text_strong } else { p.text };
    let mut job = egui::text::LayoutJob::simple_singleline(name.to_string(), body, name_color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_width);
    let name_galley = painter.layout_job(job);
    painter.galley(
        egui::pos2(text_left, rect.top() + 7.0),
        name_galley,
        name_color,
    );
    painter.text(
        egui::pos2(text_left, rect.bottom() - 8.0),
        Align2::LEFT_BOTTOM,
        super::state_label(state),
        FontId::proportional(11.5),
        p.muted,
    );
    response
}
