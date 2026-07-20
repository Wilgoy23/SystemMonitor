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

/// Draw a glassmorphic "bubble" card: a soft drop shadow, a translucent
/// frosted fill with a light edge highlight, and a weak title row above a
/// clipped body. `rect` is the full card footprint; the body is laid out
/// inside consistent padding. egui 0.28 has no real backdrop blur, so the
/// "glass" is a translucent fill over the dashboard's gradient backdrop plus
/// a highlight stroke — the look without live refraction.
pub fn card(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    let dark = ui.visuals().dark_mode;
    let rounding = egui::Rounding::same(16.0);

    // Soft drop shadow to lift the glass off the backdrop.
    let shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 20.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(if dark { 110 } else { 45 }),
    };
    ui.painter().add(shadow.as_shape(rect, rounding));

    // Frosted fill + bright edge highlight.
    let fill = if dark {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20)
    } else {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 160)
    };
    let stroke = egui::Stroke::new(
        1.0,
        if dark {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 48)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)
        },
    );
    ui.painter().rect(rect, rounding, fill, stroke);

    // A brighter sheen along the top edge sells the glass.
    let sheen = egui::Color32::from_rgba_unmultiplied(255, 255, 255, if dark { 28 } else { 90 });
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 14.0, rect.top() + 1.0),
            egui::pos2(rect.right() - 14.0, rect.top() + 1.0),
        ],
        egui::Stroke::new(1.0, sheen),
    );

    // The glass fill washes out the default (already-dimmed) text, so force
    // high-contrast body text and a legible—if secondary—title tone.
    let (body_col, title_col) = if dark {
        (egui::Color32::from_gray(236), egui::Color32::from_gray(200))
    } else {
        (egui::Color32::from_gray(20), egui::Color32::from_gray(70))
    };

    let inner = rect.shrink(12.0);
    ui.allocate_ui_at_rect(inner, |ui| {
        ui.set_clip_rect(inner);
        ui.visuals_mut().override_text_color = Some(body_col);
        ui.vertical(|ui| {
            ui.add(
                egui::Label::new(egui::RichText::new(title).color(title_col).small())
                    .truncate(),
            );
            body(ui);
        });
    });
}
