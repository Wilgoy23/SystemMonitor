use eframe::egui;
use crate::dashboard::{registry, Dashboard, WidgetEntry};
use crate::metrics::Source;
use crate::panels::{self, Panel};
use crate::widgets::{Widget, WidgetSize};
use std::time::{Duration, Instant};

/// The pseudo-tab that shows the widget dashboard. Not a `Panel`; handled
/// specially in the tab bar and central panel. No panel is named this.
const DASHBOARD_TAB: &str = "Dashboard";

/// User-tweakable settings, persisted across runs via eframe storage.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(default)]
struct Settings {
    refresh_ms: u64,
    paused: bool,
    hidden: Vec<String>,       // panel names the user has hidden
    panel_order: Vec<String>,  // user-defined tab order, by panel name
    active_tab: Option<String>, // last-selected tab
    widgets: Vec<WidgetEntry>, // dashboard layout; vec order == display order
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_ms: 1500,
            paused: false,
            hidden: Vec::new(),
            panel_order: Vec::new(),
            active_tab: None,
            widgets: Vec::new(),
        }
    }
}

pub struct App {
    source: Source,
    panels: Vec<Box<dyn Panel>>,
    dashboard: Dashboard,
    settings: Settings,
    last_refresh: Instant,
    /// Whether the widget gallery window is open.
    gallery_open: bool,
    /// Live preview widgets, one per registry kind, aligned with the catalog.
    /// Built lazily while the gallery is open and refreshed on the shared tick.
    previews: Vec<Box<dyn Widget>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut settings = cc
            .storage
            .and_then(|s| eframe::get_value::<Settings>(s, eframe::APP_KEY))
            .unwrap_or_default();

        // Fresh install: seed the default dashboard and open on it.
        if settings.widgets.is_empty() {
            settings.widgets = Dashboard::default_layout();
        }
        if settings.active_tab.is_none() {
            settings.active_tab = Some(DASHBOARD_TAB.to_string());
        }

        let mut source = Source::new();
        let mut panels = panels::default_panels();
        let mut dashboard = Dashboard::from_entries(&settings.widgets);

        // Apply the user's saved tab order, if any. Panels not mentioned
        // (e.g. newly added ones) keep their natural position at the end.
        if !settings.panel_order.is_empty() {
            let order = &settings.panel_order;
            panels.sort_by_key(|p| order.iter().position(|n| n == p.name()).unwrap_or(usize::MAX));
        }

        // Prime every panel and widget once so the first frame has data.
        let handles = source.refresh();
        for panel in &mut panels {
            panel.refresh(&handles);
        }
        dashboard.refresh(&handles);

        Self {
            source,
            panels,
            dashboard,
            settings,
            last_refresh: Instant::now(),
            gallery_open: false,
            previews: Vec::new(),
        }
    }

    /// The widget gallery: a modal listing every registry kind with a live
    /// preview, an Add button (or "Added" for single-instance kinds already
    /// present), an unavailable marker, and a Reset-layout action.
    fn gallery_window(&mut self, ctx: &egui::Context) {
        let mut open = self.gallery_open;
        egui::Window::new("Add widget")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_width(430.0);
                egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                    for (i, kind) in registry::catalog().iter().enumerate() {
                        ui.horizontal(|ui| {
                            // Live-ish preview at Small size in a fixed, clipped
                            // box. Reserve the box with allocate_exact_size (so
                            // the row advances by exactly the box width), then
                            // render the widget into a detached child_ui whose
                            // content size never feeds back into this layout —
                            // otherwise a wide label/bar would stretch the row.
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(120.0, 66.0), egui::Sense::hover());
                            ui.painter().rect(
                                rect,
                                egui::Rounding::same(8.0),
                                ui.visuals().faint_bg_color,
                                ui.visuals().widgets.noninteractive.bg_stroke,
                            );
                            let inner = rect.shrink(6.0);
                            let mut child =
                                ui.child_ui(inner, egui::Layout::top_down(egui::Align::Min), None);
                            child.set_clip_rect(inner.intersect(ui.clip_rect()));
                            if let Some(preview) = self.previews.get_mut(i) {
                                child.push_id(("gallery_preview", kind.id), |ui| {
                                    preview.ui(ui, WidgetSize::Small);
                                });
                            }
                            ui.add_space(8.0);

                            ui.vertical(|ui| {
                                ui.strong(kind.name);
                                ui.weak(kind.description);
                                let unavailable = self
                                    .previews
                                    .get(i)
                                    .map(|w| !w.available())
                                    .unwrap_or(false);
                                if unavailable {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(0xD9, 0x8A, 0x3A),
                                        "unavailable on this system",
                                    );
                                }
                                ui.add_space(2.0);
                                let already =
                                    !kind.multi_instance && self.dashboard.count_of(kind.id) > 0;
                                if already {
                                    ui.add_enabled(false, egui::Button::new("Added"));
                                } else if ui.button("Add").clicked() {
                                    self.dashboard.add(kind.id);
                                }
                            });
                        });
                        ui.separator();
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Reset layout").clicked() {
                        self.dashboard.reset();
                    }
                });
            });
        self.gallery_open = open;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The dashboard is the standalone full-screen home; the panel tab
        // strip only appears once the user has drilled into the detail layer.
        let on_home = self
            .settings
            .active_tab
            .as_deref()
            .map_or(true, |t| t == DASHBOARD_TAB);

        // Gallery previews are only needed while the gallery is open; build them
        // lazily (this also defers the GPU widget's NVML init until first open).
        if self.gallery_open && self.previews.is_empty() {
            self.previews = registry::catalog().iter().map(|k| (k.factory)()).collect();
        } else if !self.gallery_open && !self.previews.is_empty() {
            self.previews.clear();
        }

        if !self.settings.paused
            && self.last_refresh.elapsed() >= Duration::from_millis(self.settings.refresh_ms)
        {
            let handles = self.source.refresh();
            for panel in &mut self.panels {
                panel.refresh(&handles);
            }
            self.dashboard.refresh(&handles);
            if self.gallery_open {
                for preview in &mut self.previews {
                    preview.refresh(&handles);
                }
            }
            self.last_refresh = Instant::now();
        }

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.settings.paused, "⏸ Pause");
                ui.separator();
                ui.label("Refresh:");
                ui.add(egui::Slider::new(&mut self.settings.refresh_ms, 250..=5000).suffix(" ms"));
                ui.separator();
                ui.menu_button("Panels ⏷", |ui| {
                    for panel in &self.panels {
                        let name = panel.name().to_string();
                        let mut shown = !self.settings.hidden.iter().any(|h| h == &name);
                        if ui.checkbox(&mut shown, &name).changed() {
                            if shown {
                                self.settings.hidden.retain(|h| h != &name);
                            } else {
                                self.settings.hidden.push(name);
                            }
                        }
                    }
                });

                // Dashboard-only controls: add widgets and toggle edit mode.
                if on_home {
                    ui.separator();
                    if ui.button("+ Add widget").clicked() {
                        self.gallery_open = true;
                    }
                    let edit_label = if self.dashboard.edit { "Done" } else { "Edit" };
                    if ui
                        .selectable_label(self.dashboard.edit, edit_label)
                        .clicked()
                    {
                        self.dashboard.edit = !self.dashboard.edit;
                    }
                }
            });
        });

        if !on_home {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Return to the standalone dashboard home.
                if ui.add(egui::Button::new("⌂ Home")).clicked() {
                    self.settings.active_tab = Some(DASHBOARD_TAB.to_string());
                }
                ui.separator();

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
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Recompute rather than reuse `on_home`: the Home button in the
            // tab strip (rendered above, this same frame) may have just
            // switched us back to the dashboard, and the panel fallback below
            // would otherwise clobber `active_tab` with the first panel name.
            let is_home = self
                .settings
                .active_tab
                .as_deref()
                .map_or(true, |t| t == DASHBOARD_TAB);
            if is_home {
                // Full-screen standalone home: gradient backdrop behind a
                // reflowing grid of glass bubbles. Clicking one drills into
                // its panel (the detail layer).
                // Esc leaves edit mode (matches the Done button).
                if self.dashboard.edit && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.dashboard.edit = false;
                }

                let backdrop = ui.max_rect();
                crate::dashboard::paint_backdrop(ui.painter(), backdrop, ui.visuals().dark_mode);
                egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        let action = self.dashboard.ui(ui);
                        if let Some(panel) = action.open_panel {
                            self.settings.active_tab = Some(panel);
                        }
                        if action.open_gallery {
                            self.gallery_open = true;
                        }
                    });
                return;
            }

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

        if self.gallery_open {
            self.gallery_window(ctx);
        }

        // Only keep animating while actively refreshing.
        if !self.settings.paused {
            ctx.request_repaint_after(Duration::from_millis(self.settings.refresh_ms));
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.settings.widgets = self.dashboard.to_entries();
        eframe::set_value(storage, eframe::APP_KEY, &self.settings);
    }
}
