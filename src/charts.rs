use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

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

/// Draw a labelled progress bar for used/total bytes (e.g. RAM).
pub fn usage_bar(ui: &mut egui::Ui, label: &str, used: u64, total: u64) {
    let fraction = if total > 0 {
        used as f32 / total as f32
    } else {
        0.0
    };

    ui.label(format!(
        "{}: {:.1} GB / {:.1} GB",
        label,
        used as f64 / 1e9,
        total as f64 / 1e9
    ));
    ui.add(egui::ProgressBar::new(fraction).text(format!("{:.0}%", fraction * 100.0)));
}
