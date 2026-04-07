use crate::model::HostInfo;
use sysinfo::System;

pub fn get_host_info() -> HostInfo {
    let hostname = hostname::get()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = System::os_version().unwrap_or_default();

    let os = if os_version.is_empty() {
        os_name
    } else {
        format!("{} {}", os_name, os_version)
    };

    let ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "0.0.0.0".to_string());

    HostInfo { hostname, ip, os }
}