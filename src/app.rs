use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use crate::metrics::Metrics;
use std::time::{Duration, Instant};

pub struct App {
    metrics: Metrics,
    last_refresh: Instant,
}

impl App {
    pub fn new() -> Self {
        Self { metrics: Metrics::new(), last_refresh: Instant::now() }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Refresh every 1.5 seconds
        if self.last_refresh.elapsed() > Duration::from_millis(1500) {
            self.metrics.refresh();
            self.last_refresh = Instant::now();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("System Monitor");
            ui.separator();

            // CPU chart
            ui.label(format!("CPU: {:.1}%", self.metrics.cpu_history.last().unwrap_or(&0.0)));
            let points: PlotPoints = self.metrics.cpu_history.iter()
                .enumerate()
                .map(|(i, &v)| [i as f64, v])
                .collect();
            Plot::new("cpu_plot")
                .height(150.0)
                .include_y(0.0)
                .include_y(100.0)
                .show(ui, |plot_ui| plot_ui.line(Line::new(points)));

            ui.separator();

            // RAM bar
            let ram_pct = self.metrics.ram_used as f32 / self.metrics.ram_total as f32;
            ui.label(format!(
                "RAM: {:.1} GB / {:.1} GB",
                self.metrics.ram_used as f64 / 1e9,
                self.metrics.ram_total as f64 / 1e9
            ));
            ui.add(egui::ProgressBar::new(ram_pct).text(format!("{:.0}%", ram_pct * 100.0)));
        });

        ctx.request_repaint(); // keep updating
    }
}