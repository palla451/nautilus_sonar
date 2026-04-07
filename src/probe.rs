use anyhow::Result;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::Path;
use uuid::Uuid;

const PROBE_ID_FILE: &str = "output/probe_id";

pub fn get_or_create_probe_id() -> Result<String> {
    let path = Path::new(PROBE_ID_FILE);

    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }

    if path.exists() {
        let probe_id = read_to_string(path)?.trim().to_string();

        if !probe_id.is_empty() {
            return Ok(probe_id);
        }
    }

    let probe_id = Uuid::new_v4().to_string();
    write(path, &probe_id)?;

    Ok(probe_id)
}