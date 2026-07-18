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

/// The dashboard view: owns live widget instances and renders them as a
/// reflowing grid of bubble cards.
pub struct Dashboard {
    instances: Vec<Instance>,
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
        Self { instances }
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

    /// Render the grid. M1: static flow layout, no editing.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if self.instances.is_empty() {
            ui.weak("No widgets.");
            return;
        }

        let avail = ui.available_width();
        let columns = (((avail + GAP) / (CELL + GAP)).floor() as usize).max(2);
        let cell = (avail - GAP * (columns as f32 - 1.0)) / columns as f32;

        let footprints: Vec<(u8, u8)> = self.instances.iter().map(|i| i.size.cells()).collect();
        let placements = layout::pack(&footprints, columns);
        let rows = layout::row_count(&placements, &footprints);

        let origin = ui.cursor().min;
        for (instance, (&(col, row), &(wc, hc))) in self
            .instances
            .iter_mut()
            .zip(placements.iter().zip(&footprints))
        {
            let x = origin.x + col as f32 * (cell + GAP);
            let y = origin.y + row as f32 * (cell + GAP);
            let w = wc as f32 * cell + (wc as f32 - 1.0) * GAP;
            let h = hc as f32 * cell + (hc as f32 - 1.0) * GAP;
            let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));

            let title = instance.widget.title();
            let size = instance.size;
            widgets::card(ui, rect, &title, |ui| instance.widget.ui(ui, size));
        }

        // Reserve the grid's footprint so the scroll area sizes correctly.
        let total_h = rows as f32 * cell + (rows.saturating_sub(1)) as f32 * GAP;
        ui.allocate_space(egui::vec2(avail, total_h));
    }
}
