use crate::model::ProbeEvent;
use anyhow::Result;
use chrono::Utc;
use std::fs::{self, create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_FILE_SIZE_MB: u64 = 10;
const MAX_FILE_AGE_HOURS: u64 = 24;

pub fn dispatch(event: &ProbeEvent) -> Result<()> {
    let output_dir = Path::new("output");

    create_dir_all(output_dir)?;

    cleanup_old_files(output_dir)?;

    let output_file = get_current_output_file(output_dir)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&output_file)?;

    let json = serde_json::to_string(event)?;

    writeln!(file, "{json}")?;

    Ok(())
}

fn get_current_output_file(output_dir: &Path) -> Result<PathBuf> {
    let current_file = output_dir.join("events.jsonl");

    if current_file.exists() {
        let metadata = fs::metadata(&current_file)?;

        let size_mb = metadata.len() / 1024 / 1024;

        if size_mb >= MAX_FILE_SIZE_MB {
            rotate_file(&current_file)?;
        }
    }

    Ok(current_file)
}

fn rotate_file(current_file: &Path) -> Result<()> {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");

    let rotated_file = current_file.with_file_name(format!(
        "events-{}.jsonl",
        timestamp
    ));

    fs::rename(current_file, rotated_file)?;

    Ok(())
}

fn cleanup_old_files(output_dir: &Path) -> Result<()> {
    let now = SystemTime::now();

    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;

        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let metadata = fs::metadata(&path)?;

        let modified = metadata.modified()?;

        let age = now
            .duration_since(modified)
            .unwrap_or(Duration::from_secs(0));

        if age > Duration::from_secs(MAX_FILE_AGE_HOURS * 3600) {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}