use std::time::Instant;
use sysinfo::{Components, Disks, Networks, System};

const HISTORY_LEN: usize = 60;

pub struct DiskInfo {
    pub name: String,
    pub used: u64,
    pub total: u64,
}

pub struct Metrics {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    last_net: Instant,

    pub cpu_history: Vec<f64>, // rolling window of global CPU %
    pub per_core: Vec<f32>,    // current per-core usage %
    pub ram_used: u64,
    pub ram_total: u64,
    pub disks_info: Vec<DiskInfo>,
    pub net_down: f64, // bytes/sec, summed across interfaces
    pub net_up: f64,
    pub net_down_history: Vec<f64>,
    pub net_up_history: Vec<f64>,
    pub temps: Vec<(String, f32)>, // (label, °C)
}

impl Metrics {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self {
            sys,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            last_net: Instant::now(),
            cpu_history: vec![0.0; HISTORY_LEN],
            per_core: Vec::new(),
            ram_used: 0,
            ram_total: 0,
            disks_info: Vec::new(),
            net_down: 0.0,
            net_up: 0.0,
            net_down_history: vec![0.0; HISTORY_LEN],
            net_up_history: vec![0.0; HISTORY_LEN],
            temps: Vec::new(),
        }
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_all();

        // CPU
        let cpu = self.sys.global_cpu_info().cpu_usage() as f64;
        push(&mut self.cpu_history, cpu);
        self.per_core = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();

        // Memory
        self.ram_used = self.sys.used_memory();
        self.ram_total = self.sys.total_memory();

        // Disks
        self.disks.refresh();
        self.disks_info = self
            .disks
            .iter()
            .map(|d| DiskInfo {
                name: d.mount_point().to_string_lossy().into_owned(),
                total: d.total_space(),
                used: d.total_space().saturating_sub(d.available_space()),
            })
            .collect();

        // Network — convert per-refresh byte counts into a per-second rate
        self.networks.refresh();
        let elapsed = self.last_net.elapsed().as_secs_f64().max(0.001);
        self.last_net = Instant::now();
        let (mut down, mut up) = (0u64, 0u64);
        for (_name, data) in self.networks.iter() {
            down += data.received();
            up += data.transmitted();
        }
        self.net_down = down as f64 / elapsed;
        self.net_up = up as f64 / elapsed;
        push(&mut self.net_down_history, self.net_down);
        push(&mut self.net_up_history, self.net_up);

        // Temperatures (often empty on Windows without elevated access)
        self.components.refresh();
        self.temps = self
            .components
            .iter()
            .map(|c| (c.label().to_string(), c.temperature()))
            .collect();
    }
}

fn push(buf: &mut Vec<f64>, value: f64) {
    buf.push(value);
    if buf.len() > HISTORY_LEN {
        buf.remove(0);
    }
}
