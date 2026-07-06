use eframe::egui;
use super::Panel;
use crate::charts;
use crate::metrics::SysHandles;

struct DiskInfo {
    name: String,
    used: u64,
    total: u64,
}

#[derive(Default)]
pub struct DisksPanel {
    disks: Vec<DiskInfo>,
}

impl Panel for DisksPanel {
    fn name(&self) -> &str {
        "Disks"
    }

    fn refresh(&mut self, h: &SysHandles) {
        self.disks = h
            .disks
            .iter()
            .map(|d| DiskInfo {
                name: d.mount_point().to_string_lossy().into_owned(),
                total: d.total_space(),
                used: d.total_space().saturating_sub(d.available_space()),
            })
            .collect();
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        for disk in &self.disks {
            charts::usage_bar(ui, &disk.name, disk.used, disk.total);
        }
    }
}
