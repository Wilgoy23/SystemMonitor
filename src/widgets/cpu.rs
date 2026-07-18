use eframe::egui;
use super::{Widget, WidgetSize};
use crate::charts;
use crate::history::History;
use crate::metrics::SysHandles;

const HISTORY_LEN: usize = 60;

pub struct CpuWidget {
    history: History, // rolling global CPU %
    per_core: Vec<f32>,
}

impl Default for CpuWidget {
    fn default() -> Self {
        Self {
            history: History::new(HISTORY_LEN),
            per_core: Vec::new(),
        }
    }
}

impl Widget for CpuWidget {
    fn kind(&self) -> &'static str {
        "cpu"
    }

    fn title(&self) -> String {
        "CPU".into()
    }

    fn supported_sizes(&self) -> &'static [WidgetSize] {
        &[WidgetSize::Small, WidgetSize::Medium, WidgetSize::Large]
    }

    fn refresh(&mut self, h: &SysHandles) {
        self.history.push(h.sys.global_cpu_info().cpu_usage() as f64);
        self.per_core = h.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
    }

    fn ui(&mut self, ui: &mut egui::Ui, size: WidgetSize) {
        ui.heading(format!("{:.0}%", self.history.last()));
        let height = if size == WidgetSize::Small { 40.0 } else { 60.0 };
        charts::sparkline(ui, "cpu_widget_spark", &self.history, height, true);

        if size == WidgetSize::Large {
            ui.add_space(4.0);
            charts::per_core_bars(ui, &self.per_core);
        }
    }

    fn linked_panel(&self) -> Option<&'static str> {
        Some("CPU")
    }
}
