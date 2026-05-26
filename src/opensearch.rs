use crate::model::ProbeEvent;
use crate::BatchItem;
use anyhow::{bail, Result};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::env;
use std::time::Duration;

pub struct OpenSearchClient {
    client: Client,
    url: String,
    index: String,
    username: String,
    password: String,
}

impl OpenSearchClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()?;

        let url = env::var("OPENSEARCH_URL")
            .unwrap_or_else(|_| "http://opensearch:9200".to_string())
            .trim_end_matches('/')
            .to_string();

        Ok(Self {
            client,
            url,
            index: env::var("OPENSEARCH_INDEX")
                .unwrap_or_else(|_| "nautilus-events".to_string()),
            username: env::var("OPENSEARCH_USERNAME")
                .unwrap_or_else(|_| "admin".to_string()),
            password: env::var("OPENSEARCH_PASSWORD")
                .unwrap_or_else(|_| "admin".to_string()),
        })
    }

    pub fn send_batch(&self, batch: &[BatchItem]) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        let mut bulk_body = String::new();

        for item in batch {
            let metadata = json!({
                "index": {
                    "_index": self.index,
                    "_id": item.id
                }
            });

            bulk_body.push_str(&serde_json::to_string(&metadata)?);
            bulk_body.push('\n');

            bulk_body.push_str(&serde_json::to_string(&item.event)?);
            bulk_body.push('\n');
        }

        let response = self
            .client
            .post(format!("{}/_bulk", self.url))
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "application/x-ndjson")
            .body(bulk_body)
            .send()?;

        let status = response.status();
        let body = response.text()?;

        if !status.is_success() {
            bail!("OpenSearch HTTP error {}: {}", status, body);
        }

        let parsed: Value = serde_json::from_str(&body)?;

        let has_errors = parsed
            .get("errors")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        if has_errors {
            bail!("OpenSearch bulk completed with item errors: {}", parsed);
        }

        println!("📦 OpenSearch batch sent: {} eventi", batch.len());

        Ok(())
    }
}