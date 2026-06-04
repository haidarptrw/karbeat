use std::thread;
use std::time::Duration;

pub use karbeat_core::api::monitor_api::*;

use crate::frb_generated::StreamSink;

pub struct PerformanceMetricsDTO {
    pub os_cpu_usage: f32,
    pub ram_usage_mb: f32,
    pub total_ram_mb: f32,
    pub dsp_headroom: f32,
}

impl From<PerformanceMetrics> for PerformanceMetricsDTO {
    fn from(value: PerformanceMetrics) -> Self {
        Self {
            os_cpu_usage: value.os_cpu_usage,
            ram_usage_mb: value.ram_usage_mb,
            total_ram_mb: value.total_ram_mb,
            dsp_headroom: value.dsp_headroom,
        }
    }
}
#[flutter_rust_bridge::frb(sync)]
pub fn start_performance_monitor(sink: StreamSink<PerformanceMetricsDTO>) {
    thread::spawn(move || {
        let mut sys = init_sys();

        // Sleep briefly to let the system module gather its first delta
        thread::sleep(Duration::from_millis(200));

        loop {
            let metrics = fetch_metrics(&mut sys);

            let metrics_dto = metrics.into();
            if sink.add(metrics_dto).is_err() {
                log::info!("Performance monitor stream closed.");
                break;
            }

            thread::sleep(Duration::from_millis(33));
        }
    });
}
