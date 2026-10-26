//! System tray icon. Supported on Windows and macOS, where it runs on winit's
//! own event loop; on other platforms `Tray::new` returns an error and the
//! panel simply quits on close.

use crate::app::AppCommand;
use std::sync::mpsc::Sender;

pub const SUPPORTED: bool = cfg!(any(windows, target_os = "macos"));

#[cfg(any(windows, target_os = "macos"))]
pub use imp::Tray;

#[cfg(not(any(windows, target_os = "macos")))]
pub struct Tray;

#[cfg(not(any(windows, target_os = "macos")))]
impl Tray {
    pub fn new(
        _ctx: &egui::Context,
        _icon: Option<&egui::IconData>,
        _commands: Sender<AppCommand>,
    ) -> anyhow::Result<Self> {
        anyhow::bail!("The system tray is not supported on this platform")
    }

    pub fn set_tooltip(&mut self, _text: &str) {}
}

#[cfg(any(windows, target_os = "macos"))]
mod imp {
    use super::*;
    use anyhow::Result;
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

    pub struct Tray {
        icon: TrayIcon,
        tooltip: String,
    }

    impl Tray {
        /// Must be called from the event loop thread once it is running
        /// (i.e. from the eframe app creator).
        pub fn new(
            ctx: &egui::Context,
            icon: Option<&egui::IconData>,
            commands: Sender<AppCommand>,
        ) -> Result<Self> {
            let show = MenuItem::new("Show BeamMP Panel", true, None);
            let quit = MenuItem::new("Quit", true, None);
            let menu = Menu::new();
            menu.append_items(&[&show, &PredefinedMenuItem::separator(), &quit])?;

            let mut builder = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                // Left click restores the window; right click opens the menu.
                .with_menu_on_left_click(false)
                .with_tooltip("BeamMP Panel");
            if let Some(icon) = icon {
                builder = builder.with_icon(tray_icon::Icon::from_rgba(
                    icon.rgba.clone(),
                    icon.width,
                    icon.height,
                )?);
            }
            let tray = builder.build()?;

            // The handlers run on the event loop thread even while the window
            // is hidden; forward to the app and wake it so `logic` runs.
            let (show_id, quit_id) = (show.id().clone(), quit.id().clone());
            let menu_ctx = ctx.clone();
            let menu_commands = commands.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                let command = if event.id == show_id {
                    AppCommand::Show
                } else if event.id == quit_id {
                    AppCommand::Quit
                } else {
                    return;
                };
                let _ = menu_commands.send(command);
                menu_ctx.request_repaint();
            }));

            let icon_ctx = ctx.clone();
            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                let show = matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } | TrayIconEvent::DoubleClick { .. }
                );
                if show {
                    let _ = commands.send(AppCommand::Show);
                    icon_ctx.request_repaint();
                }
            }));

            Ok(Self {
                icon: tray,
                tooltip: "BeamMP Panel".to_string(),
            })
        }

        pub fn set_tooltip(&mut self, text: &str) {
            if self.tooltip != text {
                let _ = self.icon.set_tooltip(Some(text));
                self.tooltip = text.to_string();
            }
        }
    }
}
