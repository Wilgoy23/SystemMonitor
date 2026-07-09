mod app;
mod charts;
mod history;
mod metrics;
mod panels;
mod platform;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([480.0, 360.0])
            .with_app_id("sysmon")
            .with_title("System Monitor"),
        ..Default::default()
    };
    eframe::run_native("sysmon", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
