use anyhow::{bail, Context, Result};
use uuid::Uuid;
use chrono::Utc;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RulesResponse {
    probe_uuid: String,
    ruleset_version: i64,
    rules_count: usize,
    rules: Vec<RemoteRule>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RemoteRule {
    uuid: String,
    name: String,
    description: Option<String>,

    #[serde(rename = "type")]
    rule_type: String,

    version: i64,
    content: Value,
}

#[derive(Debug, Clone)]
struct Threshold {
    count: u64,
    window_seconds: i64,
}

#[derive(Debug, Clone)]
struct AggregationRule {
    id: String,
    uuid: String,
    name: String,
    description: Option<String>,
    source_index: String,
    target_index: String,
    filter: HashMap<String, Value>,
    group_by: Vec<String>,
    threshold: Threshold,
    severity: String,
    incident_type: String,
    run_every_seconds: u64,
}

#[derive(Debug, Clone)]
struct CorrelationRule {
    id: String,
    uuid: String,
    name: String,
    description: Option<String>,
    source_index: String,
    target_index: String,
    conditions: Vec<HashMap<String, Value>>,
    window_seconds: i64,
    severity: String,
    incident_type: String,
    run_every_seconds: u64,
}

#[derive(Debug, Serialize)]
struct NautilusIncident {
    timestamp: String,

    incident_id: String,

    correlation_key: String,

    rule_id: String,
    rule_uuid: String,
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
            .put(format!("{}/{}/_doc/{}", self.url, target_index, incident_id))
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

    println!("🚀 Nautilus Detection Engine avviato");

    let client = OpenSearchIncidentClient::new()?;

    let backend_url = env::var("CORRELATOR_RULES_BACKEND_URL")
        .or_else(|_| env::var("RULES_BACKEND_URL"))
        .unwrap_or_else(|_| "http://host.docker.internal:8080/api".to_string());

    let probe_id_file = env::var("CORRELATOR_PROBE_ID_FILE")
        .unwrap_or_else(|_| "/app/output/probe_id".to_string());

    let cache_file = env::var("CORRELATOR_RULES_CACHE_FILE")
        .unwrap_or_else(|_| "/app/output/correlator_rules_cache.json".to_string());

    let default_sleep = env::var("CORRELATOR_RULES_SYNC_INTERVAL_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(10);

    let probe_uuid = read_probe_uuid(&probe_id_file)?;

    println!("🆔 Probe UUID: {}", probe_uuid);
    println!("🌐 Rules backend: {}", backend_url);
    println!("📄 Correlator cache: {}", cache_file);

    loop {
        let rules = match fetch_rules_from_laravel(&backend_url, &probe_uuid, &cache_file) {
            Ok(rules) => rules,
            Err(err) => {
                eprintln!("⚠️ Errore download regole Laravel: {:?}", err);
                println!("📦 Provo cache locale correlator...");
                load_rules_from_cache(&cache_file)?
            }
        };

        let aggregation_rules = build_aggregation_rules(rules.clone())?;
        let correlation_rules = build_correlation_rules(rules)?;

        if aggregation_rules.is_empty() {
            println!("⚠️ Nessuna regola aggregation trovata");
        }

        if correlation_rules.is_empty() {
            println!("⚠️ Nessuna regola correlation trovata");
        }

        let mut min_sleep = default_sleep;

        for rule in aggregation_rules {
            min_sleep = min_sleep.min(rule.run_every_seconds);

            if let Err(err) = execute_aggregation_rule(&client, &rule) {
                eprintln!("❌ Errore aggregation rule {}: {}", rule.id, err);
            }
        }

        for rule in correlation_rules {
            min_sleep = min_sleep.min(rule.run_every_seconds);

            if let Err(err) = execute_correlation_rule(&client, &rule) {
                eprintln!("❌ Errore correlation rule {}: {}", rule.id, err);
            }
        }

        println!("😴 Detection Engine sleep: {}s", min_sleep);
        thread::sleep(Duration::from_secs(min_sleep));
    }
}

fn read_probe_uuid(probe_id_file: &str) -> Result<String> {
    let value = fs::read_to_string(probe_id_file)
        .with_context(|| format!("Errore lettura probe_id da {}", probe_id_file))?;

    Ok(value.trim().to_string())
}

fn fetch_rules_from_laravel(
    backend_url: &str,
    probe_uuid: &str,
    cache_file: &str,
) -> Result<Vec<RemoteRule>> {
    let url = format!(
        "{}/probes/{}/rules",
        backend_url.trim_end_matches('/'),
        probe_uuid
    );

    println!("🔄 Scarico regole centralizzate da Laravel: {}", url);

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;

    let response = client
        .get(&url)
        .send()
        .context("Errore chiamata Laravel rules API")?
        .error_for_status()
        .context("Laravel rules API ha risposto con errore HTTP")?
        .json::<RulesResponse>()
        .context("Errore parsing JSON Laravel rules")?;

    save_rules_cache(cache_file, &response)?;

    println!(
        "✅ Regole ricevute da Laravel: {} rule(s), version {}",
        response.rules_count,
        response.ruleset_version
    );

    Ok(response.rules)
}

fn save_rules_cache(cache_file: &str, response: &RulesResponse) -> Result<()> {
    if let Some(parent) = Path::new(cache_file).parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(cache_file, serde_json::to_string_pretty(response)?)?;

    Ok(())
}

fn load_rules_from_cache(cache_file: &str) -> Result<Vec<RemoteRule>> {
    let content = fs::read_to_string(cache_file)
        .with_context(|| format!("Errore lettura cache {}", cache_file))?;

    let response = serde_json::from_str::<RulesResponse>(&content)
        .context("Errore parsing correlator rules cache")?;

    println!(
        "✅ Uso cache locale correlator: {} rule(s), version {}",
        response.rules_count,
        response.ruleset_version
    );

    Ok(response.rules)
}

fn build_aggregation_rules(remote_rules: Vec<RemoteRule>) -> Result<Vec<AggregationRule>> {
    let mut rules = Vec::new();

    for remote in remote_rules {
        if remote.rule_type != "aggregation" {
            continue;
        }

        let content = normalize_content(remote.content)?;

        let id = get_string(&content, "id").unwrap_or_else(|| remote.uuid.clone());

        let source_index = get_string(&content, "source_index")
            .unwrap_or_else(|| "nautilus-events".to_string());

        let target_index = get_string(&content, "target_index")
            .unwrap_or_else(|| "nautilus-incidents".to_string());

        let filter = content
            .get("filter")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<HashMap<String, Value>>()
            })
            .unwrap_or_default();

        let group_by = content
            .get("group_by")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        let threshold_count = content
            .pointer("/threshold/count")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);

        let window_seconds = content
            .pointer("/threshold/window_seconds")
            .and_then(|v| v.as_i64())
            .unwrap_or(60);

        let severity = get_string(&content, "severity")
            .unwrap_or_else(|| "low".to_string());

        let incident_type = get_string(&content, "incident_type")
            .unwrap_or_else(|| "aggregation_incident".to_string());

        let run_every_seconds = content
            .get("run_every_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(60);

        let rule = AggregationRule {
            id,
            uuid: remote.uuid,
            name: remote.name,
            description: remote.description,
            source_index,
            target_index,
            filter,
            group_by,
            threshold: Threshold {
                count: threshold_count,
                window_seconds,
            },
            severity,
            incident_type,
            run_every_seconds,
        };

        println!("📜 Aggregation rule caricata da Laravel: {}", rule.id);

        rules.push(rule);
    }

    Ok(rules)
}

fn build_correlation_rules(remote_rules: Vec<RemoteRule>) -> Result<Vec<CorrelationRule>> {
    let mut rules = Vec::new();

    for remote in remote_rules {
        if remote.rule_type != "correlation" {
            continue;
        }

        let content = normalize_content(remote.content)?;

        let id = get_string(&content, "id").unwrap_or_else(|| remote.uuid.clone());

        let source_index = get_string(&content, "source_index")
            .unwrap_or_else(|| "nautilus-events".to_string());

        let target_index = get_string(&content, "target_index")
            .unwrap_or_else(|| "nautilus-incidents".to_string());

        let conditions = content
            .get("conditions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.as_object().map(|obj| {
                            obj.iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect::<HashMap<String, Value>>()
                        })
                    })
                    .collect::<Vec<HashMap<String, Value>>>()
            })
            .unwrap_or_default();

        let window_seconds = content
            .get("window_seconds")
            .and_then(|v| v.as_i64())
            .unwrap_or(300);

        let severity = get_string(&content, "severity")
            .unwrap_or_else(|| "medium".to_string());

        let incident_type = get_string(&content, "incident_type")
            .unwrap_or_else(|| "correlation_incident".to_string());

        let run_every_seconds = content
            .get("run_every_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(60);

        let rule = CorrelationRule {
            id,
            uuid: remote.uuid,
            name: remote.name,
            description: remote.description,
            source_index,
            target_index,
            conditions,
            window_seconds,
            severity,
            incident_type,
            run_every_seconds,
        };

        println!("📜 Correlation rule caricata da Laravel: {}", rule.id);

        rules.push(rule);
    }

    Ok(rules)
}

fn normalize_content(content: Value) -> Result<Value> {
    if let Some(raw) = content.as_str() {
        return Ok(serde_json::from_str::<Value>(raw)
            .context("Errore decodifica content JSON string")?);
    }

    Ok(content)
}

fn get_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn execute_aggregation_rule(
    client: &OpenSearchIncidentClient,
    rule: &AggregationRule,
) -> Result<()> {
    if rule.group_by.is_empty() {
        bail!("rule {} has empty group_by", rule.id);
    }

    if rule.group_by.len() > 2 {
        bail!("rule {} supports max 2 group_by fields for now", rule.id);
    }

    println!("🔎 Eseguo aggregation rule: {}", rule.id);

    let query = build_aggregation_query(rule);

    let response = client.search(&rule.source_index, query)?;

    let matches = extract_aggregation_matches(rule, &response)?;

    if matches.is_empty() {
        println!("✅ Nessun match per aggregation rule {}", rule.id);
        return Ok(());
    }

    println!(
        "🚨 Aggregation rule {} ha generato {} incident candidate",
        rule.id,
        matches.len()
    );

    for rule_match in matches {
        let correlation_key = build_aggregation_incident_id(rule, &rule_match.values);
        let incident_id = stable_incident_uuid(&correlation_key);

        let incident = NautilusIncident {
            timestamp: Utc::now().to_rfc3339(),
            incident_id: incident_id.clone(),
            correlation_key: correlation_key.clone(),
            rule_id: rule.id.clone(),
            rule_uuid: rule.uuid.clone(),
            rule_name: rule.name.clone(),
            incident_type: rule.incident_type.clone(),
            severity: rule.severity.clone(),
            description: rule.description.clone().unwrap_or_else(|| rule.name.clone()),
            source_index: rule.source_index.clone(),
            evidence: IncidentEvidence {
                count: rule_match.count,
                window_seconds: rule.threshold.window_seconds,
                group_by: rule.group_by.clone(),
                values: rule_match.values,
            },
        };

        client.index_incident(&rule.target_index, &correlation_key, &incident)?;

        println!(
            "🚨 Incidente aggregation indicizzato: {} correlation_key={} severity={} count={}",
            incident_id, correlation_key, rule.severity, incident.evidence.count
        );
    }

    Ok(())
}

fn execute_correlation_rule(
    client: &OpenSearchIncidentClient,
    rule: &CorrelationRule,
) -> Result<()> {
    if rule.conditions.is_empty() {
        bail!("rule {} has empty conditions", rule.id);
    }

    println!("🔎 Eseguo correlation rule: {}", rule.id);

    let mut total_count = 0;
    let mut values = HashMap::new();

    for (index, condition) in rule.conditions.iter().enumerate() {
        let query = build_condition_query(condition, rule.window_seconds);

        let response = client.search(&rule.source_index, query)?;

        let count = extract_total_hits(&response);

        if count == 0 {
            println!(
                "✅ Correlation rule {}: condition {} non trovata",
                rule.id,
                index + 1
            );
            return Ok(());
        }

        total_count += count;

        for (field, expected) in condition {
            values.insert(
                format!("condition_{}.{}", index + 1, field),
                value_to_key(expected),
            );
        }

        values.insert(format!("condition_{}.count", index + 1), count.to_string());
    }

    let correlation_key = build_correlation_incident_id(rule);
    let incident_id = stable_incident_uuid(&correlation_key);

    let incident = NautilusIncident {
        timestamp: Utc::now().to_rfc3339(),
        incident_id: incident_id.clone(),
        correlation_key: correlation_key.clone(),
        rule_id: rule.id.clone(),
        rule_uuid: rule.uuid.clone(),
        rule_name: rule.name.clone(),
        incident_type: rule.incident_type.clone(),
        severity: rule.severity.clone(),
        description: rule.description.clone().unwrap_or_else(|| rule.name.clone()),
        source_index: rule.source_index.clone(),
        evidence: IncidentEvidence {
            count: total_count,
            window_seconds: rule.window_seconds,
            group_by: Vec::new(),
            values,
        },
    };

    client.index_incident(&rule.target_index, &correlation_key, &incident)?;

    println!(
        "🚨 Incidente correlation indicizzato: {} correlation_key={} severity={} count={}",
        incident_id, correlation_key, rule.severity, incident.evidence.count
    );

    Ok(())
}

#[derive(Debug)]
struct RuleMatch {
    count: u64,
    values: HashMap<String, String>,
}

fn build_aggregation_query(rule: &AggregationRule) -> Value {
    let mut filters = Vec::new();

    for (field, expected) in &rule.filter {
        filters.push(json!({
            "term": {
                field: expected
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

fn build_condition_query(condition: &HashMap<String, Value>, window_seconds: i64) -> Value {
    let mut filters = Vec::new();

    for (field, expected) in condition {
        filters.push(json!({
            "term": {
                field: expected
            }
        }));
    }

    filters.push(json!({
        "range": {
            "timestamp": {
                "gte": format!("now-{}s", window_seconds),
                "lte": "now"
            }
        }
    }));

    json!({
        "size": 1,
        "query": {
            "bool": {
                "filter": filters
            }
        }
    })
}

fn extract_aggregation_matches(rule: &AggregationRule, response: &Value) -> Result<Vec<RuleMatch>> {
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
            let count = bucket.get("doc_count").and_then(|v| v.as_u64()).unwrap_or(0);

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
            let count = bucket2.get("doc_count").and_then(|v| v.as_u64()).unwrap_or(0);

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

fn extract_total_hits(response: &Value) -> u64 {
    response
        .pointer("/hits/total/value")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
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

fn build_aggregation_incident_id(rule: &AggregationRule, values: &HashMap<String, String>) -> String {
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

fn build_correlation_incident_id(rule: &CorrelationRule) -> String {
    let now = Utc::now().timestamp();
    let window = rule.window_seconds.max(1);
    let window_start = now - (now % window);

    format!(
        "{}-{}",
        sanitize_id_part(&rule.id),
        window_start
    )
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

fn stable_incident_uuid(correlation_key: &str) -> String {
    // UUID v5 deterministico: stessa correlation_key => stesso incident_id.
    // In questo modo OpenSearch usa correlation_key come _id per la deduplica,
    // mentre incident_id rimane un UUID stabile e pulito da mostrare lato UI/API.
    let namespace = Uuid::NAMESPACE_URL;

    Uuid::new_v5(&namespace, correlation_key.as_bytes()).to_string()
}

fn value_to_key(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}