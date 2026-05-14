use crate::model::ProbeEvent;
use anyhow::Result;
use chrono::Utc;
use std::env;
use std::fs::{self, create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

const DEFAULT_MAX_FILE_SIZE_MB: u64 = 10;
const DEFAULT_MAX_FILE_AGE_HOURS: u64 = 24;
const METRICS_EVERY_EVENTS: u64 = 1000;

static EVENTS_DISPATCHED: AtomicU64 = AtomicU64::new(0);
static EVENTS_WRITTEN_TO_FILE: AtomicU64 = AtomicU64::new(0);
static EVENTS_PUBLISHED_TO_VALKEY: AtomicU64 = AtomicU64::new(0);
static VALKEY_PUBLISH_ERRORS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_ROTATIONS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_CLEANUP_DELETIONS: AtomicU64 = AtomicU64::new(0);

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

    EVENTS_DISPATCHED.fetch_add(1, Ordering::Relaxed);
    EVENTS_WRITTEN_TO_FILE.fetch_add(1, Ordering::Relaxed);

    match crate::queue::publish_event(event) {
        Ok(_) => {
            EVENTS_PUBLISHED_TO_VALKEY.fetch_add(1, Ordering::Relaxed);
        }
        Err(err) => {
            VALKEY_PUBLISH_ERRORS.fetch_add(1, Ordering::Relaxed);
            eprintln!("❌ ValKey publish error: {}", err);
        }
    }

    print_metrics_if_needed();

    Ok(())
}

fn get_current_output_file(output_dir: &Path) -> Result<PathBuf> {
    let current_file = output_dir.join("events.jsonl");

    if current_file.exists() {
        let metadata = fs::metadata(&current_file)?;
        let max_size_bytes = max_file_size_mb() * 1024 * 1024;

        if metadata.len() >= max_size_bytes {
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

    OUTPUT_ROTATIONS.fetch_add(1, Ordering::Relaxed);

    println!("🔄 Output rotated: {}", current_file.display());

    Ok(())
}

fn cleanup_old_files(output_dir: &Path) -> Result<()> {
    let now = SystemTime::now();
    let max_age = Duration::from_secs(max_file_age_hours() * 3600);

    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !file_name.starts_with("events-") || !file_name.ends_with(".jsonl") {
            continue;
        }

        let metadata = fs::metadata(&path)?;
        let modified = metadata.modified()?;

        let age = now
            .duration_since(modified)
            .unwrap_or(Duration::from_secs(0));

        if age > max_age {
            fs::remove_file(&path)?;
            OUTPUT_CLEANUP_DELETIONS.fetch_add(1, Ordering::Relaxed);
            println!("🧹 Old output removed: {}", path.display());
        }
    }

    Ok(())
}

fn max_file_size_mb() -> u64 {
    env::var("OUTPUT_MAX_FILE_SIZE_MB")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MAX_FILE_SIZE_MB)
}

fn max_file_age_hours() -> u64 {
    env::var("OUTPUT_MAX_FILE_AGE_HOURS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MAX_FILE_AGE_HOURS)
}

fn print_metrics_if_needed() {
    let dispatched = EVENTS_DISPATCHED.load(Ordering::Relaxed);

    if dispatched == 0 || dispatched % METRICS_EVERY_EVENTS != 0 {
        return;
    }

    println!(
        "📊 SONAR metrics | dispatched={} written_file={} published_valkey={} valkey_errors={} rotations={} cleanup_deleted={}",
        dispatched,
        EVENTS_WRITTEN_TO_FILE.load(Ordering::Relaxed),
        EVENTS_PUBLISHED_TO_VALKEY.load(Ordering::Relaxed),
        VALKEY_PUBLISH_ERRORS.load(Ordering::Relaxed),
        OUTPUT_ROTATIONS.load(Ordering::Relaxed),
        OUTPUT_CLEANUP_DELETIONS.load(Ordering::Relaxed),
    );
}