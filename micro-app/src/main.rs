//! Точка входа: создание окна и запуск приложения.

// Без консольного окна в Windows-сборках
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod settings;
mod theme;
mod views;
mod widgets;

use app::MicroApp;

fn load_window_icon() -> Option<std::sync::Arc<egui::IconData>> {
    let bytes = include_bytes!("../assets/favicon.ico");
    match image::load_from_memory_with_format(bytes, image::ImageFormat::Ico) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            Some(std::sync::Arc::new(egui::IconData {
                width: rgba.width(),
                height: rgba.height(),
                rgba: rgba.into_raw(),
            }))
        }
        Err(_) => None,
    }
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Micro — Микропрограммируемый процессор")
        .with_inner_size([1200.0, 820.0])
        .with_min_inner_size([1000.0, 700.0]);
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Micro",
        options,
        Box::new(|cc| Ok(Box::new(MicroApp::new(cc)))),
    )
}
