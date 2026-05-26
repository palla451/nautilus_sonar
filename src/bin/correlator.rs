use anyhow::{bail, Result};
use chrono::Utc;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
struct CorrelationRule {
    id: String,
    name: String,
    description: Option<String>,
    enabled: bool,
    source_index: String,
    target_index: String,
    filter: HashMap<String, String>,
    group_by: Vec<String>,
    threshold: Threshold,
    severity: String,
    incident_type: String,
    run_every_seconds: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct Threshold {
    count: u64,
    window_seconds: i64,
}

#[derive(Debug, Serialize)]
struct NautilusIncident {
    timestamp: String,
    incident_id: String,
    rule_id: String,
    rule_name: String,
    incident_type: String,
    severity: String,
    description: String,
    source_index: String,
    evidence: IncidentEvidence,
}

#[derive(Debug, Serialize)]
struct IncidentEvidence {
    count: u64,
    window_seconds: i64,
    group_by: Vec<String>,
    values: HashMap<String, String>,
}

struct OpenSearchIncidentClient {
    client: Client,
    url: String,
    username: String,
    password: String,
}

impl OpenSearchIncidentClient {
    fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()?;

        let url = env::var("OPENSEARCH_URL")
            .unwrap_or_else(|_| "http://opensearch:9200".to_string())
            .trim_end_matches('/')
            .to_string();

        let username = env::var("OPENSEARCH_USERNAME")
            .unwrap_or_else(|_| "admin".to_string());

        let password = env::var("OPENSEARCH_PASSWORD")
            .unwrap_or_else(|_| "admin".to_string());

        Ok(Self {
            client,
            url,
            username,
            password,
        })
    }

    fn search(&self, index: &str, query: Value) -> Result<Value> {
        let response = self
            .client
            .post(format!("{}/{}/_search", self.url, index))
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "application/json")
            .json(&query)
            .send()?;

        let status = response.status();
        let body = response.text()?;

        if !status.is_success() {
            bail!("OpenSearch search error {}: {}", status, body);
        }

        Ok(serde_json::from_str(&body)?)
    }

    fn index_incident(
        &self,
        target_index: &str,
        incident_id: &str,
        incident: &NautilusIncident,
    ) -> Result<()> {
        let response = self
            .client
            .put(format!(
                "{}/{}/_doc/{}",
                self.url, target_index, incident_id
            ))
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "application/json")
            .json(incident)
            .send()?;

        let status = response.status();
        let body = response.text()?;

        if !status.is_success() {
            bail!("OpenSearch incident index error {}: {}", status, body);
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    println!("🚀 Nautilus correlator avviato");

    let rules_dir = env::var("CORRELATOR_RULES_DIR")
        .unwrap_or_else(|_| "/app/rules".to_string());

    println!("📁 Rules dir: {}", rules_dir);

    let client = OpenSearchIncidentClient::new()?;

    loop {
        let rules = load_rules(&rules_dir)?;

        if rules.is_empty() {
            println!("⚠️ Nessuna regola trovata in {}", rules_dir);
        }

        let mut min_sleep = 60;

        for rule in rules {
            if !rule.enabled {
                println!("⏭️ Regola disabilitata: {}", rule.id);
                continue;
            }

            let run_every = rule.run_every_seconds.unwrap_or(60);
            min_sleep = min_sleep.min(run_every);

            if let Err(err) = execute_rule(&client, &rule) {
                eprintln!("❌ Errore regola {}: {}", rule.id, err);
            }
        }

        println!("😴 Correlator sleep: {}s", min_sleep);
        thread::sleep(Duration::from_secs(min_sleep));
    }
}

fn load_rules(rules_dir: &str) -> Result<Vec<CorrelationRule>> {
    let mut rules = Vec::new();

    let entries = fs::read_dir(rules_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };

        if extension != "yaml" && extension != "yml" {
            continue;
        }

        let content = fs::read_to_string(&path)?;
        let rule: CorrelationRule = serde_yaml::from_str(&content)?;

        println!("📜 Regola caricata: {} ({})", rule.id, path.display());
        rules.push(rule);
    }

    Ok(rules)
}

fn execute_rule(client: &OpenSearchIncidentClient, rule: &CorrelationRule) -> Result<()> {
    if rule.group_by.is_empty() {
        bail!("rule {} has empty group_by", rule.id);
    }

    if rule.group_by.len() > 2 {
        bail!("rule {} supports max 2 group_by fields for now", rule.id);
    }

    println!("🔎 Eseguo regola: {}", rule.id);

    let query = build_aggregation_query(rule);

    let response = client.search(&rule.source_index, query)?;

    let matches = extract_matches(rule, &response)?;

    if matches.is_empty() {
        println!("✅ Nessun match per regola {}", rule.id);
        return Ok(());
    }

    println!(
        "🚨 Regola {} ha generato {} incident candidate",
        rule.id,
        matches.len()
    );

    for rule_match in matches {
        let incident_id = build_incident_id(rule, &rule_match.values);

        let incident = NautilusIncident {
            timestamp: Utc::now().to_rfc3339(),
            incident_id: incident_id.clone(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            incident_type: rule.incident_type.clone(),
            severity: rule.severity.clone(),
            description: rule
                .description
                .clone()
                .unwrap_or_else(|| rule.name.clone()),
            source_index: rule.source_index.clone(),
            evidence: IncidentEvidence {
                count: rule_match.count,
                window_seconds: rule.threshold.window_seconds,
                group_by: rule.group_by.clone(),
                values: rule_match.values,
            },
        };

        client.index_incident(&rule.target_index, &incident_id, &incident)?;

        println!(
            "🚨 Incidente indicizzato: {} severity={} count={}",
            incident_id, rule.severity, incident.evidence.count
        );
    }

    Ok(())
}

#[derive(Debug)]
struct RuleMatch {
    count: u64,
    values: HashMap<String, String>,
}

fn build_aggregation_query(rule: &CorrelationRule) -> Value {
    let mut filters = Vec::new();

    for (field, value) in &rule.filter {
        filters.push(json!({
            "term": {
                field: value
            }
        }));
    }

    filters.push(json!({
        "range": {
            "timestamp": {
                "gte": format!("now-{}s", rule.threshold.window_seconds),
                "lte": "now"
            }
        }
    }));

    let first_field = &rule.group_by[0];

    let aggs = if rule.group_by.len() == 1 {
        json!({
            "g1": {
                "terms": {
                    "field": first_field,
                    "size": 100,
                    "min_doc_count": rule.threshold.count
                }
            }
        })
    } else {
        let second_field = &rule.group_by[1];

        json!({
            "g1": {
                "terms": {
                    "field": first_field,
                    "size": 100,
                    "min_doc_count": 1
                },
                "aggs": {
                    "g2": {
                        "terms": {
                            "field": second_field,
                            "size": 100,
                            "min_doc_count": rule.threshold.count
                        }
                    }
                }
            }
        })
    };

    json!({
        "size": 0,
        "query": {
            "bool": {
                "filter": filters
            }
        },
        "aggs": aggs
    })
}

fn extract_matches(rule: &CorrelationRule, response: &Value) -> Result<Vec<RuleMatch>> {
    let mut matches = Vec::new();

    let Some(g1_buckets) = response
        .get("aggregations")
        .and_then(|a| a.get("g1"))
        .and_then(|g| g.get("buckets"))
        .and_then(|b| b.as_array())
    else {
        return Ok(matches);
    };

    if rule.group_by.len() == 1 {
        for bucket in g1_buckets {
            let count = bucket
                .get("doc_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            if count < rule.threshold.count {
                continue;
            }

            let key = bucket_key_to_string(bucket.get("key"));

            let mut values = HashMap::new();
            values.insert(rule.group_by[0].clone(), key);

            matches.push(RuleMatch { count, values });
        }

        return Ok(matches);
    }

    for bucket1 in g1_buckets {
        let key1 = bucket_key_to_string(bucket1.get("key"));

        let Some(g2_buckets) = bucket1
            .get("g2")
            .and_then(|g| g.get("buckets"))
            .and_then(|b| b.as_array())
        else {
            continue;
        };

        for bucket2 in g2_buckets {
            let count = bucket2
                .get("doc_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            if count < rule.threshold.count {
                continue;
            }

            let key2 = bucket_key_to_string(bucket2.get("key"));

            let mut values = HashMap::new();
            values.insert(rule.group_by[0].clone(), key1.clone());
            values.insert(rule.group_by[1].clone(), key2);

            matches.push(RuleMatch { count, values });
        }
    }

    Ok(matches)
}

fn bucket_key_to_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
        None => "-".to_string(),
    }
}

fn build_incident_id(rule: &CorrelationRule, values: &HashMap<String, String>) -> String {
    let now = Utc::now().timestamp();
    let window = rule.threshold.window_seconds.max(1);
    let window_start = now - (now % window);

    let mut parts = vec![sanitize_id_part(&rule.id)];

    for field in &rule.group_by {
        let value = values.get(field).cloned().unwrap_or_else(|| "-".to_string());
        parts.push(sanitize_id_part(&value));
    }

    parts.push(window_start.to_string());

    parts.join("-")
}

fn sanitize_id_part(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}