use eframe::egui;
use crate::metrics::Source;
use crate::panels::{self, Panel};
use std::time::{Duration, Instant};

/// User-tweakable settings, persisted across runs via eframe storage.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(default)]
struct Settings {
    refresh_ms: u64,
    paused: bool,
    hidden: Vec<String>, // panel names the user has hidden
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_ms: 1500,
            paused: false,
            hidden: Vec::new(),
        }
    }
}

pub struct App {
    source: Source,
    panels: Vec<Box<dyn Panel>>,
    settings: Settings,
    last_refresh: Instant,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = cc
            .storage
            .and_then(|s| eframe::get_value::<Settings>(s, eframe::APP_KEY))
            .unwrap_or_default();

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
            settings,
            last_refresh: Instant::now(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.settings.paused
            && self.last_refresh.elapsed() >= Duration::from_millis(self.settings.refresh_ms)
        {
            let handles = self.source.refresh();
            for panel in &mut self.panels {
                panel.refresh(&handles);
            }
            self.last_refresh = Instant::now();
        }

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let settings = &mut self.settings;
                ui.checkbox(&mut settings.paused, "⏸ Pause");
                ui.separator();
                ui.label("Refresh:");
                ui.add(egui::Slider::new(&mut settings.refresh_ms, 250..=5000).suffix(" ms"));
                ui.separator();
                ui.menu_button("Panels ⏷", |ui| {
                    for panel in &self.panels {
                        let name = panel.name().to_string();
                        let mut shown = !settings.hidden.iter().any(|h| h == &name);
                        if ui.checkbox(&mut shown, &name).changed() {
                            if shown {
                                settings.hidden.retain(|h| h != &name);
                            } else {
                                settings.hidden.push(name);
                            }
                        }
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("System Monitor");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let hidden = &self.settings.hidden;
                for panel in &mut self.panels {
                    if hidden.iter().any(|h| h == panel.name()) {
                        continue;
                    }
                    ui.heading(panel.name());
                    panel.ui(ui);
                    ui.separator();
                }
            });
        });

        // Only keep animating while actively refreshing.
        if !self.settings.paused {
            ctx.request_repaint_after(Duration::from_millis(self.settings.refresh_ms));
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.settings);
    }
}
