use eframe::egui;
use super::{Widget, WidgetSize};
use crate::charts;
use crate::history::History;
use crate::metrics::SysHandles;

const HISTORY_LEN: usize = 60;

pub struct MemoryWidget {
    ram_used: u64,
    ram_total: u64,
    swap_used: u64,
    swap_total: u64,
    history: History, // rolling RAM used %
}

impl Default for MemoryWidget {
    fn default() -> Self {
        Self {
            ram_used: 0,
            ram_total: 0,
            swap_used: 0,
            swap_total: 0,
            history: History::new(HISTORY_LEN),
        }
    }
}

impl Widget for MemoryWidget {
    fn kind(&self) -> &'static str {
        "memory"
    }

    fn title(&self) -> String {
        "Memory".into()
    }

    fn supported_sizes(&self) -> &'static [WidgetSize] {
        &[WidgetSize::Small, WidgetSize::Medium]
    }

    fn refresh(&mut self, h: &SysHandles) {
        self.ram_used = h.sys.used_memory();
        self.ram_total = h.sys.total_memory();
        self.swap_used = h.sys.used_swap();
        self.swap_total = h.sys.total_swap();
        let pct = if self.ram_total > 0 {
            self.ram_used as f64 / self.ram_total as f64 * 100.0
        } else {
            0.0
        };
        self.history.push(pct);
    }

    fn ui(&mut self, ui: &mut egui::Ui, size: WidgetSize) {
        charts::usage_bar(ui, "RAM", self.ram_used, self.ram_total);
        if size == WidgetSize::Medium {
            charts::usage_bar(ui, "Swap", self.swap_used, self.swap_total);
            charts::sparkline(ui, "mem_widget_spark", &self.history, 44.0, true);
        }
    }

    fn linked_panel(&self) -> Option<&'static str> {
        Some("Memory")
    }
}
