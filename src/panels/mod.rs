use eframe::egui;
use crate::metrics::SysHandles;

mod cpu;
mod memory;
mod explorer;
mod gpu;
mod mft_scan;
mod network;
mod processes;
mod system;
mod temps;

pub use cpu::CpuPanel;
pub use memory::MemoryPanel;
pub use explorer::ExplorerPanel;
pub use gpu::GpuPanel;
pub use network::NetworkPanel;
pub use processes::ProcessesPanel;
pub use system::SystemPanel;
pub use temps::TempsPanel;

/// A single self-contained section of the monitor. It keeps its own derived
/// state (histories, sorted lists, ...), pulls what it needs from the shared
/// `SysHandles` on refresh, and draws itself on `ui`.
pub trait Panel {
    /// Heading shown above the panel.
    fn name(&self) -> &str;
    /// Extract this panel's data from the freshly refreshed OS handles.
    fn refresh(&mut self, h: &SysHandles);
    /// Render the panel.
    fn ui(&mut self, ui: &mut egui::Ui);
}

/// The default set of panels, in display order.
pub fn default_panels() -> Vec<Box<dyn Panel>> {
    vec![
        Box::new(SystemPanel::default()),
        Box::new(CpuPanel::default()),
        Box::new(MemoryPanel::default()),
        Box::new(NetworkPanel::default()),
        Box::new(ProcessesPanel::default()),
        Box::new(TempsPanel::default()),
        Box::new(GpuPanel::default()),
        Box::new(ExplorerPanel::default()),
    ]
}
