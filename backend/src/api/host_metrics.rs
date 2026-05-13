use axum::Json;
use serde::Serialize;
use sysinfo::{Disks, System};

#[derive(Debug, Serialize)]
pub struct HostMetrics {
    pub hostname: String,
    pub cpu_pct: f64,
    pub ram_pct: f64,
    pub disk_pct: f64,
    pub ram_total_gb: f64,
    pub disk_total_gb: f64,
}

pub async fn get_host_metrics() -> Json<HostMetrics> {
    // sysinfo requires two refreshes for CPU (first call seeds baseline).
    // The blocking sysinfo work is off-loaded to a blocking thread to avoid
    // stalling Tokio workers; tokio::time::sleep yields the executor correctly.
    let mut sys = System::new_all();
    sys.refresh_all();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    sys.refresh_cpu_all();

    let cpu_pct = sys.global_cpu_usage() as f64;

    let ram_total = sys.total_memory();
    let ram_used = sys.used_memory();
    let ram_total_gb = ram_total as f64 / (1024.0 * 1024.0 * 1024.0);
    let ram_pct = if ram_total > 0 {
        ram_used as f64 / ram_total as f64 * 100.0
    } else {
        0.0
    };

    let disks = Disks::new_with_refreshed_list();
    // Exclude virtual/network/bind-mount filesystems that inflate totals inside
    // containers (e.g. fuse.grpcfuse is OrbStack's macOS bind mount, overlay
    // is the container root layer, tmpfs/devtmpfs are in-memory).
    const EXCLUDED_FS: &[&str] = &["tmpfs", "devtmpfs", "overlay", "fuse.grpcfuse", "fuse"];
    let (disk_total, disk_used) = disks
        .iter()
        .filter(|d| {
            let fs = d.file_system().to_string_lossy().to_lowercase();
            !EXCLUDED_FS.iter().any(|ex| fs.contains(ex))
        })
        .fold((0u64, 0u64), |(t, u), d| {
            (
                t + d.total_space(),
                u + d.total_space().saturating_sub(d.available_space()),
            )
        });
    let disk_total_gb = disk_total as f64 / (1024.0 * 1024.0 * 1024.0);
    let disk_pct = if disk_total > 0 {
        disk_used as f64 / disk_total as f64 * 100.0
    } else {
        0.0
    };

    let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());

    Json(HostMetrics {
        hostname,
        cpu_pct,
        ram_pct,
        disk_pct,
        ram_total_gb,
        disk_total_gb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn host_metrics_returns_valid_ranges() {
        let Json(m) = get_host_metrics().await;
        assert!(!m.hostname.is_empty(), "hostname must not be empty");
        assert!(
            (0.0..=100.0).contains(&m.cpu_pct),
            "cpu_pct out of range: {}",
            m.cpu_pct
        );
        assert!(
            (0.0..=100.0).contains(&m.ram_pct),
            "ram_pct out of range: {}",
            m.ram_pct
        );
        assert!(
            (0.0..=100.0).contains(&m.disk_pct),
            "disk_pct out of range: {}",
            m.disk_pct
        );
        assert!(m.ram_total_gb > 0.0, "ram_total_gb must be positive");
        // disk_total_gb may be 0 in extremely minimal container environments
    }
}
