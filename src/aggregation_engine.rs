use crate::rules_client::RemoteRule;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
struct WindowEntry {
    timestamp: DateTime<Utc>,
}

pub struct AggregationEngine {
    rules: Vec<RemoteRule>,
    windows: HashMap<String, VecDeque<WindowEntry>>,
}

impl AggregationEngine {
    pub fn new(rules: Vec<RemoteRule>) -> Self {
        Self {
            rules,
            windows: HashMap::new(),
        }
    }

    pub fn reload_rules(&mut self, rules: Vec<RemoteRule>) {
        self.rules = rules;
        self.windows.clear();

        println!("🔁 Aggregation engine ricaricato con nuove regole");
    }

    pub fn process_event(&mut self, event: &Value) -> Result<Vec<Value>> {
        let mut incidents = Vec::new();

        for rule in &self.rules {
            if rule.rule_type != "aggregation" {
                continue;
            }

            let content = &rule.content;

            if !self.event_matches_filter(event, content) {
                continue;
            }

            let group_key = self.build_group_key(rule, event);

            let threshold_count = content
                .pointer("/threshold/count")
                .and_then(|v| v.as_u64())
                .unwrap_or(1);

            let window_seconds = content
                .pointer("/threshold/window_seconds")
                .and_then(|v| v.as_i64())
                .unwrap_or(60);

            let now = Utc::now();

            let window = self
                .windows
                .entry(group_key.clone())
                .or_insert_with(VecDeque::new);

            window.push_back(WindowEntry { timestamp: now });

            while let Some(front) = window.front() {
                let age = now.signed_duration_since(front.timestamp).num_seconds();

                if age > window_seconds {
                    window.pop_front();
                } else {
                    break;
                }
            }

            if window.len() as u64 >= threshold_count {
                let incident = json!({
                    "incident_type": content.pointer("/incident_type").and_then(|v| v.as_str()).unwrap_or("aggregation_incident"),
                    "rule_uuid": rule.uuid,
                    "rule_name": rule.name,
                    "severity": content.pointer("/severity").and_then(|v| v.as_str()).unwrap_or("low"),
                    "event_count": window.len(),
                    "window_seconds": window_seconds,
                    "group_key": group_key,
                    "last_event": event,
                    "timestamp": now.to_rfc3339(),
                    "source": "local_aggregation_engine"
                });

                incidents.push(incident);

                window.clear();
            }
        }

        Ok(incidents)
    }

    fn event_matches_filter(&self, event: &Value, content: &Value) -> bool {
        let Some(filter) = content.get("filter") else {
            return true;
        };

        let Some(filter_obj) = filter.as_object() else {
            return true;
        };

        for (field, expected) in filter_obj {
            let actual = get_path_value(event, field);

            if actual != Some(expected) {
                return false;
            }
        }

        true
    }

    fn build_group_key(&self, rule: &RemoteRule, event: &Value) -> String {
        let group_by = rule
            .content
            .get("group_by")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut parts = vec![rule.uuid.clone()];

        for field in group_by {
            if let Some(field_name) = field.as_str() {
                let value = get_path_value(event, field_name)
                    .map(value_to_key)
                    .unwrap_or_else(|| "null".to_string());

                parts.push(format!("{}={}", field_name, value));
            }
        }

        parts.join("|")
    }
}

fn get_path_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let pointer = format!("/{}", path.replace('.', "/"));
    value.pointer(&pointer)
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