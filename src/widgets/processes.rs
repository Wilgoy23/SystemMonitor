use std::cmp::Ordering;
use eframe::egui;
use super::{Widget, WidgetSize};
use crate::metrics::SysHandles;

/// Top processes by CPU. Medium shows the top 3, Large the top 8. Links to the
/// full processes panel.
#[derive(Default)]
pub struct ProcessesWidget {
    rows: Vec<(String, f32, u64)>, // (name, cpu %, memory bytes), hottest first
}

impl Widget for ProcessesWidget {
    fn kind(&self) -> &'static str {
        "processes"
    }

    fn title(&self) -> String {
        "Top processes".into()
    }

    fn supported_sizes(&self) -> &'static [WidgetSize] {
        &[WidgetSize::Medium, WidgetSize::Large]
    }

    fn refresh(&mut self, h: &SysHandles) {
        let mut rows: Vec<(String, f32, u64)> = h
            .sys
            .processes()
            .values()
            .map(|p| (p.name().to_string(), p.cpu_usage(), p.memory()))
            .collect();
        // Highest CPU first, breaking ties by memory.
        rows.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| b.2.cmp(&a.2))
        });
        rows.truncate(8);
        self.rows = rows;
    }

    fn ui(&mut self, ui: &mut egui::Ui, size: WidgetSize) {
        let n = if size == WidgetSize::Large { 8 } else { 3 };
        egui::Grid::new("proc_widget_grid")
            .num_columns(2)
            .spacing(egui::vec2(10.0, 2.0))
            .show(ui, |ui| {
                for (name, cpu, _mem) in self.rows.iter().take(n) {
                    ui.add(egui::Label::new(name.as_str()).truncate());
                    ui.label(format!("{cpu:.0}%"));
                    ui.end_row();
                }
            });
    }

    fn linked_panel(&self) -> Option<&'static str> {
        Some("Top processes")
    }
}
