use anyhow::Result;
use redis::{Client, Commands};
use crate::model::ProbeEvent;

pub fn publish_event(event: &ProbeEvent) -> Result<()> {
    let client = Client::open("redis://valkey:6379")?;

    let mut con = client.get_connection()?;

    let json = serde_json::to_string(event)?;

    redis::cmd("XADD")
        .arg("nautilus-stream")
        .arg("*")
        .arg("payload")
        .arg(json)
        .query::<()>(&mut con)?;

    Ok(())
}