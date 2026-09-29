use std::cmp::Ordering;
use eframe::egui;
use super::{Widget, WidgetSize};
use crate::metrics::SysHandles;

/// Temperature sensors, hottest first. Small shows the single hottest reading;
/// Medium lists the top few. Degrades to an inline "n/a" state when no sensors
/// are reported (common on Windows without elevated access).
#[derive(Default)]
pub struct TempsWidget {
    temps: Vec<(String, f32)>, // (label, °C), hottest first
}

impl Widget for TempsWidget {
    fn kind(&self) -> &'static str {
        "temps"
    }

    fn title(&self) -> String {
        "Temps".into()
    }

    fn supported_sizes(&self) -> &'static [WidgetSize] {
        &[WidgetSize::Small, WidgetSize::Medium]
    }

    fn refresh(&mut self, h: &SysHandles) {
        let mut temps: Vec<(String, f32)> = h
            .components
            .iter()
            .map(|c| (c.label().to_string(), c.temperature()))
            .collect();
        temps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        self.temps = temps;
    }

    fn ui(&mut self, ui: &mut egui::Ui, size: WidgetSize) {
        let Some((label, temp)) = self.temps.first() else {
            ui.weak("n/a");
            ui.weak("No sensors");
            return;
        };

        ui.heading(format!("{temp:.0} °C"));
        ui.weak(label);

        if size == WidgetSize::Medium {
            ui.add_space(2.0);
            for (label, temp) in self.temps.iter().skip(1).take(3) {
                ui.weak(format!("{label}: {temp:.0} °C"));
            }
        }
    }

    fn linked_panel(&self) -> Option<&'static str> {
        Some("Temperatures")
    }

    fn available(&self) -> bool {
        !self.temps.is_empty()
    }
}
