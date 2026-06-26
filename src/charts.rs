use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};

/// Draw a 0–100% line chart of a rolling history (e.g. CPU usage).
pub fn percent_plot(ui: &mut egui::Ui, id: &str, history: &[f64]) {
    let points: PlotPoints = history
        .iter()
        .enumerate()
        .map(|(i, &v)| [i as f64, v])
        .collect();

    Plot::new(id)
        .height(150.0)
        .include_y(0.0)
        .include_y(100.0)
        .show(ui, |plot_ui| plot_ui.line(Line::new(points)));
}

/// Draw a labelled progress bar for used/total bytes (e.g. RAM, disks).
pub fn usage_bar(ui: &mut egui::Ui, label: &str, used: u64, total: u64) {
    let fraction = if total > 0 {
        used as f32 / total as f32
    } else {
        0.0
    };

    ui.label(format!(
        "{}: {} / {}",
        label,
        format_bytes(used),
        format_bytes(total)
    ));
    ui.add(egui::ProgressBar::new(fraction).text(format!("{:.0}%", fraction * 100.0)));
}

/// Draw one labelled bar per logical CPU core.
pub fn per_core_bars(ui: &mut egui::Ui, cores: &[f32]) {
    for (i, &usage) in cores.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.monospace(format!("Core {i:>2}"));
            ui.add(
                egui::ProgressBar::new(usage / 100.0)
                    .desired_width(220.0)
                    .text(format!("{usage:.0}%")),
            );
        });
    }
}

/// Draw download/upload throughput history (bytes/sec) as two lines (KB/s).
pub fn network_plot(ui: &mut egui::Ui, down: &[f64], up: &[f64]) {
    let to_kb = |h: &[f64]| -> PlotPoints {
        h.iter()
            .enumerate()
            .map(|(i, &v)| [i as f64, v / 1024.0])
            .collect()
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
