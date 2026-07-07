use eframe::egui;
use super::Panel;
use crate::charts;
use crate::history::History;
use crate::metrics::SysHandles;

const HISTORY_LEN: usize = 60;

pub struct NetworkPanel {
    down: f64, // bytes/sec
    up: f64,
    down_history: History,
    up_history: History,
}

impl Default for NetworkPanel {
    fn default() -> Self {
        Self {
            down: 0.0,
            up: 0.0,
            down_history: History::new(HISTORY_LEN),
            up_history: History::new(HISTORY_LEN),
        }
    }
}

impl Panel for NetworkPanel {
    fn name(&self) -> &str {
        "Network"
    }

    fn refresh(&mut self, h: &SysHandles) {
        let (mut down, mut up) = (0u64, 0u64);
        for (_name, data) in h.networks.iter() {
            down += data.received();
            up += data.transmitted();
        }
        self.down = down as f64 / h.elapsed;
        self.up = up as f64 / h.elapsed;
        self.down_history.push(self.down);
        self.up_history.push(self.up);
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "down {}/s · up {}/s",
            charts::format_bytes(self.down as u64),
            charts::format_bytes(self.up as u64)
        ));
        charts::network_plot(ui, &self.down_history, &self.up_history);
    }
}
