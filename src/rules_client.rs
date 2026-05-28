use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RulesResponse {
    pub probe_uuid: String,
    pub ruleset_version: i64,
    pub rules_count: usize,
    pub rules: Vec<RemoteRule>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteRule {
    pub uuid: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub rule_type: String,
    pub version: i64,
    pub content: serde_json::Value,
}

pub fn sync_rules(
    backend_url: &str,
    probe_uuid: &str,
    cache_file: &str,
) -> Result<RulesResponse> {
    let url = format!(
        "{}/probes/{}/rules",
        backend_url.trim_end_matches('/'),
        probe_uuid
    );

    println!("🔄 Scarico regole da backend: {}", url);

    let client = Client::new();

    let response = client
        .get(&url)
        .send()
        .context("Errore durante la chiamata al backend rules")?
        .error_for_status()
        .context("Il backend rules ha risposto con errore HTTP")?
        .json::<RulesResponse>()
        .context("Errore parsing JSON rules")?;

    save_rules_cache(cache_file, &response)?;

    println!(
        "✅ Regole sincronizzate: {} rule(s), version {}",
        response.rules_count,
        response.ruleset_version
    );

    Ok(response)
}

pub fn save_rules_cache(
    cache_file: &str,
    rules: &RulesResponse,
) -> Result<()> {
    if let Some(parent) = Path::new(cache_file).parent() {
        fs::create_dir_all(parent)
            .context("Errore creazione directory cache rules")?;
    }

    let json = serde_json::to_string_pretty(rules)
        .context("Errore serializzazione rules cache")?;

    fs::write(cache_file, json)
        .context("Errore scrittura rules_cache.json")?;

    Ok(())
}

pub fn load_rules_cache(cache_file: &str) -> Result<RulesResponse> {
    let content = fs::read_to_string(cache_file)
        .context("Errore lettura rules_cache.json")?;

    let rules = serde_json::from_str::<RulesResponse>(&content)
        .context("Errore parsing rules_cache.json")?;

    Ok(rules)
}

pub fn load_aggregation_rules(cache_file: &str) -> Result<Vec<RemoteRule>> {
    let cache = load_rules_cache(cache_file)?;

    let rules = cache
        .rules
        .into_iter()
        .filter(|rule| rule.rule_type == "aggregation")
        .collect();

    Ok(rules)
}