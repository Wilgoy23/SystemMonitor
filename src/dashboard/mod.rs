use eframe::egui;
use crate::metrics::SysHandles;
use crate::widgets::{self, Widget, WidgetSize};

mod layout;
mod registry;

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

/// The dashboard view: owns live widget instances and renders them as a
/// reflowing grid of bubble cards.
pub struct Dashboard {
    instances: Vec<Instance>,
    drag: Option<Drag>,
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
        Self {
            instances,
            drag: None,
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

    /// Render the grid. Returns the name of a linked panel if a widget was
    /// clicked, so the caller can navigate to it. Cards can be dragged to
    /// reorder: the grid reflows to open a slot, positions animate to their
    /// targets, and the new order is committed on drop (persisted by the
    /// caller via `to_entries`). A plain click still opens the linked panel —
    /// egui's click/drag disambiguation means a drag never fires a click.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<String> {
        if self.instances.is_empty() {
            ui.weak("No widgets.");
            return None;
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

        let ctx = ui.ctx().clone();
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
            widgets::card(ui, rect, &title, false, |ui| {
                self.instances[i].widget.ui(ui, size)
            });

            // One interaction per card, sensing both click and drag (a single
            // widget — layering a drag widget over a click one eats the click).
            let resp = ui
                .interact(rect, card_id, egui::Sense::click_and_drag())
                .on_hover_cursor(egui::CursorIcon::Grab);
            if resp.drag_started() {
                if let Some(pp) = resp.interact_pointer_pos() {
                    just_started = Some((id, pp - rect.min));
                }
            }
            if resp.clicked() {
                if let Some(panel) = self.instances[i].widget.linked_panel() {
                    clicked_panel = Some(panel.to_string());
                }
            }
        }

        // Draw the lifted card last so it floats above the grid.
        if let Some((i, fr)) = floating {
            let title = self.instances[i].widget.title();
            let size = self.instances[i].size;
            widgets::card(ui, fr, &title, true, |ui| {
                self.instances[i].widget.ui(ui, size)
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

        // Reserve the grid's footprint so the scroll area sizes correctly.
        let total_h = targets
            .iter()
            .filter(|r| r.min.y.is_finite())
            .map(|r| r.bottom())
            .fold(origin.y, f32::max)
            - origin.y;
        ui.allocate_space(egui::vec2(avail, total_h));

        clicked_panel
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
