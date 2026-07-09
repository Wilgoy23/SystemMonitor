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
    hidden: Vec<String>,       // panel names the user has hidden
    panel_order: Vec<String>,  // user-defined tab order, by panel name
    active_tab: Option<String>, // last-selected tab
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_ms: 1500,
            paused: false,
            hidden: Vec::new(),
            panel_order: Vec::new(),
            active_tab: None,
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

        // Apply the user's saved tab order, if any. Panels not mentioned
        // (e.g. newly added ones) keep their natural position at the end.
        if !settings.panel_order.is_empty() {
            let order = &settings.panel_order;
            panels.sort_by_key(|p| order.iter().position(|n| n == p.name()).unwrap_or(usize::MAX));
        }

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

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let hidden = self.settings.hidden.clone();
                let visible: Vec<usize> = self
                    .panels
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| !hidden.iter().any(|h| h == p.name()))
                    .map(|(i, _)| i)
                    .collect();

                let mut drag_from: Option<usize> = None;
                let mut drop_to: Option<usize> = None;

                for &idx in &visible {
                    let name = self.panels[idx].name().to_string();
                    let is_active = self.settings.active_tab.as_deref() == Some(name.as_str());

                    // A single widget sensing both click and drag: egui's hit
                    // test discards clicks when a *separate* drag-only widget
                    // sits on top of a click-only one at the same rect (that's
                    // what `dnd_drag_source` wrapping `selectable_label` gave
                    // us — it silently ate every click), so click and drag
                    // must be one widget here, not two overlapping ones.
                    let response = ui.add(
                        egui::Button::new(&name)
                            .selected(is_active)
                            .sense(egui::Sense::click_and_drag()),
                    );
                    response.dnd_set_drag_payload(idx);

                    if response.clicked() {
                        self.settings.active_tab = Some(name.clone());
                    }

                    if response.dnd_hover_payload::<usize>().is_some() {
                        if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                            let rect = response.rect;
                            let before = pointer.x < rect.center().x;
                            let x = if before { rect.left() } else { rect.right() };
                            ui.painter()
                                .vline(x, rect.y_range(), ui.visuals().selection.stroke);

                            if let Some(released) = response.dnd_release_payload::<usize>() {
                                drag_from = Some(*released);
                                drop_to = Some(if before { idx } else { idx + 1 });
                            }
                        }
                    }
                }

                if let (Some(from), Some(to)) = (drag_from, drop_to) {
                    if from < self.panels.len() {
                        let panel = self.panels.remove(from);
                        let to = if to > from { to - 1 } else { to }.min(self.panels.len());
                        self.panels.insert(to, panel);
                        self.settings.panel_order =
                            self.panels.iter().map(|p| p.name().to_string()).collect();
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let hidden = self.settings.hidden.clone();
            let visible_names: Vec<String> = self
                .panels
                .iter()
                .map(|p| p.name().to_string())
                .filter(|n| !hidden.iter().any(|h| h == n))
                .collect();

            let active = self
                .settings
                .active_tab
                .clone()
                .filter(|n| visible_names.contains(n))
                .or_else(|| visible_names.first().cloned());

            match active {
                Some(name) => {
                    self.settings.active_tab = Some(name.clone());
                    if let Some(panel) = self.panels.iter_mut().find(|p| p.name() == name) {
                        ui.heading(panel.name());
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| panel.ui(ui));
                    }
                }
                None => {
                    ui.weak("No panels visible — enable one from the Panels menu.");
                }
            }
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
