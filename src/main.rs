mod config;
mod model;
mod normalizer;
mod reader;
mod dispatcher;
mod host;
mod probe;

use anyhow::Result;
use config::Config;
use host::get_host_info;
use normalizer::normalize_event;
use probe::get_or_create_probe_id;
use reader::EveReader;
use serde_json::Value;

fn main() -> Result<()> {

    println!("========== SONO IL MAIN REALE ==========");
    dotenvy::dotenv().ok();

    let cfg = Config::from_env();

    let mut reader = EveReader::new(&cfg.eve_path)?;
    let host_info = get_host_info();
    let probe_id = get_or_create_probe_id()?;

    println!("🚀 Probe avviata");
    println!("📄 Lettura da: {}", cfg.eve_path);
    println!("🆔 Probe ID: {}", probe_id);
    println!("📡 Sensor: {}", cfg.sensor_name);
    println!("🌐 Interface: {}", cfg.interface);
    println!("💻 Hostname: {}", host_info.hostname);
    println!("🌍 IP: {}", host_info.ip);
    println!("🖥️ OS: {}", host_info.os);

    if cfg.nmea_enabled {
        println!(
            "🛰️ NMEA abilitato su {}:{}",
            cfg.nmea_multicast_ip, cfg.nmea_port
        );
    } else {
        println!("🛰️ NMEA disabilitato");
    }

    if cfg.backend_enabled {
        println!("📤 Backend abilitato: {}", cfg.backend_url);
    } else {
        println!("📤 Backend disabilitato");
    }

    loop {
        let raw: Value = reader.next_json()?;

        if let Some(event) = normalize_event(
            &raw,
            &probe_id,
            cfg.sensor_name.as_str(),
            &host_info,
        ) {
            println!("{}", serde_json::to_string_pretty(&event)?);
            dispatcher::dispatch(&event)?;
        }
    }
}