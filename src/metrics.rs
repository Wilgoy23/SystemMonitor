use sysinfo::System;

pub struct Metrics {
    pub sys: System,
    pub cpu_history: Vec<f64>,   // rolling window
    pub ram_used: u64,
    pub ram_total: u64,
}

impl Metrics {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self { sys, cpu_history: vec![0.0; 60], ram_used: 0, ram_total: 0 }
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_all();
        let cpu = self.sys.global_cpu_info().cpu_usage() as f64;
        self.cpu_history.push(cpu);
        if self.cpu_history.len() > 60 { self.cpu_history.remove(0); }
        self.ram_used = self.sys.used_memory();
        self.ram_total = self.sys.total_memory();
    }
}