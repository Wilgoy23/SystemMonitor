use eframe::egui;
use egui::Color32;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use crate::history::History;

/// Draw a 0–100% line chart of a rolling history (e.g. CPU usage).
pub fn percent_plot(ui: &mut egui::Ui, id: &str, history: &History) {
    let points: PlotPoints = history
        .iter()
        .enumerate()
        .map(|(i, v)| [i as f64, v])
        .collect();

    Plot::new(id)
        .height(150.0)
        .include_y(0.0)
        .include_y(100.0)
        .show(ui, |plot_ui| plot_ui.line(Line::new(points)));
}

/// Draw a labelled progress bar for used/total bytes (e.g. RAM, disks),
/// tinted amber/red as it approaches full.
pub fn usage_bar(ui: &mut egui::Ui, label: &str, used: u64, total: u64) {
    let fraction = if total > 0 {
        used as f32 / total as f32
    } else {
        0.0
    };

    if label.is_empty() {
        ui.label(format!("{} / {}", format_bytes(used), format_bytes(total)));
    } else {
        ui.label(format!(
            "{}: {} / {}",
            label,
            format_bytes(used),
            format_bytes(total)
        ));
    }

    let mut bar = egui::ProgressBar::new(fraction).text(format!("{:.0}%", fraction * 100.0));
    if let Some(color) = threshold_color(fraction) {
        bar = bar.fill(color);
    }
    ui.add(bar);
}

/// A bare progress bar (no leading label line) showing `part` as a fraction
/// of `whole` — used where the name/size are already shown in a grid column
/// and repeating them via `usage_bar` would be redundant.
pub fn size_bar(ui: &mut egui::Ui, part: u64, whole: u64) {
    let fraction = if whole > 0 {
        (part as f32 / whole as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut bar = egui::ProgressBar::new(fraction).desired_width(80.0).show_percentage();
    if let Some(color) = threshold_color(fraction) {
        bar = bar.fill(color);
    }
    ui.add(bar);
}

/// Draw one labelled bar per logical CPU core, tinted by load.
pub fn per_core_bars(ui: &mut egui::Ui, cores: &[f32]) {
    for (i, &usage) in cores.iter().enumerate() {
        let fraction = usage / 100.0;
        ui.horizontal(|ui| {
            ui.monospace(format!("Core {i:>2}"));
            let mut bar = egui::ProgressBar::new(fraction)
                .desired_width(220.0)
                .text(format!("{usage:.0}%"));
            if let Some(color) = threshold_color(fraction) {
                bar = bar.fill(color);
            }
            ui.add(bar);
        });
    }
}

/// Draw download/upload throughput history (bytes/sec) as two lines (KB/s).
pub fn network_plot(ui: &mut egui::Ui, down: &History, up: &History) {
    let to_kb = |h: &History| -> PlotPoints {
        h.iter().enumerate().map(|(i, v)| [i as f64, v / 1024.0]).collect()
    };

    Plot::new("net_plot")
        .height(150.0)
        .include_y(0.0)
        .legend(Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new(to_kb(down)).name("Down KB/s"));
            plot_ui.line(Line::new(to_kb(up)).name("Up KB/s"));
        });
}

/// Amber past 75%, red past 90%, default fill below that.
fn threshold_color(fraction: f32) -> Option<Color32> {
    if fraction >= 0.90 {
        Some(Color32::from_rgb(0xD9, 0x3A, 0x3A))
    } else if fraction >= 0.75 {
        Some(Color32::from_rgb(0xD9, 0x8A, 0x3A))
    } else {
        None
    }
}

/// Format a byte count into a human-readable string (KB/MB/GB...).
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn formats_byte_scales() {
        assert_eq!(format_bytes(512), "512.0 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
