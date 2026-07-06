use eframe::egui;
use super::Panel;
use crate::charts;
use crate::metrics::SysHandles;

const HISTORY_LEN: usize = 60;

pub struct NetworkPanel {
    down: f64, // bytes/sec
    up: f64,
    down_history: Vec<f64>,
    up_history: Vec<f64>,
}

impl Default for NetworkPanel {
    fn default() -> Self {
        Self {
            down: 0.0,
            up: 0.0,
            down_history: vec![0.0; HISTORY_LEN],
            up_history: vec![0.0; HISTORY_LEN],
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
        push(&mut self.down_history, self.down);
        push(&mut self.up_history, self.up);
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

fn push(buf: &mut Vec<f64>, value: f64) {
    buf.push(value);
    if buf.len() > HISTORY_LEN {
        buf.remove(0);
    }
}
