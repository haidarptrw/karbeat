pub use sysinfo::{CpuRefreshKind, MemoryRefreshKind, System};

use crate::audio::engine::get_current_dsp_load;
#[derive(Clone, Debug)]
pub struct PerformanceMetrics {
    pub os_cpu_usage: f32, // 0.0 to 100.0
    pub ram_usage_mb: f32,
    pub total_ram_mb: f32,
    pub dsp_headroom: f32, // The "FL Studio" meter (0.0 to 100.0)
}

pub fn init_sys() -> System {
    System::new_with_specifics(
        sysinfo::RefreshKind::everything()
            .with_memory(MemoryRefreshKind::everything().without_swap()),
    )
}

pub fn fetch_metrics(sys: &mut System) -> PerformanceMetrics {
    sys.refresh_cpu_specifics(CpuRefreshKind::everything());
    sys.refresh_memory_specifics(MemoryRefreshKind::everything().without_swap());

    PerformanceMetrics {
        os_cpu_usage: sys.global_cpu_usage(),
        ram_usage_mb: (sys.used_memory() as f32) / 1_048_576.0,
        total_ram_mb: (sys.total_memory() as f32) / 1_048_576.0,
        dsp_headroom: get_current_dsp_load(),
    }
}
