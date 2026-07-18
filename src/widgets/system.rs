use eframe::egui;
use sysinfo::System;
use super::{Widget, WidgetSize};
use crate::metrics::SysHandles;

#[derive(Default)]
pub struct SystemWidget {
    host: String,
    os: String,
    kernel: String,
    uptime: u64, // seconds
}

impl Widget for SystemWidget {
    fn kind(&self) -> &'static str {
        "system"
    }

    fn title(&self) -> String {
        "Uptime".into()
    }

    fn supported_sizes(&self) -> &'static [WidgetSize] {
        &[WidgetSize::Small, WidgetSize::Medium]
    }

    fn refresh(&mut self, _h: &SysHandles) {
        // Process-wide/static queries, not tied to the System handle.
        self.host = System::host_name().unwrap_or_else(|| "unknown".into());
        self.os = format!(
            "{} {}",
            System::name().unwrap_or_else(|| "unknown".into()),
            System::os_version().unwrap_or_default()
        );
        self.kernel = System::kernel_version().unwrap_or_else(|| "unknown".into());
        self.uptime = System::uptime();
    }

    fn ui(&mut self, ui: &mut egui::Ui, size: WidgetSize) {
        ui.heading(format_uptime(self.uptime));
        if size == WidgetSize::Medium {
            ui.add_space(2.0);
            ui.weak(&self.host);
            ui.weak(self.os.trim().to_string());
            ui.weak(format!("kernel {}", self.kernel));
        }
    }

    fn linked_panel(&self) -> Option<&'static str> {
        Some("System")
    }
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}
