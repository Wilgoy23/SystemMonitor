use eframe::egui;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::Nvml;
use super::Panel;
use crate::charts;
use crate::metrics::SysHandles;

struct NvidiaGpu {
    name: String,
    temp_c: Option<u32>,
    utilization_pct: Option<u32>,
    mem_used: u64,
    mem_total: u64,
    power_w: Option<f32>,
}

struct AmdGpu {
    label: String,
    temp_c: f32,
}

pub struct GpuPanel {
    nvml: Option<Nvml>,
    nvidia: Vec<NvidiaGpu>,
    amd: Vec<AmdGpu>,
}

impl Default for GpuPanel {
    fn default() -> Self {
        Self {
            // NVML dynamically loads nvml.dll at runtime; this is a no-op
            // (returns Err) on machines without an NVIDIA driver installed.
            nvml: Nvml::init().ok(),
            nvidia: Vec::new(),
            amd: Vec::new(),
        }
    }
}

impl Panel for GpuPanel {
    fn name(&self) -> &str {
        "GPU"
    }

    fn refresh(&mut self, h: &SysHandles) {
        self.nvidia.clear();
        if let Some(nvml) = &self.nvml {
            if let Ok(count) = nvml.device_count() {
                for i in 0..count {
                    let Ok(device) = nvml.device_by_index(i) else {
                        continue;
                    };
                    let name = device.name().unwrap_or_else(|_| format!("GPU {i}"));
                    let temp_c = device.temperature(TemperatureSensor::Gpu).ok();
                    let utilization_pct = device.utilization_rates().ok().map(|u| u.gpu);
                    let (mem_used, mem_total) = device
                        .memory_info()
                        .map(|m| (m.used, m.total))
                        .unwrap_or((0, 0));
                    let power_w = device.power_usage().ok().map(|mw| mw as f32 / 1000.0);

                    self.nvidia.push(NvidiaGpu {
                        name,
                        temp_c,
                        utilization_pct,
                        mem_used,
                        mem_total,
                        power_w,
                    });
                }
            }
        }

        // Best-effort AMD/other GPU temps from whatever sensors sysinfo
        // already exposes (same source `TempsPanel` reads).
        self.amd = h
            .components
            .iter()
            .filter(|c| {
                let l = c.label().to_lowercase();
                l.contains("gpu") || l.contains("radeon") || l.contains("amd")
            })
            .map(|c| AmdGpu {
                label: c.label().to_string(),
                temp_c: c.temperature(),
            })
            .collect();
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if self.nvidia.is_empty() && self.amd.is_empty() {
            ui.weak("No supported GPU detected (NVIDIA driver not found, and no GPU-labelled sensors reported).");
            return;
        }

        for gpu in &self.nvidia {
            ui.strong(&gpu.name);
            egui::Grid::new(("gpu_grid", &gpu.name)).num_columns(2).show(ui, |ui| {
                ui.label("Temperature");
                match gpu.temp_c {
                    Some(t) => ui.label(format!("{t} °C")),
                    None => ui.weak("n/a"),
                };
                ui.end_row();

                ui.label("Utilization");
                match gpu.utilization_pct {
                    Some(u) => ui.label(format!("{u}%")),
                    None => ui.weak("n/a"),
                };
                ui.end_row();

                ui.label("Power draw");
                match gpu.power_w {
                    Some(w) => ui.label(format!("{w:.1} W")),
                    None => ui.weak("n/a"),
                };
                ui.end_row();
            });
            if gpu.mem_total > 0 {
                charts::usage_bar(ui, "VRAM", gpu.mem_used, gpu.mem_total);
            }
            ui.separator();
        }

        if !self.amd.is_empty() {
            ui.strong("Other GPU sensors (best-effort)");
            for gpu in &self.amd {
                ui.label(format!("{}: {:.1} °C", gpu.label, gpu.temp_c));
            }
        }
    }
}
