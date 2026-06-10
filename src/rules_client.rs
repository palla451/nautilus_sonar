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

pub fn save_rules_cache(cache_file: &str, rules: &RulesResponse) -> Result<()> {
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

    Ok(cache
        .rules
        .into_iter()
        .filter(|rule| rule.rule_type == "aggregation")
        .collect())
}

pub fn load_correlation_rules(cache_file: &str) -> Result<Vec<RemoteRule>> {
    let cache = load_rules_cache(cache_file)?;

    Ok(cache
        .rules
        .into_iter()
        .filter(|rule| rule.rule_type == "correlation")
        .collect())
}

pub fn load_suricata_rules(cache_file: &str) -> Result<Vec<RemoteRule>> {
    let cache = load_rules_cache(cache_file)?;

    Ok(cache
        .rules
        .into_iter()
        .filter(|rule| rule.rule_type == "suricata")
        .collect())
}

pub fn build_suricata_rules_file(cache_file: &str) -> Result<String> {
    let rules = load_suricata_rules(cache_file)?;

    let mut output = String::new();

    output.push_str("# ==================================================\n");
    output.push_str("# Nautilus Sonar - Managed Suricata Rules\n");
    output.push_str("# DO NOT EDIT MANUALLY\n");
    output.push_str("# Generated from Laravel Rule Manager\n");
    output.push_str("# ==================================================\n\n");

    for rule in rules {
        let content = normalize_content(rule.content)?;

        if let Some(rule_text) = content.get("rule").and_then(|v| v.as_str()) {
            let clean_rule = normalize_suricata_rule(rule_text);

            if !clean_rule.is_empty() {
                output.push_str("# ");
                output.push_str(&rule.name);
                output.push('\n');
                output.push_str(&clean_rule);
                output.push_str("\n\n");
            }
        }
    }

    Ok(output)
}

pub fn write_suricata_rules_file(
    cache_file: &str,
    suricata_rules_file: &str,
) -> Result<bool> {
    let generated = build_suricata_rules_file(cache_file)?;

    let current = fs::read_to_string(suricata_rules_file).unwrap_or_default();

    if current == generated {
        println!("ℹ️ Regole Suricata invariate");
        return Ok(false);
    }

    if let Some(parent) = Path::new(suricata_rules_file).parent() {
        fs::create_dir_all(parent)
            .context("Errore creazione directory Suricata rules")?;
    }

    let tmp_file = format!("{}.tmp", suricata_rules_file);

    fs::write(&tmp_file, generated)
        .context("Errore scrittura file temporaneo Suricata rules")?;

    fs::rename(&tmp_file, suricata_rules_file)
        .context("Errore sostituzione file Suricata rules")?;

    println!("✅ File regole Suricata aggiornato: {}", suricata_rules_file);

    Ok(true)
}

fn normalize_content(content: serde_json::Value) -> Result<serde_json::Value> {
    if let Some(raw) = content.as_str() {
        let decoded = serde_json::from_str::<serde_json::Value>(raw)
            .context("Errore decodifica content JSON string")?;

        return Ok(decoded);
    }

    Ok(content)
}

fn normalize_suricata_rule(rule: &str) -> String {
    rule.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join(" ")
}