use eframe::egui;
use super::Panel;
use crate::charts;
use crate::metrics::SysHandles;

#[derive(Default)]
pub struct MemoryPanel {
    ram_used: u64,
    ram_total: u64,
    swap_used: u64,
    swap_total: u64,
}

impl Panel for MemoryPanel {
    fn name(&self) -> &str {
        "Memory"
    }

    fn refresh(&mut self, h: &SysHandles) {
        self.ram_used = h.sys.used_memory();
        self.ram_total = h.sys.total_memory();
        self.swap_used = h.sys.used_swap();
        self.swap_total = h.sys.total_swap();
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        charts::usage_bar(ui, "RAM", self.ram_used, self.ram_total);
        charts::usage_bar(ui, "Swap", self.swap_used, self.swap_total);
    }
}
