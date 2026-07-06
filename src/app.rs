use eframe::egui;
use crate::metrics::Source;
use crate::panels::{self, Panel};
use std::time::{Duration, Instant};

pub struct App {
    source: Source,
    panels: Vec<Box<dyn Panel>>,
    last_refresh: Instant,
}

impl App {
    pub fn new() -> Self {
        let mut source = Source::new();
        let mut panels = panels::default_panels();

        // Prime every panel once so the first frame has data.
        let handles = source.refresh();
        for panel in &mut panels {
            panel.refresh(&handles);
        }

        Self {
            source,
            panels,
            last_refresh: Instant::now(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Refresh every 1.5 seconds
        if self.last_refresh.elapsed() > Duration::from_millis(1500) {
            let handles = self.source.refresh();
            for panel in &mut self.panels {
                panel.refresh(&handles);
            }
            self.last_refresh = Instant::now();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("System Monitor");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for panel in &mut self.panels {
                    ui.heading(panel.name());
                    panel.ui(ui);
                    ui.separator();
                }
            });
        });

        ctx.request_repaint(); // keep updating
    }
}
