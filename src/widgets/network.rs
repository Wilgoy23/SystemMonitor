use eframe::egui;
use super::{Widget, WidgetSize};
use crate::charts;
use crate::history::History;
use crate::metrics::SysHandles;

const HISTORY_LEN: usize = 60;

pub struct NetworkWidget {
    down: f64, // bytes/sec
    up: f64,
    down_history: History,
    up_history: History,
}

impl Default for NetworkWidget {
    fn default() -> Self {
        Self {
            down: 0.0,
            up: 0.0,
            down_history: History::new(HISTORY_LEN),
            up_history: History::new(HISTORY_LEN),
        }
    }
}

impl Widget for NetworkWidget {
    fn kind(&self) -> &'static str {
        "network"
    }

    fn title(&self) -> String {
        "Network".into()
    }

    fn supported_sizes(&self) -> &'static [WidgetSize] {
        &[WidgetSize::Small, WidgetSize::Medium]
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

    fn ui(&mut self, ui: &mut egui::Ui, size: WidgetSize) {
        ui.label(format!("↓ {}/s", charts::format_bytes(self.down as u64)));
        ui.label(format!("↑ {}/s", charts::format_bytes(self.up as u64)));
        if size == WidgetSize::Medium {
            charts::sparkline(ui, "net_widget_spark", &self.down_history, 44.0, false);
        }
    }

    fn linked_panel(&self) -> Option<&'static str> {
        Some("Network")
    }
}
