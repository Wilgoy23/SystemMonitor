use eframe::egui;
use sysinfo::System;
use super::Panel;
use crate::metrics::SysHandles;

#[derive(Default)]
pub struct SystemPanel {
    host: String,
    os: String,
    kernel: String,
    uptime: u64,      // seconds
    load: (f64, f64, f64), // 1 / 5 / 15 min (zeros on Windows)
}

impl Panel for SystemPanel {
    fn name(&self) -> &str {
        "System"
    }

    fn refresh(&mut self, _h: &SysHandles) {
        // These are process-wide/static queries, not tied to the System handle.
        self.host = System::host_name().unwrap_or_else(|| "unknown".into());
        self.os = format!(
            "{} {}",
            System::name().unwrap_or_else(|| "unknown".into()),
            System::os_version().unwrap_or_default()
        );
        self.kernel = System::kernel_version().unwrap_or_else(|| "unknown".into());
        self.uptime = System::uptime();
        let load = System::load_average();
        self.load = (load.one, load.five, load.fifteen);
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("system_grid").num_columns(2).show(ui, |ui| {
            ui.strong("Host");
            ui.label(&self.host);
            ui.end_row();

            ui.strong("OS");
            ui.label(self.os.trim());
            ui.end_row();

            ui.strong("Kernel");
            ui.label(&self.kernel);
            ui.end_row();

            ui.strong("Uptime");
            ui.label(format_uptime(self.uptime));
            ui.end_row();

            // Load average is only meaningful on Unix; skip when unavailable.
            if self.load.0 > 0.0 || self.load.1 > 0.0 || self.load.2 > 0.0 {
                ui.strong("Load avg");
                ui.label(format!(
                    "{:.2} / {:.2} / {:.2}",
                    self.load.0, self.load.1, self.load.2
                ));
                ui.end_row();
            }
        });
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

#[cfg(test)]
mod tests {
    use super::format_uptime;

    #[test]
    fn formats_uptime_spans() {
        assert_eq!(format_uptime(0), "0m");
        assert_eq!(format_uptime(90), "1m");
        assert_eq!(format_uptime(3_661), "1h 1m");
        assert_eq!(format_uptime(90_061), "1d 1h 1m");
    }
}
