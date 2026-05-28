mod config;
mod model;
mod normalizer;
mod reader;
mod dispatcher;
mod host;
mod probe;
mod queue;
mod rules_client;
mod aggregation_engine;

use aggregation_engine::AggregationEngine;
use anyhow::Result;
use config::Config;
use host::get_host_info;
use normalizer::normalize_event;
use probe::get_or_create_probe_id;
use reader::EveReader;
use serde_json::Value;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
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

    let rules_sync_enabled = std::env::var("RULES_SYNC_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        == "true";

    let rules_backend_url = std::env::var("RULES_BACKEND_URL")
        .unwrap_or_else(|_| cfg.backend_url.clone());

    let rules_cache_file = std::env::var("RULES_CACHE_FILE")
        .unwrap_or_else(|_| "/app/output/rules_cache.json".to_string());

    let rules_sync_interval_seconds: u64 = std::env::var("RULES_SYNC_INTERVAL_SECONDS")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .unwrap_or(60);

    if rules_sync_enabled {
        sync_rules_or_use_cache(
            &rules_backend_url,
            &probe_id,
            &rules_cache_file,
        );
    } else {
        println!("📜 Rules sync disabilitato");
    }

    let aggregation_rules = load_aggregation_rules_from_cache(&rules_cache_file);

    let mut aggregation_engine = AggregationEngine::new(aggregation_rules);

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
            );

            let updated_rules = load_aggregation_rules_from_cache(&rules_cache_file);

            aggregation_engine.reload_rules(updated_rules);

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

            let event_value = serde_json::to_value(&event)?;

            let incidents = aggregation_engine.process_event(&event_value)?;

            for incident in incidents {
                println!(
                    "🚨 INCIDENT DETECTED:\n{}",
                    serde_json::to_string_pretty(&incident)?
                );
            }
        }
    }
}

fn sync_rules_or_use_cache(
    rules_backend_url: &str,
    probe_id: &str,
    rules_cache_file: &str,
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

fn load_aggregation_rules_from_cache(
    rules_cache_file: &str,
) -> Vec<rules_client::RemoteRule> {
    match rules_client::load_aggregation_rules(rules_cache_file) {
        Ok(rules) => {
            println!("📜 Regole aggregation caricate: {}", rules.len());

            for rule in &rules {
                println!(
                    "📌 Rule: {} | type={} | version={}",
                    rule.name,
                    rule.rule_type,
                    rule.version
                );
            }

            rules
        }

        Err(e) => {
            eprintln!("⚠️ Impossibile caricare regole aggregation: {:?}", e);
            Vec::new()
        }
    }
}