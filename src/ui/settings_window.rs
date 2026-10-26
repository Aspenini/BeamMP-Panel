use super::theme::{self, palette};
use crate::app::App;
use crate::settings::{Settings, Theme};
use crate::tray;
use egui::{Context, DragValue, Margin, RichText, Stroke};

pub fn show(app: &mut App, ctx: &Context) {
    if !app.settings_open {
        return;
    }
    let before = app.settings.clone();
    let tray_ok = app.tray_available();
    let mut open = true;

    let style = ctx.global_style();
    let p = if style.visuals.dark_mode {
        &theme::DARK
    } else {
        &theme::LIGHT
    };
    egui::Window::new(
        RichText::new("Settings").font(egui::FontId::new(16.0, theme::heading_family())),
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .frame(
        egui::Frame::new()
            .fill(p.surface)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(12)
            .shadow(style.visuals.window_shadow)
            .inner_margin(Margin::same(20)),
    )
    .show(ctx, |ui| {
        ui.set_width(380.0);
        let p = palette(ui);
        let s = &mut app.settings;

        theme::section_title(ui, "When the window is closed");
        ui.add_enabled_ui(tray_ok, |ui| {
            ui.radio_value(
                &mut s.close_to_tray,
                true,
                "Keep running in the system tray",
            );
            ui.radio_value(&mut s.close_to_tray, false, "Quit the panel");
            ui.add_space(2.0);
            ui.checkbox(&mut s.start_minimized, "Start minimized to the tray");
        });
        if !tray_ok {
            ui.label(
                RichText::new(if tray::SUPPORTED {
                    "The tray icon could not be created, so closing quits."
                } else {
                    "The system tray isn't supported on this platform."
                })
                .small()
                .color(p.muted),
            );
        }
        ui.checkbox(&mut s.confirm_quit, "Ask before quitting while servers run");

        ui.add_space(14.0);
        theme::section_title(ui, "Appearance");
        ui.add_space(2.0);
        theme::segmented(
            ui,
            &mut s.theme,
            &[
                (Theme::System, "System"),
                (Theme::Light, "Light"),
                (Theme::Dark, "Dark"),
            ],
        );

        ui.add_space(14.0);
        theme::section_title(ui, "Servers");
        ui.add_space(2.0);
        egui::Grid::new("settings_servers")
            .num_columns(2)
            .spacing([24.0, 8.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Console history").color(p.muted));
                ui.add(
                    DragValue::new(&mut s.console_max_lines)
                        .range(Settings::CONSOLE_LINES_RANGE)
                        .speed(100)
                        .suffix(" lines"),
                );
                ui.end_row();
                ui.label(RichText::new("Stop timeout").color(p.muted));
                ui.add(
                    DragValue::new(&mut s.stop_timeout_secs)
                        .range(Settings::STOP_TIMEOUT_RANGE)
                        .suffix(" s"),
                )
                .on_hover_text("How long to wait for a graceful exit before force-killing");
                ui.end_row();
            });

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Saved automatically").small().color(p.muted));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(theme::link_button(ui, "Reset to defaults"))
                    .clicked()
                {
                    *s = Settings::default();
                }
            });
        });
    });

    app.settings_open = open;
    if app.settings != before {
        app.save_settings();
    }
}
