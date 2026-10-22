#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod icons;

use eframe::egui;
use passlock::storage::Vault;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 650.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PassLock",
        options,
        Box::new(|cc| {
            icons::install(&cc.egui_ctx);
            app::apply_theme(&cc.egui_ctx);
            Ok(Box::new(app::App::new(Vault::default_path())))
        }),
    )
}
