use crate::model::ProbeEvent;
use anyhow::Result;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn dispatch(event: &ProbeEvent) -> Result<()> {
    let output_dir = Path::new("output");
    let output_file = output_dir.join("events.jsonl");

    create_dir_all(output_dir)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&output_file)?;

    let json = serde_json::to_string(event)?;
    writeln!(file, "{json}")?;

    println!("DISPATCH PATH: {}", output_file.display());

    Ok(())
}