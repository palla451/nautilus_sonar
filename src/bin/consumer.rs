use anyhow::Result;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{Client, Commands, Value};
use reqwest::blocking::Client as HttpClient;
use std::env;
use std::time::Duration;

#[path = "../model.rs"]
mod model;

use model::ProbeEvent;

struct BatchItem {
    id: String,
    event: ProbeEvent,
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

    let backend_enabled = env::var("BACKEND_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .eq_ignore_ascii_case("true");

    let backend_url = env::var("BACKEND_URL")
        .unwrap_or_else(|_| "http://backend.local/api/events".to_string());

    let bearer_token = env::var("BEARER_TOKEN")
        .unwrap_or_else(|_| "test-token".to_string());

    let backend_batch_size: usize = env::var("BACKEND_BATCH_SIZE")
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
    println!("📦 Backend batch size: {}", backend_batch_size);

    if backend_enabled {
        println!("📤 Backend abilitato: {}", backend_url);
    } else {
        println!("📤 Backend disabilitato");
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
        )?;

        let opts = StreamReadOptions::default()
            .group(&group, &consumer)
            .count(backend_batch_size)
            .block(0);

        let reply: StreamReadReply =
            con.xread_options(&[stream.as_str()], &[">"], &opts)?;

        let mut batch: Vec<BatchItem> = Vec::new();

        for stream_key in reply.keys {
            for stream_id in stream_key.ids {
                let id = stream_id.id.clone();

                let Some(value) = stream_id.map.get("payload") else {
                    eprintln!("❌ Evento senza payload [{}]", id);

                    send_to_dead_letter(
                        &mut con,
                        &dead_letter_stream,
                        &id,
                        "",
                        "missing_payload",
                    )?;

                    ack_message(&mut con, &stream, &group, &id)?;
                    continue;
                };

                let payload: String = redis::from_redis_value(value)?;

                println!("📩 Evento ricevuto [{}]", id);

                let event: ProbeEvent = match serde_json::from_str(&payload) {
                    Ok(event) => event,
                    Err(err) => {
                        eprintln!("❌ Errore deserializzazione JSON [{}]: {}", id, err);

                        send_to_dead_letter(
                            &mut con,
                            &dead_letter_stream,
                            &id,
                            &payload,
                            &format!("deserialization_error: {}", err),
                        )?;

                        ack_message(&mut con, &stream, &group, &id)?;
                        continue;
                    }
                };

                log_event(&event)?;

                if backend_enabled {
                    batch.push(BatchItem { id, event });
                } else {
                    println!("📤 Backend disabilitato, evento non inviato");
                    ack_message(&mut con, &stream, &group, &id)?;
                }
            }
        }

        if backend_enabled && !batch.is_empty() {
            match send_batch_to_backend(&batch, &backend_url, &bearer_token, &http_client) {
                Ok(_) => {
                    for item in batch {
                        ack_message(&mut con, &stream, &group, &item.id)?;
                    }
                }
                Err(err) => {
                    eprintln!("❌ Errore invio batch backend: {}", err);
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
        ) {
            eprintln!("❌ Retry fallito [{}]: {}", id, err);
            eprintln!("⏳ NO ACK: evento resta pending [{}]", id);
        }
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
) -> Result<()> {
    println!("🔁 Retry evento pending [{}]", id);

    let event: ProbeEvent = match serde_json::from_str(payload) {
        Ok(event) => event,
        Err(err) => {
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

    log_event(&event)?;

    if backend_enabled {
        send_to_backend(&event, backend_url, bearer_token, http_client)?;
    } else {
        println!("📤 Backend disabilitato, evento non inviato");
    }

    ack_message(con, stream, group, id)?;

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

fn send_to_backend(
    event: &ProbeEvent,
    backend_url: &str,
    bearer_token: &str,
    http_client: &HttpClient,
) -> Result<()> {
    let response = http_client
        .post(backend_url)
        .bearer_auth(bearer_token)
        .json(event)
        .send()?;

    if !response.status().is_success() {
        anyhow::bail!("backend returned HTTP {}", response.status());
    }

    println!("📤 Evento inviato al backend");

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

    println!("📤 Batch inviato al backend: {} eventi", events.len());

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