use eframe::egui;
use super::Panel;
use crate::metrics::SysHandles;

#[derive(Default)]
pub struct TempsPanel {
    temps: Vec<(String, f32)>, // (label, °C)
}

impl Panel for TempsPanel {
    fn name(&self) -> &str {
        "Temperatures"
    }

    fn refresh(&mut self, h: &SysHandles) {
        self.temps = h
            .components
            .iter()
            .map(|c| (c.label().to_string(), c.temperature()))
            .collect();
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if self.temps.is_empty() {
            ui.weak("No temperature sensors available (often requires elevated access on Windows).");
        } else {
            for (label, temp) in &self.temps {
                ui.label(format!("{label}: {temp:.1} °C"));
            }
        }
    }
}
