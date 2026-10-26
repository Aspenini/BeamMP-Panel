//! Visual design: palettes, fonts, global widget styling and a few small
//! custom-painted widgets. Everything here is set up once at startup or is
//! plain painting; nothing animates or requests repaints on its own.

use egui::{
    Color32, CornerRadius, FontData, FontFamily, FontId, Margin, Response, RichText, Sense,
    Shadow, Stroke, TextStyle, Ui, Vec2, vec2,
};
use std::sync::Arc;

pub struct Palette {
    /// Main content background
    pub bg: Color32,
    /// Sidebar and status bar
    pub sidebar: Color32,
    /// Cards, dialogs
    pub surface: Color32,
    /// Text inputs, console
    pub sunken: Color32,
    pub border: Color32,
    pub control: Color32,
    pub control_hover: Color32,
    pub row_hover: Color32,
    pub row_selected: Color32,
    pub text: Color32,
    pub text_strong: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub info: Color32,
    pub vehicle: Color32,
}

pub const DARK: Palette = Palette {
    bg: Color32::from_rgb(0x17, 0x19, 0x1e),
    sidebar: Color32::from_rgb(0x11, 0x13, 0x17),
    surface: Color32::from_rgb(0x1f, 0x22, 0x29),
    sunken: Color32::from_rgb(0x0d, 0x0f, 0x12),
    border: Color32::from_rgb(0x33, 0x38, 0x43),
    control: Color32::from_rgb(0x29, 0x2d, 0x36),
    control_hover: Color32::from_rgb(0x33, 0x38, 0x43),
    row_hover: Color32::from_rgb(0x1c, 0x1f, 0x26),
    row_selected: Color32::from_rgb(0x25, 0x29, 0x32),
    text: Color32::from_rgb(0xc9, 0xcd, 0xd6),
    text_strong: Color32::from_rgb(0xf2, 0xf4, 0xf8),
    muted: Color32::from_rgb(0x80, 0x87, 0x96),
    accent: Color32::from_rgb(0xf2, 0x7a, 0x1a),
    on_accent: Color32::from_rgb(0x16, 0x10, 0x0a),
    success: Color32::from_rgb(0x3f, 0xc1, 0x6a),
    warning: Color32::from_rgb(0xe3, 0xb3, 0x41),
    danger: Color32::from_rgb(0xf2, 0x5f, 0x5c),
    info: Color32::from_rgb(0x5a, 0xb4, 0xf5),
    vehicle: Color32::from_rgb(0xc6, 0x8c, 0xf2),
};

pub const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(0xf6, 0xf7, 0xf9),
    sidebar: Color32::from_rgb(0xec, 0xee, 0xf2),
    surface: Color32::from_rgb(0xff, 0xff, 0xff),
    sunken: Color32::from_rgb(0xf9, 0xfa, 0xfb),
    border: Color32::from_rgb(0xdc, 0xdf, 0xe5),
    control: Color32::from_rgb(0xec, 0xee, 0xf2),
    control_hover: Color32::from_rgb(0xe1, 0xe4, 0xea),
    row_hover: Color32::from_rgb(0xe4, 0xe7, 0xec),
    row_selected: Color32::from_rgb(0xdc, 0xe0, 0xe7),
    text: Color32::from_rgb(0x2b, 0x30, 0x3a),
    text_strong: Color32::from_rgb(0x10, 0x13, 0x19),
    muted: Color32::from_rgb(0x67, 0x6e, 0x7c),
    accent: Color32::from_rgb(0xdd, 0x62, 0x0a),
    on_accent: Color32::WHITE,
    success: Color32::from_rgb(0x1a, 0x7f, 0x37),
    warning: Color32::from_rgb(0x9a, 0x67, 0x00),
    danger: Color32::from_rgb(0xcf, 0x22, 0x2e),
    info: Color32::from_rgb(0x0b, 0x69, 0xc7),
    vehicle: Color32::from_rgb(0x82, 0x50, 0xdf),
};

pub fn palette(ui: &Ui) -> &'static Palette {
    if ui.visuals().dark_mode { &DARK } else { &LIGHT }
}

/// Font family used for headings and titles.
pub fn heading_family() -> FontFamily {
    FontFamily::Name("heading".into())
}

// ------------------------------------------------------------------ setup

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.style_mut_of(egui::Theme::Dark, |style| apply(style, &DARK));
    ctx.style_mut_of(egui::Theme::Light, |style| apply(style, &LIGHT));
}

/// Use the platform UI fonts where available (read once at startup, never
/// bundled); egui's built-in fonts stay as fallbacks for other scripts.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let default_proportional = fonts.families[&FontFamily::Proportional].clone();
    let mut heading = default_proportional.clone();

    let mut load = |name: &str, candidates: &[&str]| -> bool {
        for path in candidates {
            if let Ok(bytes) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert(name.to_string(), Arc::new(FontData::from_owned(bytes)));
                return true;
            }
        }
        false
    };

    if cfg!(windows) {
        if load("ui", &[r"C:\Windows\Fonts\segoeui.ttf"]) {
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "ui".into());
            heading.insert(0, "ui".into());
        }
        if load("ui-semibold", &[r"C:\Windows\Fonts\seguisb.ttf"]) {
            heading.insert(0, "ui-semibold".into());
        }
        if load(
            "mono",
            &[r"C:\Windows\Fonts\CascadiaMono.ttf", r"C:\Windows\Fonts\consola.ttf"],
        ) {
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "mono".into());
        }
    }

    fonts.families.insert(heading_family(), heading);
    ctx.set_fonts(fonts);
}

fn apply(style: &mut egui::Style, p: &Palette) {
    use TextStyle::*;
    style.text_styles = [
        (Heading, FontId::new(20.0, heading_family())),
        (Body, FontId::proportional(14.0)),
        (Button, FontId::proportional(14.0)),
        (Small, FontId::proportional(11.5)),
        (Monospace, FontId::monospace(13.0)),
    ]
    .into();

    let s = &mut style.spacing;
    s.item_spacing = vec2(8.0, 6.0);
    s.button_padding = vec2(12.0, 5.0);
    s.interact_size = vec2(40.0, 28.0);
    s.window_margin = Margin::same(18);
    s.menu_margin = Margin::same(6);
    s.indent = 16.0;
    s.icon_width = 16.0;
    s.icon_width_inner = 8.0;
    s.combo_width = 200.0;
    s.text_edit_width = 260.0;
    s.scroll = egui::style::ScrollStyle::thin();

    style.interaction.selectable_labels = true;

    let v = &mut style.visuals;
    let radius = CornerRadius::same(6);
    v.panel_fill = p.bg;
    v.window_fill = p.surface;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(12);
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(if v.dark_mode { 110 } else { 40 }),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 14,
        spread: 0,
        color: Color32::from_black_alpha(if v.dark_mode { 90 } else { 30 }),
    };
    v.menu_corner_radius = CornerRadius::same(8);
    v.extreme_bg_color = p.sunken;
    v.text_edit_bg_color = Some(p.sunken);
    v.faint_bg_color = p.row_hover;
    v.code_bg_color = p.sunken;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warning;
    v.error_fg_color = p.danger;
    v.override_text_color = None;
    v.weak_text_color = Some(p.muted);
    v.selection.bg_fill = p.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.indent_has_left_vline = false;
    v.striped = false;
    v.interact_cursor = Some(egui::CursorIcon::PointingHand);
    v.text_cursor.stroke = Stroke::new(2.0, p.accent);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = radius;

    w.inactive.bg_fill = p.control;
    w.inactive.weak_bg_fill = p.control;
    w.inactive.bg_stroke = Stroke::new(1.0, p.border);
    w.inactive.fg_stroke = Stroke::new(1.0, p.text_strong);
    w.inactive.corner_radius = radius;
    w.inactive.expansion = 0.0;

    w.hovered.bg_fill = p.control_hover;
    w.hovered.weak_bg_fill = p.control_hover;
    w.hovered.bg_stroke = Stroke::new(1.0, p.muted.gamma_multiply(0.6));
    w.hovered.fg_stroke = Stroke::new(1.5, p.text_strong);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.control_hover;
    w.active.weak_bg_fill = p.control_hover;
    w.active.bg_stroke = Stroke::new(1.0, p.accent);
    w.active.fg_stroke = Stroke::new(1.5, p.text_strong);
    w.active.corner_radius = radius;
    w.active.expansion = 0.0;

    w.open = w.hovered;
}

// ------------------------------------------------------------- containers

/// A bordered card with a title.
pub fn card<R>(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = palette(ui);
    egui::Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(10)
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                section_title(ui, title);
                ui.add_space(6.0);
            }
            add(ui)
        })
        .inner
}

/// Small uppercase muted label introducing a section.
pub fn section_title(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .small()
            .strong()
            .extra_letter_spacing(0.8)
            .color(palette(ui).muted),
    );
}

pub fn heading(ui: &mut Ui, text: &str) -> Response {
    ui.label(
        RichText::new(text)
            .font(FontId::new(22.0, heading_family()))
            .color(palette(ui).text_strong),
    )
}

// ---------------------------------------------------------------- buttons

pub fn primary_button(ui: &Ui, text: &str) -> egui::Button<'static> {
    let p = palette(ui);
    egui::Button::new(RichText::new(text).color(p.on_accent).strong())
        .fill(p.accent)
        .stroke(Stroke::NONE)
}

pub fn danger_button(ui: &Ui, text: &str) -> egui::Button<'static> {
    let p = palette(ui);
    egui::Button::new(RichText::new(text).color(p.danger))
        .fill(p.danger.gamma_multiply(0.12))
        .stroke(Stroke::new(1.0, p.danger.gamma_multiply(0.45)))
}

/// Frameless text button, for low-emphasis actions.
pub fn link_button(ui: &Ui, text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).color(palette(ui).muted)).frame(false)
}

// ----------------------------------------------------------- indicators

pub fn dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

/// A rounded label with a tinted background.
pub fn pill(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let font = FontId::new(11.0, heading_family());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, color);
    let pad = vec2(8.0, 3.0);
    let (rect, response) = ui.allocate_exact_size(galley.size() + pad * 2.0, Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(255), color.gamma_multiply(0.16));
        painter.galley(rect.min + pad, galley, color);
    }
    response
}

/// Underlined tab bar. Returns true if the selection changed.
pub fn tabs<T: PartialEq + Copy>(ui: &mut Ui, current: &mut T, items: &[(T, &str)]) -> bool {
    let p = palette(ui);
    let mut changed = false;
    let font = TextStyle::Button.resolve(ui.style());
    let line_y = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let mut bottom = ui.cursor().min.y;
            for (value, label) in items {
                let selected = *current == *value;
                let color = if selected { p.text_strong } else { p.muted };
                let galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), color);
                let size = galley.size() + vec2(24.0, 18.0);
                let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                let color = if response.hovered() && !selected { p.text } else { color };
                ui.painter()
                    .galley_with_override_text_color(rect.center() - galley.size() / 2.0 - vec2(0.0, 1.0), galley, color);
                if selected {
                    let underline = egui::Rect::from_min_max(
                        egui::pos2(rect.left() + 8.0, rect.bottom() - 2.0),
                        egui::pos2(rect.right() - 8.0, rect.bottom()),
                    );
                    ui.painter().rect_filled(underline, 1.0, p.accent);
                }
                if response.clicked() && !selected {
                    *current = *value;
                    changed = true;
                }
                bottom = rect.bottom();
            }
            bottom
        })
        .inner;
    let rect = ui.max_rect();
    ui.painter().hline(rect.x_range(), line_y, Stroke::new(1.0, p.border));
    changed
}

/// Segmented control: a row of joined toggle buttons.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, current: &mut T, items: &[(T, &str)]) -> bool {
    let p = palette(ui);
    let mut changed = false;
    egui::Frame::new()
        .fill(p.sunken)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(8)
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.spacing_mut().button_padding = vec2(10.0, 3.0);
                for (value, label) in items {
                    let selected = *current == *value;
                    let text = if selected {
                        RichText::new(*label).color(p.text_strong)
                    } else {
                        RichText::new(*label).color(p.muted)
                    };
                    let button = egui::Button::new(text)
                        .fill(if selected { p.control_hover } else { Color32::TRANSPARENT })
                        .stroke(Stroke::NONE)
                        .corner_radius(6)
                        .min_size(vec2(0.0, 24.0));
                    if ui.add(button).clicked() && !selected {
                        *current = *value;
                        changed = true;
                    }
                }
            });
        });
    changed
}
