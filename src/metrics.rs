use std::time::Instant;
use sysinfo::{Components, Disks, Networks, System};

/// Immutable, freshly-refreshed views of the OS handles, handed to each panel
/// on every tick. `elapsed` is the wall-clock time since the previous refresh,
/// used to turn cumulative counters (e.g. network bytes) into per-second rates.
pub struct SysHandles<'a> {
    pub sys: &'a System,
    pub disks: &'a Disks,
    pub networks: &'a Networks,
    pub components: &'a Components,
    pub elapsed: f64,
}

/// Owns the sysinfo handles and refreshes them in one place. Panels never
/// refresh the OS themselves — they only read from a `SysHandles`.
pub struct Source {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    last: Instant,
}

impl Source {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self {
            sys,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            last: Instant::now(),
        }
    }

    pub fn refresh(&mut self) -> SysHandles<'_> {
        self.sys.refresh_all();
        self.disks.refresh();
        self.networks.refresh();
        self.components.refresh();

        let elapsed = self.last.elapsed().as_secs_f64().max(0.001);
        self.last = Instant::now();

        SysHandles {
            sys: &self.sys,
            disks: &self.disks,
            networks: &self.networks,
            components: &self.components,
            elapsed,
        }
    }
}
