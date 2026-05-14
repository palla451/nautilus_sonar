use crate::model::ProbeEvent;
use anyhow::{bail, Result};
use reqwest::blocking::Client;
use serde_json::json;
use std::env;

pub struct OpenSearchClient {
    client: Client,
    url: String,
    index: String,
    username: String,
    password: String,
}

impl OpenSearchClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder().build()?;

        Ok(Self {
            client,
            url: env::var("OPENSEARCH_URL")
                .unwrap_or_else(|_| "http://opensearch:9200".to_string()),

            index: env::var("OPENSEARCH_INDEX")
                .unwrap_or_else(|_| "nautilus-events".to_string()),

            username: env::var("OPENSEARCH_USERNAME")
                .unwrap_or_else(|_| "admin".to_string()),

            password: env::var("OPENSEARCH_PASSWORD")
                .unwrap_or_else(|_| "admin".to_string()),
        })
    }

    pub fn send_batch(
        &self,
        events: &[ProbeEvent],
    ) -> Result<()> {

        if events.is_empty() {
            return Ok(());
        }

        let mut bulk_body = String::new();

        for event in events {

            let metadata = json!({
                "index": {
                    "_index": self.index
                }
            });

            bulk_body.push_str(
                &serde_json::to_string(&metadata)?
            );

            bulk_body.push('\n');

            bulk_body.push_str(
                &serde_json::to_string(event)?
            );

            bulk_body.push('\n');
        }

        let response = self.client
            .post(format!("{}/_bulk", self.url))
            .basic_auth(
                &self.username,
                Some(&self.password)
            )
            .header(
                "Content-Type",
                "application/x-ndjson"
            )
            .body(bulk_body)
            .send()?;

        if !response.status().is_success() {

            let text = response.text()?;

            bail!(
                "OpenSearch error: {}",
                text
            );
        }

        println!(
            "📦 OpenSearch batch sent: {}",
            events.len()
        );

        Ok(())
    }
}