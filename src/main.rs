mod config;
mod model;
mod normalizer;
mod reader;
mod dispatcher;
mod host;
mod probe;
mod queue;
mod rules_client;

use anyhow::Result;
use config::Config;
use host::get_host_info;
use normalizer::normalize_event;
use probe::get_or_create_probe_id;
use reader::EveReader;
use serde_json::Value;
use std::env;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cfg = Config::from_env();

    let mut reader = EveReader::new(&cfg.eve_path)?;
    let host_info = get_host_info();
    let probe_id = get_or_create_probe_id()?;

    println!("🚀 Nautilus Sonar avviata");
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
        println!("📤 Backend HTTP abilitato: {}", cfg.backend_url);
    } else {
        println!("📤 Backend HTTP disabilitato");
    }

    let rules_sync_enabled = env_bool("RULES_SYNC_ENABLED", false);

    let rules_backend_url = env::var("RULES_BACKEND_URL")
        .unwrap_or_else(|_| cfg.backend_url.clone());

    let rules_cache_file = env::var("RULES_CACHE_FILE")
        .unwrap_or_else(|_| "/app/output/rules_cache.json".to_string());

    let suricata_rules_file = env::var("SURICATA_RULES_FILE")
        .unwrap_or_else(|_| "/var/lib/suricata/rules/nautilus.rules".to_string());

    let rules_sync_interval_seconds = env_u64("RULES_SYNC_INTERVAL_SECONDS", 60);

    if rules_sync_enabled {
        sync_rules_or_use_cache(
            &rules_backend_url,
            &probe_id,
            &rules_cache_file,
            &suricata_rules_file,
        );
    } else {
        println!("📜 Rules sync disabilitato");
    }

    let mut last_rules_sync = Instant::now();

    loop {
        if rules_sync_enabled
            && last_rules_sync.elapsed() >= Duration::from_secs(rules_sync_interval_seconds)
        {
            println!("🔄 Refresh periodico regole...");

            sync_rules_or_use_cache(
                &rules_backend_url,
                &probe_id,
                &rules_cache_file,
                &suricata_rules_file,
            );

            last_rules_sync = Instant::now();
        }

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

fn sync_rules_or_use_cache(
    rules_backend_url: &str,
    probe_id: &str,
    rules_cache_file: &str,
    suricata_rules_file: &str,
) {
    match rules_client::sync_rules(
        rules_backend_url,
        probe_id,
        rules_cache_file,
    ) {
        Ok(response) => {
            println!(
                "✅ Rules cache aggiornata: {} rule(s), version {}",
                response.rules_count,
                response.ruleset_version
            );

            update_suricata_rules_from_cache(
                rules_cache_file,
                suricata_rules_file,
            );
        }

        Err(e) => {
            eprintln!("⚠️ Backend rules non raggiungibile: {:?}", e);

            match rules_client::load_rules_cache(rules_cache_file) {
                Ok(cache) => {
                    println!(
                        "✅ Uso rules cache locale: {} rule(s), version {}",
                        cache.rules_count,
                        cache.ruleset_version
                    );

                    update_suricata_rules_from_cache(
                        rules_cache_file,
                        suricata_rules_file,
                    );
                }

                Err(cache_err) => {
                    eprintln!(
                        "❌ Nessuna rules cache disponibile: {:?}",
                        cache_err
                    );
                }
            }
        }
    }
}

fn update_suricata_rules_from_cache(
    rules_cache_file: &str,
    suricata_rules_file: &str,
) {
    match rules_client::write_suricata_rules_file(
        rules_cache_file,
        suricata_rules_file,
    ) {
        Ok(updated) => {
            if updated {
                println!("✅ Regole Suricata aggiornate");
                println!("ℹ️ Suricata deve ricaricare le regole");
            } else {
                println!("ℹ️ Regole Suricata invariate");
            }
        }

        Err(e) => {
            eprintln!(
                "⚠️ Errore aggiornamento regole Suricata: {:?}",
                e
            );
        }
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}