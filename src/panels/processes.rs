use std::cmp::Ordering;
use eframe::egui;
use super::Panel;
use crate::charts;
use crate::metrics::SysHandles;

struct ProcRow {
    name: String,
    cpu: f32, // percent, may exceed 100 across multiple cores
    mem: u64, // bytes
}

pub struct ProcessesPanel {
    rows: Vec<ProcRow>,
    limit: usize,
}

impl Default for ProcessesPanel {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            limit: 10,
        }
    }
}

impl Panel for ProcessesPanel {
    fn name(&self) -> &str {
        "Top processes"
    }

    fn refresh(&mut self, h: &SysHandles) {
        let mut rows: Vec<ProcRow> = h
            .sys
            .processes()
            .values()
            .map(|p| ProcRow {
                name: p.name().to_string(),
                cpu: p.cpu_usage(),
                mem: p.memory(),
            })
            .collect();

        // Highest CPU first, breaking ties by memory.
        rows.sort_by(|a, b| {
            b.cpu
                .partial_cmp(&a.cpu)
                .unwrap_or(Ordering::Equal)
                .then_with(|| b.mem.cmp(&a.mem))
        });
        rows.truncate(self.limit);
        self.rows = rows;
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("proc_grid")
            .striped(true)
            .num_columns(3)
            .show(ui, |ui| {
                ui.strong("Process");
                ui.strong("CPU %");
                ui.strong("Memory");
                ui.end_row();

                for row in &self.rows {
                    ui.label(&row.name);
                    ui.label(format!("{:.1}", row.cpu));
                    ui.label(charts::format_bytes(row.mem));
                    ui.end_row();
                }
            });
    }
}
