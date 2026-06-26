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

            egui::ScrollArea::vertical().show(ui, |ui| {
                let m = &self.metrics;

                // CPU
                ui.label(format!("CPU: {:.1}%", m.cpu_history.last().unwrap_or(&0.0)));
                charts::percent_plot(ui, "cpu_plot", &m.cpu_history);
                ui.collapsing(format!("Per-core CPU ({} cores)", m.per_core.len()), |ui| {
                    charts::per_core_bars(ui, &m.per_core);
                });

                ui.separator();

                // Memory
                charts::usage_bar(ui, "RAM", m.ram_used, m.ram_total);

                ui.separator();

                // Disks
                ui.collapsing(format!("Disks ({})", m.disks_info.len()), |ui| {
                    for disk in &m.disks_info {
                        charts::usage_bar(ui, &disk.name, disk.used, disk.total);
                    }
                });

                ui.separator();

                // Network
                ui.label(format!(
                    "Network — down {}/s, up {}/s",
                    charts::format_bytes(m.net_down as u64),
                    charts::format_bytes(m.net_up as u64)
                ));
                charts::network_plot(ui, &m.net_down_history, &m.net_up_history);

                ui.separator();

                // Temperatures
                ui.collapsing("Temperatures", |ui| {
                    if m.temps.is_empty() {
                        ui.weak("No temperature sensors available.");
                    } else {
                        for (label, temp) in &m.temps {
                            ui.label(format!("{label}: {temp:.1} °C"));
                        }
                    }
                });
            });
        });

        ctx.request_repaint(); // keep updating
    }
}