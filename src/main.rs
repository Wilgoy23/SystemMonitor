mod app;
mod metrics;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_title("System Monitor"),
        ..Default::default()
    };
    eframe::run_native("sysmon", options, Box::new(|_cc| Ok(Box::new(app::App::new()))))
}