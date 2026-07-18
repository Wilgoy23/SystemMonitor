use eframe::egui;
use crate::metrics::SysHandles;

mod cpu;
mod memory;
mod network;
mod system;

pub use cpu::CpuWidget;
pub use memory::MemoryWidget;
pub use network::NetworkWidget;
pub use system::SystemWidget;

/// The size classes a widget can occupy, expressed as a footprint on the
/// dashboard's square-cell grid. The first entry a kind lists is its default.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WidgetSize {
    Small,
    Medium,
    Large,
}

impl WidgetSize {
    /// Footprint in grid cells as `(width, height)`.
    pub fn cells(self) -> (u8, u8) {
        match self {
            WidgetSize::Small => (1, 1),
            WidgetSize::Medium => (2, 1),
            WidgetSize::Large => (2, 2),
        }
    }
}

/// One live widget instance on the dashboard. Mirrors `Panel` but renders into
/// a fixed-size card and carries per-instance config. Most methods have
/// sensible defaults so simple, single-instance widgets only implement the
/// four required ones.
pub trait Widget {
    /// Stable kind id, e.g. "cpu" — used for (de)serialization via the registry.
    fn kind(&self) -> &'static str;

    /// Title shown in the card header. May reflect config, e.g. "Disk C:".
    fn title(&self) -> String;

    /// Size classes this kind supports (first = default).
    fn supported_sizes(&self) -> &'static [WidgetSize];

    /// Pull data on the shared tick — same contract as `Panel::refresh`.
    fn refresh(&mut self, h: &SysHandles);

    /// Render into a card body of the given size. Must not exceed the rect
    /// (the card frame clips, but content should be designed per size).
    fn ui(&mut self, ui: &mut egui::Ui, size: WidgetSize);

    /// Panel name to open on click, if any.
    fn linked_panel(&self) -> Option<&'static str> {
        None
    }

    /// Serialize per-instance config (interface name, mount, ...).
    fn config(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    /// Restore per-instance config produced by `config`.
    fn set_config(&mut self, _v: &serde_json::Value) {}

    /// Optional config UI shown from the ⚙ badge in edit mode.
    /// Returns true if the config changed.
    fn config_ui(&mut self, _ui: &mut egui::Ui) -> bool {
        false
    }
}

/// Draw a "bubble" card: a rounded, subtly filled rect with a weak title row
/// above a clipped body. `rect` is the full card footprint; the body is laid
/// out inside consistent padding. The shared frame enforces padding and
/// typography so widgets only worry about their content.
pub fn card(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    let visuals = ui.visuals();
    let fill = visuals.faint_bg_color;
    let stroke = visuals.widgets.noninteractive.bg_stroke;
    ui.painter()
        .rect(rect, egui::Rounding::same(16.0), fill, stroke);

    let inner = rect.shrink(12.0);
    ui.allocate_ui_at_rect(inner, |ui| {
        ui.set_clip_rect(inner);
        ui.vertical(|ui| {
            ui.add(egui::Label::new(egui::RichText::new(title).weak().small()).truncate());
            body(ui);
        });
    });
}
