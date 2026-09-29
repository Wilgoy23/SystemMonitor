use eframe::egui;
use super::{Widget, WidgetSize};
use crate::charts;
use crate::metrics::SysHandles;

/// Disk-space usage for one mount. The `mount` config selects which volume;
/// when unset it tracks the first mount reported (per-mount selection lands in
/// M4). Degrades to an inline "n/a" state when no disks are reported.
#[derive(Default)]
pub struct DiskWidget {
    mount: Option<String>, // configured mount point; None = first available
    label: String,
    used: u64,
    total: u64,
    free: u64,
    fs: String,
    available: bool,
}

impl Widget for DiskWidget {
    fn kind(&self) -> &'static str {
        "disk"
    }

    fn title(&self) -> String {
        if self.label.is_empty() {
            "Disk".into()
        } else {
            format!("Disk {}", self.label)
        }
    }

    fn supported_sizes(&self) -> &'static [WidgetSize] {
        &[WidgetSize::Small, WidgetSize::Medium]
    }

    fn refresh(&mut self, h: &SysHandles) {
        let chosen = match &self.mount {
            Some(m) => h
                .disks
                .iter()
                .find(|d| d.mount_point().to_string_lossy() == m.as_str()),
            None => h.disks.iter().next(),
        };

        match chosen {
            Some(d) => {
                self.label = d.mount_point().to_string_lossy().into_owned();
                self.total = d.total_space();
                self.free = d.available_space();
                self.used = self.total.saturating_sub(self.free);
                self.fs = d.file_system().to_string_lossy().into_owned();
                self.available = true;
            }
            None => self.available = false,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, size: WidgetSize) {
        if !self.available {
            ui.weak("n/a");
            ui.weak("No disk");
            return;
        }

        charts::usage_bar(ui, "", self.used, self.total);
        if size == WidgetSize::Medium {
            ui.weak(format!("Free: {}", charts::format_bytes(self.free)));
            if !self.fs.is_empty() {
                ui.weak(&self.fs);
            }
        }
    }

    fn linked_panel(&self) -> Option<&'static str> {
        Some("Storage")
    }
}
