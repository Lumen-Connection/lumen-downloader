// Compila como app de GUI (sem janela de console) no Windows.
#![windows_subsystem = "windows"]

mod app;
mod applog;
mod config;
mod db;
mod download;
mod notify;
mod queue;
mod ui;

fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _enter = rt.enter();

    applog::info("Lumen Downloader iniciado");
    let cfg = config::settings::Config::load();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([cfg.win_w, cfg.win_h])
            .with_min_inner_size([700.0, 450.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Lumen Downloader",
        options,
        Box::new(|_cc| {
            let app: Box<dyn eframe::App> = Box::new(app::App::new());
            app
        }),
    )
}
