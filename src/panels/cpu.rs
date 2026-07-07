use eframe::egui;
use super::Panel;
use crate::charts;
use crate::history::History;
use crate::metrics::SysHandles;

const HISTORY_LEN: usize = 60;

pub struct CpuPanel {
    history: History, // rolling global CPU %
    per_core: Vec<f32>,
}

impl Default for CpuPanel {
    fn default() -> Self {
        Self {
            history: History::new(HISTORY_LEN),
            per_core: Vec::new(),
        }
    }
}

impl Panel for CpuPanel {
    fn name(&self) -> &str {
        "CPU"
    }

    fn refresh(&mut self, h: &SysHandles) {
        let cpu = h.sys.global_cpu_info().cpu_usage() as f64;
        self.history.push(cpu);
        self.per_core = h.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("CPU: {:.1}%", self.history.last()));
        charts::percent_plot(ui, "cpu_plot", &self.history);
        ui.collapsing(format!("Per-core ({} cores)", self.per_core.len()), |ui| {
            charts::per_core_bars(ui, &self.per_core);
        });
    }
}
