#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod console;
mod instance;
mod mods;
mod process;
mod server;
mod session;
mod settings;
mod tray;
mod ui;
mod util;

use eframe::egui;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let listener = match instance::claim() {
        instance::Claim::Primary(listener) => listener,
        // The running panel has been asked to show its window.
        instance::Claim::AlreadyRunning => return Ok(()),
    };
    let start_minimized = std::env::args().any(|a| a == "--minimized");

    let icon = load_icon();
    // Handed to whichever renderer attempt gets as far as creating the app
    let pending = Rc::new(RefCell::new(Some((icon.clone(), listener))));

    let run = |renderer| {
        let pending = Rc::clone(&pending);
        eframe::run_native(
            "BeamMP Panel",
            native_options(icon.as_ref(), renderer),
            Box::new(move |cc| {
                let (icon, listener) = pending.borrow_mut().take().unwrap_or_default();
                Ok(Box::new(app::App::new(
                    cc,
                    icon.map(tray_icon_size),
                    listener,
                    start_minimized,
                )))
            }),
        )
    };

    // OpenGL uses a fraction of wgpu's memory. Where it isn't available
    // (Remote Desktop on a GPU-less VPS, broken drivers), fall back to wgpu,
    // which can use DirectX/Vulkan or a software adapter.
    match run(eframe::Renderer::Glow) {
        Err(e) if pending.borrow().is_some() => {
            eprintln!("OpenGL renderer unavailable ({e}); falling back to wgpu");
            run(eframe::Renderer::Wgpu)
        }
        result => result,
    }
}

fn native_options(icon: Option<&egui::IconData>, renderer: eframe::Renderer) -> eframe::NativeOptions {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 720.0])
        .with_min_inner_size([760.0, 420.0])
        .with_title("BeamMP Panel")
        .with_app_id("beammp-panel");
    if let Some(icon) = icon {
        viewport = viewport.with_icon(Arc::new(icon.clone()));
    }
    eframe::NativeOptions {
        viewport,
        renderer,
        // No 3D content: skip the depth/stencil buffers and MSAA
        depth_buffer: 0,
        stencil_buffer: 0,
        multisampling: 0,
        // Restores size/position from the last run
        persist_window: true,
        // Keep window state next to the rest of the data when it's relocated
        persistence_path: std::env::var_os(util::CONFIG_DIR_ENV)
            .filter(|d| !d.is_empty())
            .map(|d| std::path::PathBuf::from(d).join("window.ron")),
        ..Default::default()
    }
}

fn load_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../icons/BeamMP-Panel.ico")).ok()?;
    let rgba = img.into_rgba8();
    let (width, height) = rgba.dimensions();
    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

/// Tray icons are drawn at 16-32px; hand the OS a small image instead of 256px.
fn tray_icon_size(icon: egui::IconData) -> egui::IconData {
    const SIZE: u32 = 32;
    if icon.width <= SIZE {
        return icon;
    }
    let Some(img) = image::RgbaImage::from_raw(icon.width, icon.height, icon.rgba.clone()) else {
        return icon;
    };
    let small = image::imageops::resize(&img, SIZE, SIZE, image::imageops::FilterType::Lanczos3);
    egui::IconData {
        rgba: small.into_raw(),
        width: SIZE,
        height: SIZE,
    }
}
