use eframe::egui;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::Nvml;
use super::{Widget, WidgetSize};
use crate::charts;
use crate::history::History;
use crate::metrics::SysHandles;

const HISTORY_LEN: usize = 60;

/// GPU utilization + VRAM. Prefers an NVIDIA GPU via NVML; falls back to a
/// GPU-labelled temperature sensor (AMD/other) for temp only. Degrades to an
/// inline "n/a" state when no GPU is detected.
pub struct GpuWidget {
    // NVML dynamically loads nvml.dll at runtime; `None` on machines without
    // an NVIDIA driver, in which case we fall back to sysinfo sensors.
    nvml: Option<Nvml>,
    available: bool,
    name: String,
    util: Option<u32>,
    mem_used: u64,
    mem_total: u64,
    temp_c: Option<u32>,
    power_w: Option<f32>,
    history: History, // utilization %
}

impl Default for GpuWidget {
    fn default() -> Self {
        Self {
            nvml: Nvml::init().ok(),
            available: false,
            name: String::new(),
            util: None,
            mem_used: 0,
            mem_total: 0,
            temp_c: None,
            power_w: None,
            history: History::new(HISTORY_LEN),
        }
    }
}

impl Widget for GpuWidget {
    fn kind(&self) -> &'static str {
        "gpu"
    }

    fn title(&self) -> String {
        if self.name.is_empty() {
            "GPU".into()
        } else {
            self.name.clone()
        }
    }

    fn supported_sizes(&self) -> &'static [WidgetSize] {
        &[WidgetSize::Small, WidgetSize::Medium]
    }

    fn refresh(&mut self, h: &SysHandles) {
        // Prefer the first NVIDIA GPU when NVML is present and reports one.
        if let Some(nvml) = &self.nvml {
            if let Ok(device) = nvml.device_by_index(0) {
                self.name = device.name().unwrap_or_else(|_| "GPU".into());
                self.util = device.utilization_rates().ok().map(|u| u.gpu);
                if let Ok(m) = device.memory_info() {
                    self.mem_used = m.used;
                    self.mem_total = m.total;
                }
                self.temp_c = device.temperature(TemperatureSensor::Gpu).ok();
                self.power_w = device.power_usage().ok().map(|mw| mw as f32 / 1000.0);
                self.history.push(self.util.unwrap_or(0) as f64);
                self.available = true;
                return;
            }
        }

        // Fallback: a GPU-labelled sensor (temperature only, no util/VRAM).
        if let Some(c) = h.components.iter().find(|c| {
            let l = c.label().to_lowercase();
            l.contains("gpu") || l.contains("radeon") || l.contains("amd")
        }) {
            self.name = c.label().to_string();
            self.temp_c = Some(c.temperature() as u32);
            self.util = None;
            self.mem_total = 0;
            self.available = true;
            return;
        }

        self.available = false;
    }

    fn ui(&mut self, ui: &mut egui::Ui, size: WidgetSize) {
        if !self.available {
            ui.weak("n/a");
            ui.weak("No supported GPU");
            return;
        }

        match self.util {
            Some(u) => ui.heading(format!("{u}%")),
            None => ui.heading("—"),
        };
        if self.util.is_some() {
            let height = if size == WidgetSize::Small { 36.0 } else { 48.0 };
            charts::sparkline(ui, "gpu_widget_spark", &self.history, height, true);
        }
        if self.mem_total > 0 {
            charts::usage_bar(ui, "VRAM", self.mem_used, self.mem_total);
        }
        if size == WidgetSize::Medium {
            if let Some(t) = self.temp_c {
                ui.weak(format!("{t} °C"));
            }
            if let Some(w) = self.power_w {
                ui.weak(format!("{w:.0} W"));
            }
        }
    }

    fn linked_panel(&self) -> Option<&'static str> {
        Some("GPU")
    }

    fn available(&self) -> bool {
        self.available
    }
}
