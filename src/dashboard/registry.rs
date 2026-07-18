//! Widget kind registry — the widget-world analog of `default_panels()`.
//! Deserialization looks up factories by `id` here and skips unknown ids
//! (forward compatibility with layouts saved by newer builds).

use crate::widgets::{CpuWidget, MemoryWidget, NetworkWidget, SystemWidget, Widget};

/// Construct a fresh widget instance for a registry `id`, or `None` if the id
/// is unknown to this build.
pub fn make(id: &str) -> Option<Box<dyn Widget>> {
    Some(match id {
        "cpu" => Box::new(CpuWidget::default()),
        "memory" => Box::new(MemoryWidget::default()),
        "network" => Box::new(NetworkWidget::default()),
        "system" => Box::new(SystemWidget::default()),
        _ => return None,
    })
}
