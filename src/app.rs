use eframe::egui;
use crate::charts;
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
            charts::percent_plot(ui, "cpu_plot", &self.metrics.cpu_history);

            ui.separator();

            // RAM bar
            charts::usage_bar(ui, "RAM", self.metrics.ram_used, self.metrics.ram_total);
        });

        ctx.request_repaint(); // keep updating
    }
}