use std::time::{Duration, Instant};
use eframe::egui;
use crate::metrics::SysHandles;
use crate::widgets::{self, Widget, WidgetSize};

mod layout;
pub mod registry;

/// How long the undo toast lingers after a widget is removed.
const UNDO_WINDOW: Duration = Duration::from_secs(5);

/// Target edge length of one grid cell at 1.0 UI scale.
const CELL: f32 = 150.0;
/// Gap between cells, in points.
const GAP: f32 = 12.0;

/// A persisted widget placement. The dashboard's display order is the vec
/// order in `Settings`. Additive/`serde(default)` fields keep old blobs loading.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct WidgetEntry {
    pub kind: String, // registry id
    pub id: u64,      // instance id, unique within the layout
    pub size: WidgetSize,
    #[serde(default)]
    pub config: serde_json::Value,
}

/// A live widget: its persisted metadata plus the constructed instance.
struct Instance {
    id: u64,
    size: WidgetSize,
    widget: Box<dyn Widget>,
}

/// In-progress drag of a widget card. `grab_offset` is the pointer's position
/// relative to the card's top-left at grab time, so the card stays pinned under
/// the cursor as it moves.
struct Drag {
    id: u64,
    grab_offset: egui::Vec2,
}

/// A just-removed widget, held so the undo toast can restore it at its original
/// position for a few seconds.
struct Undo {
    entry: WidgetEntry,
    index: usize,
    at: Instant,
}

/// What the dashboard is asking the host app to do this frame.
#[derive(Default)]
pub struct DashboardAction {
    /// Open this panel (a widget was clicked in normal mode).
    pub open_panel: Option<String>,
    /// Open the widget gallery (the empty-state Add button was clicked).
    pub open_gallery: bool,
}

/// The dashboard view: owns live widget instances and renders them as a
/// reflowing grid of bubble cards.
pub struct Dashboard {
    instances: Vec<Instance>,
    drag: Option<Drag>,
    /// Edit mode shows per-card remove/resize badges and suppresses click-through.
    pub edit: bool,
    undo: Option<Undo>,
    /// Next instance id to hand out; kept unique within the live layout.
    next_id: u64,
}

impl Dashboard {
    /// Build live instances from persisted entries, skipping any whose kind is
    /// unknown to this build (forward compatibility).
    pub fn from_entries(entries: &[WidgetEntry]) -> Self {
        let instances = entries
            .iter()
            .filter_map(|e| {
                let mut widget = registry::make(&e.kind)?;
                widget.set_config(&e.config);
                Some(Instance {
                    id: e.id,
                    size: e.size,
                    widget,
                })
            })
            .collect();
        let next_id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        Self {
            instances,
            drag: None,
            edit: false,
            undo: None,
            next_id,
        }
    }

    /// The default widget set for a fresh install: CPU, Memory, Network, Uptime.
    pub fn default_layout() -> Vec<WidgetEntry> {
        [
            ("cpu", WidgetSize::Small),
            ("memory", WidgetSize::Small),
            ("network", WidgetSize::Medium),
            ("system", WidgetSize::Small),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (kind, size))| WidgetEntry {
            kind: kind.into(),
            id: i as u64 + 1,
            size,
            config: serde_json::Value::Null,
        })
        .collect()
    }

    /// Serialize the current layout back to persistable entries.
    pub fn to_entries(&self) -> Vec<WidgetEntry> {
        self.instances
            .iter()
            .map(|i| WidgetEntry {
                kind: i.widget.kind().into(),
                id: i.id,
                size: i.size,
                config: i.widget.config(),
            })
            .collect()
    }

    /// Refresh every widget on the shared tick.
    pub fn refresh(&mut self, h: &SysHandles) {
        for i in &mut self.instances {
            i.widget.refresh(h);
        }
    }

    /// Append a widget of `kind` at its default size and enter edit mode so the
    /// user can place it. No-op for unknown kinds.
    pub fn add(&mut self, kind: &str) {
        if let Some(widget) = registry::make(kind) {
            let size = widget
                .supported_sizes()
                .first()
                .copied()
                .unwrap_or(WidgetSize::Small);
            let id = self.next_id;
            self.next_id += 1;
            self.instances.push(Instance { id, size, widget });
            self.edit = true;
        }
    }

    /// How many live instances of `kind` exist (for the gallery's "Added" state).
    pub fn count_of(&self, kind: &str) -> usize {
        self.instances.iter().filter(|i| i.widget.kind() == kind).count()
    }

    /// Restore the default widget set and order, leaving edit mode.
    pub fn reset(&mut self) {
        let next_id = self.next_id;
        *self = Dashboard::from_entries(&Dashboard::default_layout());
        // Preserve the id counter so a later undo can't collide with a reused id.
        self.next_id = next_id.max(self.next_id);
    }

    /// Remove the instance at `index`, stashing it for the undo toast.
    fn remove_at(&mut self, index: usize) {
        if index >= self.instances.len() {
            return;
        }
        let inst = self.instances.remove(index);
        self.undo = Some(Undo {
            entry: WidgetEntry {
                kind: inst.widget.kind().into(),
                id: inst.id,
                size: inst.size,
                config: inst.widget.config(),
            },
            index,
            at: Instant::now(),
        });
    }

    /// Restore the most recently removed widget to its original position.
    fn apply_undo(&mut self) {
        if let Some(u) = self.undo.take() {
            if let Some(mut widget) = registry::make(&u.entry.kind) {
                widget.set_config(&u.entry.config);
                let idx = u.index.min(self.instances.len());
                self.instances.insert(
                    idx,
                    Instance {
                        id: u.entry.id,
                        size: u.entry.size,
                        widget,
                    },
                );
            }
        }
    }

    /// Cycle the instance at `index` to the next size its kind supports.
    fn cycle_size(&mut self, index: usize) {
        let Some(inst) = self.instances.get_mut(index) else {
            return;
        };
        let sizes = inst.widget.supported_sizes();
        let pos = sizes.iter().position(|&s| s == inst.size).unwrap_or(0);
        inst.size = sizes[(pos + 1) % sizes.len().max(1)];
    }

    /// Render the grid and report what the app should do this frame. Cards can
    /// be dragged to reorder (reflow + animate + commit on drop). In normal
    /// mode a plain click opens the linked panel; in edit mode each card shows
    /// remove/resize badges and click-through is suppressed. A removed widget
    /// can be restored from the undo toast for a few seconds.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> DashboardAction {
        let mut action = DashboardAction::default();
        let ctx = ui.ctx().clone();

        // Undo toast (drawn on its own layer so it floats over everything, and
        // survives even the empty state so a last-widget removal is reversible).
        self.undo_toast(&ctx);

        if self.instances.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(egui::RichText::new("Your dashboard is empty").size(18.0));
                ui.add_space(6.0);
                ui.weak("Add a widget to start monitoring at a glance.");
                ui.add_space(12.0);
                if ui.button("+ Add widget").clicked() {
                    action.open_gallery = true;
                }
            });
            return action;
        }

        let avail = ui.available_width();
        let columns = (((avail + GAP) / (CELL + GAP)).floor() as usize).max(2);
        let cell = (avail - GAP * (columns as f32 - 1.0)) / columns as f32;
        let origin = ui.cursor().min;

        let n = self.instances.len();
        let footprints: Vec<(u8, u8)> =
            self.instances.iter().map(|i| i.size.cells()).collect();

        // Turn a grid slot + footprint into an absolute rect.
        let rect_of = |(col, row): (usize, usize), (wc, hc): (u8, u8)| {
            let x = origin.x + col as f32 * (cell + GAP);
            let y = origin.y + row as f32 * (cell + GAP);
            let w = wc as f32 * cell + (wc as f32 - 1.0) * GAP;
            let h = hc as f32 * cell + (hc as f32 - 1.0) * GAP;
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
        };

        let pointer = ctx.pointer_interact_pos();

        // Base layout in the current order; the reference for hit-testing the
        // pointer against slots while dragging.
        let base = layout::pack(&footprints, columns);
        let dragged_idx = self
            .drag
            .as_ref()
            .and_then(|d| self.instances.iter().position(|i| i.id == d.id));

        // Insertion index (position among the *other* cards) derived from the
        // pointer in reading order: count the cards it has passed.
        let insertion = match (dragged_idx, pointer) {
            (Some(di), Some(p)) => {
                let mut k = 0usize;
                for i in 0..n {
                    if i == di {
                        continue;
                    }
                    let r = rect_of(base[i], footprints[i]);
                    let past = p.y > r.bottom() || (p.y >= r.top() && p.x > r.center().x);
                    if past {
                        k += 1;
                    }
                }
                Some(k)
            }
            _ => None,
        };

        // Target rects: while dragging, repack with the dragged card moved to
        // the insertion slot so the rest reflow to make room.
        let targets: Vec<egui::Rect> = if let (Some(di), Some(k)) = (dragged_idx, insertion) {
            let mut order: Vec<usize> = (0..n).filter(|&i| i != di).collect();
            order.insert(k.min(order.len()), di);
            let ordered_fp: Vec<(u8, u8)> = order.iter().map(|&i| footprints[i]).collect();
            let placed = layout::pack(&ordered_fp, columns);
            let mut t = vec![egui::Rect::NOTHING; n];
            for (slot, &i) in order.iter().enumerate() {
                t[i] = rect_of(placed[slot], footprints[i]);
            }
            t
        } else {
            (0..n).map(|i| rect_of(base[i], footprints[i])).collect()
        };

        let rounding = egui::Rounding::same(16.0);
        let sel = ui.visuals().selection.stroke.color;

        let mut clicked_panel = None;
        let mut just_started: Option<(u64, egui::Vec2)> = None;
        let mut stopped = false;
        let mut floating: Option<(usize, egui::Rect)> = None;
        let mut remove_index: Option<usize> = None;
        let mut size_change: Option<usize> = None;

        for i in 0..n {
            let card_id = egui::Id::new(("widget_card", self.instances[i].id));

            if Some(i) == dragged_idx {
                // Ghost outline marking the slot the card will drop into.
                ui.painter().rect_stroke(
                    targets[i],
                    rounding,
                    egui::Stroke::new(1.5, sel),
                );
                // The lifted card follows the pointer; drawn last so it stays
                // on top of the others. We still interact here to keep the
                // drag alive and catch its release.
                let off = self.drag.as_ref().map(|d| d.grab_offset).unwrap_or_default();
                let min = pointer.map(|p| p - off).unwrap_or(targets[i].min);
                let fr = egui::Rect::from_min_size(min, targets[i].size());
                let resp = ui
                    .interact(fr, card_id, egui::Sense::click_and_drag())
                    .on_hover_cursor(egui::CursorIcon::Grabbing);
                if resp.drag_stopped() {
                    stopped = true;
                }
                floating = Some((i, fr));
                continue;
            }

            // Non-dragged cards animate toward their target slots.
            let id = self.instances[i].id;
            let t = targets[i];
            let ax = ctx.animate_value_with_time(egui::Id::new((id, "x")), t.min.x, 0.12);
            let ay = ctx.animate_value_with_time(egui::Id::new((id, "y")), t.min.y, 0.12);
            let rect = egui::Rect::from_min_size(egui::pos2(ax, ay), t.size());

            let title = self.instances[i].widget.title();
            let size = self.instances[i].size;
            // Scope each body under the instance id so inner ids (plot handles,
            // grids) stay unique across instances and the gallery previews.
            widgets::card(ui, rect, &title, false, |ui| {
                ui.push_id(id, |ui| self.instances[i].widget.ui(ui, size));
            });

            // Edit-mode badge rects (also used to keep a badge press from
            // starting a card drag).
            let has_size_toggle = self.instances[i].widget.supported_sizes().len() > 1;
            let rm_rect = egui::Rect::from_min_size(rect.min + egui::vec2(6.0, 6.0), egui::vec2(22.0, 22.0));
            let sz_rect = egui::Rect::from_min_size(rect.max - egui::vec2(30.0, 28.0), egui::vec2(24.0, 22.0));

            // One interaction per card, sensing both click and drag (a single
            // widget — layering a drag widget over a click one eats the click).
            let resp = ui
                .interact(rect, card_id, egui::Sense::click_and_drag())
                .on_hover_cursor(if self.edit {
                    egui::CursorIcon::Grab
                } else {
                    egui::CursorIcon::PointingHand
                });
            if resp.drag_started() {
                if let Some(pp) = resp.interact_pointer_pos() {
                    let on_badge = self.edit
                        && (rm_rect.contains(pp) || (has_size_toggle && sz_rect.contains(pp)));
                    if !on_badge {
                        just_started = Some((id, pp - rect.min));
                    }
                }
            }
            // Click-through only in normal mode; edit mode is for arranging.
            if !self.edit && resp.clicked() {
                if let Some(panel) = self.instances[i].widget.linked_panel() {
                    clicked_panel = Some(panel.to_string());
                }
            }

            // Edit-mode badges, drawn (and interacted) after the card so they
            // sit on top and win the click.
            if self.edit {
                if ui
                    .put(
                        rm_rect,
                        egui::Button::new(egui::RichText::new("×").size(15.0))
                            .rounding(11.0),
                    )
                    .on_hover_text("Remove")
                    .clicked()
                {
                    remove_index = Some(i);
                }
                if has_size_toggle {
                    let letter = match self.instances[i].size {
                        WidgetSize::Small => "S",
                        WidgetSize::Medium => "M",
                        WidgetSize::Large => "L",
                    };
                    if ui
                        .put(sz_rect, egui::Button::new(letter).rounding(6.0))
                        .on_hover_text("Cycle size")
                        .clicked()
                    {
                        size_change = Some(i);
                    }
                }
            }
        }

        // Draw the lifted card last so it floats above the grid.
        if let Some((i, fr)) = floating {
            let id = self.instances[i].id;
            let title = self.instances[i].widget.title();
            let size = self.instances[i].size;
            widgets::card(ui, fr, &title, true, |ui| {
                ui.push_id(id, |ui| self.instances[i].widget.ui(ui, size));
            });
        }

        // Apply drag lifecycle transitions after the layout pass.
        if let Some((id, grab_offset)) = just_started {
            self.drag = Some(Drag { id, grab_offset });
        }
        if stopped {
            if let (Some(di), Some(k)) = (dragged_idx, insertion) {
                let item = self.instances.remove(di);
                self.instances.insert(k.min(self.instances.len()), item);
            }
            self.drag = None;
        }
        if self.drag.is_some() {
            ctx.request_repaint(); // keep following the pointer smoothly
        }

        // Edit-mode mutations, applied after the layout pass so indices stay valid.
        if let Some(i) = size_change {
            self.cycle_size(i);
        }
        if let Some(i) = remove_index {
            self.remove_at(i);
        }

        // Reserve the grid's footprint so the scroll area sizes correctly.
        let total_h = targets
            .iter()
            .filter(|r| r.min.y.is_finite())
            .map(|r| r.bottom())
            .fold(origin.y, f32::max)
            - origin.y;
        ui.allocate_space(egui::vec2(avail, total_h));

        action.open_panel = clicked_panel;
        action
    }

    /// Draw the "widget removed — Undo" toast when a removal is pending, and
    /// expire it after `UNDO_WINDOW`. Applies the undo if the button is hit.
    fn undo_toast(&mut self, ctx: &egui::Context) {
        let Some(undo) = &self.undo else {
            return;
        };
        if undo.at.elapsed() >= UNDO_WINDOW {
            self.undo = None;
            return;
        }

        let mut do_undo = false;
        egui::Area::new(egui::Id::new("dashboard_undo_toast"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -28.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Widget removed");
                        if ui.button("Undo").clicked() {
                            do_undo = true;
                        }
                    });
                });
            });
        // Keep the frame ticking so the toast can expire on time.
        ctx.request_repaint_after(Duration::from_millis(250));

        if do_undo {
            self.apply_undo();
        }
    }
}

/// Paint a subtle vertical gradient across `rect`, giving the translucent glass
/// cards something to sit over so their frosted fill actually reads. Theme-aware.
pub fn paint_backdrop(painter: &egui::Painter, rect: egui::Rect, dark: bool) {
    use egui::epaint::{Mesh, Vertex};
    let (top, bottom) = if dark {
        (
            egui::Color32::from_rgb(34, 40, 58),
            egui::Color32::from_rgb(14, 16, 24),
        )
    } else {
        (
            egui::Color32::from_rgb(238, 242, 250),
            egui::Color32::from_rgb(210, 218, 233),
        )
    };
    let uv = egui::epaint::WHITE_UV;
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex { pos: rect.left_top(), uv, color: top });
    mesh.vertices.push(Vertex { pos: rect.right_top(), uv, color: top });
    mesh.vertices.push(Vertex { pos: rect.right_bottom(), uv, color: bottom });
    mesh.vertices.push(Vertex { pos: rect.left_bottom(), uv, color: bottom });
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    painter.add(mesh);
}
