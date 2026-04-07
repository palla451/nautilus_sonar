mod model;
mod normalizer;
mod reader;
mod dispatcher;
mod host;
mod probe;

use anyhow::Result;
use host::get_host_info;
use normalizer::normalize_event;
use probe::get_or_create_probe_id;
use reader::EveReader;

fn main() -> Result<()> {
    let eve_path = "/var/log/suricata/eve.json";
    let sensor_name = "nautilus-sonar";

    let mut reader = EveReader::new(eve_path)?;
    let host_info = get_host_info();
    let probe_id = get_or_create_probe_id()?;

    println!("🚀 Probe avviata");
    println!("📄 Lettura da: {}", eve_path);
    println!("🆔 Probe ID: {}", probe_id);
    println!("📡 Sensor: {}", sensor_name);
    println!("💻 Hostname: {}", host_info.hostname);
    println!("🌐 IP: {}", host_info.ip);
    println!("🖥️ OS: {}", host_info.os);

    loop {
        let raw = reader.next_json()?;

        if let Some(event) = normalize_event(&raw, &probe_id, sensor_name, &host_info) {
            println!("{}", serde_json::to_string_pretty(&event)?);
            dispatcher::dispatch(&event)?;
        }
    }
}