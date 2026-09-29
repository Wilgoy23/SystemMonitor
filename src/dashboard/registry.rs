//! Widget kind registry — the widget-world analog of `default_panels()`.
//! The gallery iterates `catalog()`; deserialization and the gallery's Add
//! button look up factories by `id` and skip unknown ids (forward
//! compatibility with layouts saved by newer builds).

use crate::widgets::{
    CpuWidget, DiskWidget, GpuWidget, MemoryWidget, NetworkWidget, ProcessesWidget, SystemWidget,
    TempsWidget, Widget,
};

/// Static metadata + factory for one widget kind.
pub struct WidgetKindInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub factory: fn() -> Box<dyn Widget>,
    /// Whether more than one instance of this kind is meaningful (e.g. one Disk
    /// per mount). Single-instance kinds show as "Added" in the gallery.
    pub multi_instance: bool,
}

/// Every registered widget kind, in gallery display order.
pub fn catalog() -> &'static [WidgetKindInfo] {
    &[
        WidgetKindInfo {
            id: "cpu",
            name: "CPU",
            description: "Processor usage with a live sparkline; large size adds per-core bars.",
            factory: || Box::new(CpuWidget::default()),
            multi_instance: false,
        },
        WidgetKindInfo {
            id: "memory",
            name: "Memory",
            description: "RAM usage; medium adds swap and a usage sparkline.",
            factory: || Box::new(MemoryWidget::default()),
            multi_instance: false,
        },
        WidgetKindInfo {
            id: "gpu",
            name: "GPU",
            description: "GPU utilization and VRAM; medium adds temperature and power.",
            factory: || Box::new(GpuWidget::default()),
            multi_instance: false,
        },
        WidgetKindInfo {
            id: "network",
            name: "Network",
            description: "Up/down throughput; medium adds a sparkline.",
            factory: || Box::new(NetworkWidget::default()),
            multi_instance: true,
        },
        WidgetKindInfo {
            id: "disk",
            name: "Disk",
            description: "Disk-space usage for a mount; medium adds free space and filesystem.",
            factory: || Box::new(DiskWidget::default()),
            multi_instance: true,
        },
        WidgetKindInfo {
            id: "temps",
            name: "Temperatures",
            description: "Hottest sensor; medium lists the top readings.",
            factory: || Box::new(TempsWidget::default()),
            multi_instance: false,
        },
        WidgetKindInfo {
            id: "processes",
            name: "Top processes",
            description: "Busiest processes by CPU (top 3, or top 8 at large size).",
            factory: || Box::new(ProcessesWidget::default()),
            multi_instance: false,
        },
        WidgetKindInfo {
            id: "system",
            name: "Uptime",
            description: "System uptime; medium adds host, OS, and kernel.",
            factory: || Box::new(SystemWidget::default()),
            multi_instance: false,
        },
    ]
}

/// Construct a fresh widget instance for a registry `id`, or `None` if the id
/// is unknown to this build.
pub fn make(id: &str) -> Option<Box<dyn Widget>> {
    catalog().iter().find(|k| k.id == id).map(|k| (k.factory)())
}
