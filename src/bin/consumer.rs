use anyhow::Result;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{Client, Commands, Value};
use reqwest::blocking::Client as HttpClient;
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[path = "../model.rs"]
mod model;

#[path = "../opensearch.rs"]
mod opensearch;

use model::ProbeEvent;
use opensearch::OpenSearchClient;

const METRICS_EVERY_EVENTS: u64 = 100;

static EVENTS_RECEIVED: AtomicU64 = AtomicU64::new(0);
static EVENTS_PROCESSED: AtomicU64 = AtomicU64::new(0);
static EVENTS_ACKED: AtomicU64 = AtomicU64::new(0);
static EVENTS_DLQ: AtomicU64 = AtomicU64::new(0);
static EVENTS_RETRIED: AtomicU64 = AtomicU64::new(0);
static BACKEND_SENT: AtomicU64 = AtomicU64::new(0);
static BACKEND_ERRORS: AtomicU64 = AtomicU64::new(0);
static OPENSEARCH_SENT: AtomicU64 = AtomicU64::new(0);
static OPENSEARCH_ERRORS: AtomicU64 = AtomicU64::new(0);
static JSON_ERRORS: AtomicU64 = AtomicU64::new(0);
static MISSING_PAYLOAD: AtomicU64 = AtomicU64::new(0);

pub struct BatchItem {
    pub id: String,
    pub event: ProbeEvent,
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    println!("🚀 Nautilus consumer avviato");

    let valkey_url = env::var("VALKEY_URL")
        .unwrap_or_else(|_| "redis://valkey:6379".to_string());

    let stream = env::var("VALKEY_STREAM")
        .unwrap_or_else(|_| "nautilus-stream".to_string());

    let dead_letter_stream = env::var("VALKEY_DEAD_LETTER_STREAM")
        .unwrap_or_else(|_| "nautilus-dead-letter-stream".to_string());

    let group = env::var("VALKEY_GROUP")
        .unwrap_or_else(|_| "sonar-group".to_string());

    let consumer = env::var("VALKEY_CONSUMER")
        .unwrap_or_else(|_| "consumer-1".to_string());

    let pending_idle_ms: usize = env::var("VALKEY_PENDING_IDLE_MS")
        .unwrap_or_else(|_| "60000".to_string())
        .parse()
        .unwrap_or(60000);

    let pending_count: usize = env::var("VALKEY_PENDING_COUNT")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10);

    let read_block_ms: usize = env::var("VALKEY_READ_BLOCK_MS")
        .unwrap_or_else(|_| "5000".to_string())
        .parse()
        .unwrap_or(5000);

    let backend_enabled = env::var("BACKEND_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .eq_ignore_ascii_case("true");

    let backend_url = env::var("BACKEND_URL")
        .unwrap_or_else(|_| "http://backend.local/api/events".to_string());

    let bearer_token = env::var("BEARER_TOKEN")
        .unwrap_or_else(|_| "test-token".to_string());

    let opensearch_enabled = env::var("OPENSEARCH_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .eq_ignore_ascii_case("true");

    let batch_size: usize = env::var("OPENSEARCH_BATCH_SIZE")
        .or_else(|_| env::var("BACKEND_BATCH_SIZE"))
        .unwrap_or_else(|_| "100".to_string())
        .parse()
        .unwrap_or(100);

    println!("🔌 ValKey URL: {}", valkey_url);
    println!("📥 Stream: {}", stream);
    println!("☠️ Dead Letter Stream: {}", dead_letter_stream);
    println!("👥 Group: {}", group);
    println!("👤 Consumer: {}", consumer);
    println!("⏳ Pending idle ms: {}", pending_idle_ms);
    println!("🔁 Pending retry count: {}", pending_count);
    println!("⏱️ Read block ms: {}", read_block_ms);
    println!("📦 Batch size: {}", batch_size);

    if backend_enabled {
        println!("📤 Backend abilitato: {}", backend_url);
    } else {
        println!("📤 Backend disabilitato");
    }

    let opensearch_client = if opensearch_enabled {
        println!("🔍 OpenSearch abilitato");
        Some(OpenSearchClient::new()?)
    } else {
        println!("🔍 OpenSearch disabilitato");
        None
    };

    if backend_enabled && opensearch_enabled {
        println!("⚠️ Backend e OpenSearch entrambi abilitati: ACK solo dopo invio riuscito su entrambi");
    }

    if !backend_enabled && !opensearch_enabled {
        println!("ℹ️ Nessun sink esterno abilitato: ACK dopo processing locale");
    }

    let redis_client = Client::open(valkey_url)?;
    let mut con = redis_client.get_connection()?;

    let http_client = HttpClient::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    ensure_group(&mut con, &stream, &group)?;

    loop {
        retry_pending_messages(
            &mut con,
            &stream,
            &group,
            &consumer,
            &dead_letter_stream,
            pending_idle_ms,
            pending_count,
            backend_enabled,
            &backend_url,
            &bearer_token,
            &http_client,
            opensearch_client.as_ref(),
        )?;

        let opts = StreamReadOptions::default()
            .group(&group, &consumer)
            .count(batch_size)
            .block(read_block_ms);

        let reply: StreamReadReply = con.xread_options(&[stream.as_str()], &[">"], &opts)?;

        let mut batch: Vec<BatchItem> = Vec::new();

        for stream_key in reply.keys {
            for stream_id in stream_key.ids {
                let id = stream_id.id.clone();
                EVENTS_RECEIVED.fetch_add(1, Ordering::Relaxed);

                let Some(value) = stream_id.map.get("payload") else {
                    MISSING_PAYLOAD.fetch_add(1, Ordering::Relaxed);
                    eprintln!("❌ Evento senza payload [{}]", id);

                    send_to_dead_letter(
                        &mut con,
                        &dead_letter_stream,
                        &id,
                        "",
                        "missing_payload",
                    )?;

                    ack_message(&mut con, &stream, &group, &id)?;
                    print_metrics_if_needed();
                    continue;
                };

                let payload: String = redis::from_redis_value(value)?;

                println!("📩 Evento ricevuto [{}]", id);

                let event: ProbeEvent = match serde_json::from_str(&payload) {
                    Ok(event) => event,
                    Err(err) => {
                        JSON_ERRORS.fetch_add(1, Ordering::Relaxed);
                        eprintln!("❌ Errore deserializzazione JSON [{}]: {}", id, err);

                        send_to_dead_letter(
                            &mut con,
                            &dead_letter_stream,
                            &id,
                            &payload,
                            &format!("deserialization_error: {}", err),
                        )?;

                        ack_message(&mut con, &stream, &group, &id)?;
                        print_metrics_if_needed();
                        continue;
                    }
                };

                EVENTS_PROCESSED.fetch_add(1, Ordering::Relaxed);
                log_event(&event)?;

                if backend_enabled || opensearch_client.is_some() {
                    batch.push(BatchItem { id, event });
                } else {
                    println!("📤 Sink esterni disabilitati, evento non inviato");
                    ack_message(&mut con, &stream, &group, &id)?;
                }

                print_metrics_if_needed();
            }
        }

        if !batch.is_empty() {
            match send_batch_to_sinks(
                &batch,
                backend_enabled,
                &backend_url,
                &bearer_token,
                &http_client,
                opensearch_client.as_ref(),
            ) {
                Ok(_) => {
                    for item in batch {
                        ack_message(&mut con, &stream, &group, &item.id)?;
                    }
                }
                Err(err) => {
                    eprintln!("❌ Errore invio batch ai sink: {}", err);
                    eprintln!("⏳ NO ACK: batch lasciato pending per retry");
                }
            }
        }
    }
}

fn retry_pending_messages(
    con: &mut redis::Connection,
    stream: &str,
    group: &str,
    consumer: &str,
    dead_letter_stream: &str,
    pending_idle_ms: usize,
    pending_count: usize,
    backend_enabled: bool,
    backend_url: &str,
    bearer_token: &str,
    http_client: &HttpClient,
    opensearch_client: Option<&OpenSearchClient>,
) -> Result<()> {
    let value: Value = redis::cmd("XAUTOCLAIM")
        .arg(stream)
        .arg(group)
        .arg(consumer)
        .arg(pending_idle_ms)
        .arg("0-0")
        .arg("COUNT")
        .arg(pending_count)
        .query(con)?;

    let entries = parse_xautoclaim_entries(value);

    if !entries.is_empty() {
        EVENTS_RETRIED.fetch_add(entries.len() as u64, Ordering::Relaxed);
        println!("🔁 Retry pending messages: {}", entries.len());
    }

    for (id, payload) in entries {
        if let Err(err) = handle_pending_message(
            con,
            stream,
            group,
            dead_letter_stream,
            &id,
            &payload,
            backend_enabled,
            backend_url,
            bearer_token,
            http_client,
            opensearch_client,
        ) {
            if opensearch_client.is_some() {
                OPENSEARCH_ERRORS.fetch_add(1, Ordering::Relaxed);
            }

            if backend_enabled {
                BACKEND_ERRORS.fetch_add(1, Ordering::Relaxed);
            }

            eprintln!("❌ Retry fallito [{}]: {}", id, err);
            eprintln!("⏳ NO ACK: evento resta pending [{}]", id);
        }

        print_metrics_if_needed();
    }

    Ok(())
}

fn handle_pending_message(
    con: &mut redis::Connection,
    stream: &str,
    group: &str,
    dead_letter_stream: &str,
    id: &str,
    payload: &str,
    backend_enabled: bool,
    backend_url: &str,
    bearer_token: &str,
    http_client: &HttpClient,
    opensearch_client: Option<&OpenSearchClient>,
) -> Result<()> {
    println!("🔁 Retry evento pending [{}]", id);

    let event: ProbeEvent = match serde_json::from_str(payload) {
        Ok(event) => event,
        Err(err) => {
            JSON_ERRORS.fetch_add(1, Ordering::Relaxed);
            eprintln!("❌ Errore deserializzazione JSON [{}]: {}", id, err);

            send_to_dead_letter(
                con,
                dead_letter_stream,
                id,
                payload,
                &format!("deserialization_error: {}", err),
            )?;

            ack_message(con, stream, group, id)?;
            return Ok(());
        }
    };

    EVENTS_PROCESSED.fetch_add(1, Ordering::Relaxed);
    log_event(&event)?;

    if backend_enabled || opensearch_client.is_some() {
        let batch = vec![BatchItem {
            id: id.to_string(),
            event,
        }];

        send_batch_to_sinks(
            &batch,
            backend_enabled,
            backend_url,
            bearer_token,
            http_client,
            opensearch_client,
        )?;
    } else {
        println!("📤 Sink esterni disabilitati, evento non inviato");
    }

    ack_message(con, stream, group, id)?;

    Ok(())
}

fn send_batch_to_sinks(
    batch: &[BatchItem],
    backend_enabled: bool,
    backend_url: &str,
    bearer_token: &str,
    http_client: &HttpClient,
    opensearch_client: Option<&OpenSearchClient>,
) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    if let Some(os_client) = opensearch_client {
        match os_client.send_batch(batch) {
            Ok(_) => {
                OPENSEARCH_SENT.fetch_add(batch.len() as u64, Ordering::Relaxed);
                println!("🔍 OpenSearch batch inviato: {} eventi", batch.len());
            }
            Err(err) => {
                OPENSEARCH_ERRORS.fetch_add(1, Ordering::Relaxed);
                anyhow::bail!("OpenSearch send failed: {}", err);
            }
        }
    }

    if backend_enabled {
        match send_batch_to_backend(batch, backend_url, bearer_token, http_client) {
            Ok(_) => {
                BACKEND_SENT.fetch_add(batch.len() as u64, Ordering::Relaxed);
            }
            Err(err) => {
                BACKEND_ERRORS.fetch_add(1, Ordering::Relaxed);
                anyhow::bail!("Backend send failed: {}", err);
            }
        }
    }

    Ok(())
}

fn parse_xautoclaim_entries(value: Value) -> Vec<(String, String)> {
    let mut result = Vec::new();

    let Value::Bulk(items) = value else {
        return result;
    };

    if items.len() < 2 {
        return result;
    }

    let Value::Bulk(entries) = &items[1] else {
        return result;
    };

    for entry in entries {
        let Value::Bulk(entry_items) = entry else {
            continue;
        };

        if entry_items.len() < 2 {
            continue;
        }

        let Some(id) = redis_value_to_string(&entry_items[0]) else {
            continue;
        };

        let Value::Bulk(fields) = &entry_items[1] else {
            continue;
        };

        let mut payload: Option<String> = None;
        let mut i = 0;

        while i + 1 < fields.len() {
            let key = redis_value_to_string(&fields[i]);
            let val = redis_value_to_string(&fields[i + 1]);

            if key.as_deref() == Some("payload") {
                payload = val;
                break;
            }

            i += 2;
        }

        if let Some(payload) = payload {
            result.push((id, payload));
        }
    }

    result
}

fn redis_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Data(bytes) => String::from_utf8(bytes.clone()).ok(),
        Value::Status(s) => Some(s.clone()),
        Value::Okay => Some("OK".to_string()),
        Value::Int(i) => Some(i.to_string()),
        _ => None,
    }
}

fn log_event(event: &ProbeEvent) -> Result<()> {
    println!(
        "✅ Evento processato: type={}, src={}:{} -> dst={}:{}, proto={}, app_proto={}",
        event.event_type,
        event.src_ip.clone().unwrap_or_else(|| "-".to_string()),
        event.src_port
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string()),
        event.dest_ip.clone().unwrap_or_else(|| "-".to_string()),
        event.dest_port
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string()),
        event.proto.clone().unwrap_or_else(|| "-".to_string()),
        event.app_proto.clone().unwrap_or_else(|| "-".to_string())
    );

    Ok(())
}

fn send_batch_to_backend(
    batch: &[BatchItem],
    backend_url: &str,
    bearer_token: &str,
    http_client: &HttpClient,
) -> Result<()> {
    let events: Vec<&ProbeEvent> = batch.iter().map(|item| &item.event).collect();

    let response = http_client
        .post(backend_url)
        .bearer_auth(bearer_token)
        .json(&events)
        .send()?;

    if !response.status().is_success() {
        anyhow::bail!("backend returned HTTP {}", response.status());
    }

    println!("📤 Backend batch inviato: {} eventi", events.len());

    Ok(())
}

fn send_to_dead_letter(
    con: &mut redis::Connection,
    dead_letter_stream: &str,
    original_id: &str,
    payload: &str,
    reason: &str,
) -> Result<()> {
    redis::cmd("XADD")
        .arg(dead_letter_stream)
        .arg("*")
        .arg("original_id")
        .arg(original_id)
        .arg("reason")
        .arg(reason)
        .arg("payload")
        .arg(payload)
        .query::<()>(con)?;

    EVENTS_DLQ.fetch_add(1, Ordering::Relaxed);

    println!(
        "☠️ Evento inviato in dead-letter-stream [{}]: {}",
        original_id, reason
    );

    Ok(())
}

fn ack_message(
    con: &mut redis::Connection,
    stream: &str,
    group: &str,
    id: &str,
) -> Result<()> {
    let _: i32 = con.xack(stream, group, &[id])?;
    EVENTS_ACKED.fetch_add(1, Ordering::Relaxed);
    println!("✅ ACK inviato [{}]", id);
    Ok(())
}

fn ensure_group(
    con: &mut redis::Connection,
    stream: &str,
    group: &str,
) -> Result<()> {
    let result: redis::RedisResult<String> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(stream)
        .arg(group)
        .arg("0")
        .arg("MKSTREAM")
        .query(con);

    match result {
        Ok(_) => println!("✅ Consumer group creato: {}", group),
        Err(err) => {
            if err.to_string().contains("BUSYGROUP") {
                println!("ℹ️ Consumer group già esistente: {}", group);
            } else {
                return Err(err.into());
            }
        }
    }

    Ok(())
}

fn print_metrics_if_needed() {
    let received = EVENTS_RECEIVED.load(Ordering::Relaxed);

    if received == 0 || received % METRICS_EVERY_EVENTS != 0 {
        return;
    }

    println!(
        "📊 CONSUMER metrics | received={} processed={} acked={} dlq={} retried={} backend_sent={} backend_errors={} opensearch_sent={} opensearch_errors={} json_errors={} missing_payload={}",
        received,
        EVENTS_PROCESSED.load(Ordering::Relaxed),
        EVENTS_ACKED.load(Ordering::Relaxed),
        EVENTS_DLQ.load(Ordering::Relaxed),
        EVENTS_RETRIED.load(Ordering::Relaxed),
        BACKEND_SENT.load(Ordering::Relaxed),
        BACKEND_ERRORS.load(Ordering::Relaxed),
        OPENSEARCH_SENT.load(Ordering::Relaxed),
        OPENSEARCH_ERRORS.load(Ordering::Relaxed),
        JSON_ERRORS.load(Ordering::Relaxed),
        MISSING_PAYLOAD.load(Ordering::Relaxed),
    );
}