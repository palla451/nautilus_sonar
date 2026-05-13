use crate::model::{
    AlertPayload, DnsPayload, FlowPayload, HostInfo, HttpPayload, ProbeEvent, ProbeInfo,
    ProbePayload, TlsPayload,
};
use serde_json::Value;

pub fn normalize_event(
    raw: &Value,
    probe_id: &str,
    sensor_name: &str,
    host: &HostInfo,
) -> Option<ProbeEvent> {
    let event_type = raw.get("event_type")?.as_str()?.to_string();

    if event_type == "stats" {
        return None;
    }

    if !matches!(
        event_type.as_str(),
        "dns" | "http" | "tls" | "flow" | "alert"
    ) {
        return None;
    }

    let payload = match event_type.as_str() {
        "dns" => ProbePayload::Dns(normalize_dns(raw)),
        "http" => ProbePayload::Http(normalize_http(raw)),
        "tls" => ProbePayload::Tls(normalize_tls(raw)),
        "flow" => ProbePayload::Flow(normalize_flow(raw)),
        "alert" => ProbePayload::Alert(normalize_alert(raw)),
        _ => ProbePayload::Unknown,
    };

    let app_proto = get_string(raw, "app_proto").or_else(|| match event_type.as_str() {
        "dns" | "http" | "tls" => Some(event_type.clone()),
        _ => None,
    });

    Some(ProbeEvent {
        timestamp: get_string(raw, "timestamp").unwrap_or_else(now_fallback),
        probe: ProbeInfo {
            probe_id: probe_id.to_string(),
            sensor_name: sensor_name.to_string(),
            host: host.clone(),
        },
        in_iface: get_string(raw, "in_iface"),
        flow_id: get_u64(raw, "flow_id"),
        event_type,
        src_ip: get_string(raw, "src_ip"),
        src_port: get_u16(raw, "src_port"),
        dest_ip: get_string(raw, "dest_ip"),
        dest_port: get_u16(raw, "dest_port"),
        proto: get_string(raw, "proto"),
        app_proto,
        payload,
    })
}

fn normalize_dns(raw: &Value) -> DnsPayload {
    let dns = raw.get("dns");

    let query = dns
        .and_then(|d| d.get("queries"))
        .and_then(|q| q.as_array())
        .and_then(|arr| arr.first())
        .and_then(|q| q.get("rrname"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let query_type = dns
        .and_then(|d| d.get("queries"))
        .and_then(|q| q.as_array())
        .and_then(|arr| arr.first())
        .and_then(|q| q.get("rrtype"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let answers = dns
        .and_then(|d| d.get("answers"))
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|ans| ans.get("rdata").and_then(|v| v.as_str()))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    DnsPayload {
        dns_type: dns
            .and_then(|d| d.get("type"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        rcode: dns
            .and_then(|d| d.get("rcode"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        query,
        query_type,
        answers,
    }
}

fn normalize_http(raw: &Value) -> HttpPayload {
    let http = raw.get("http");

    HttpPayload {
        hostname: http
            .and_then(|h| h.get("hostname"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        url: http
            .and_then(|h| h.get("url"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        http_method: http
            .and_then(|h| h.get("http_method"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        protocol: http
            .and_then(|h| h.get("protocol"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        status: http
            .and_then(|h| h.get("status"))
            .and_then(|v| v.as_u64())
            .and_then(|n| u16::try_from(n).ok()),
        length: http
            .and_then(|h| h.get("length"))
            .and_then(|v| v.as_u64()),
    }
}

fn normalize_tls(raw: &Value) -> TlsPayload {
    let tls = raw.get("tls");

    TlsPayload {
        subject: tls
            .and_then(|t| t.get("subject"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        issuerdn: tls
            .and_then(|t| t.get("issuerdn"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        sni: tls
            .and_then(|t| t.get("sni"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        version: tls
            .and_then(|t| t.get("version"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        notbefore: tls
            .and_then(|t| t.get("notbefore"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        notafter: tls
            .and_then(|t| t.get("notafter"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
    }
}

fn normalize_flow(raw: &Value) -> FlowPayload {
    let flow = raw.get("flow");

    FlowPayload {
        state: flow
            .and_then(|f| f.get("state"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        reason: flow
            .and_then(|f| f.get("reason"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        bytes_toclient: flow
            .and_then(|f| f.get("bytes_toclient"))
            .and_then(|v| v.as_u64()),
        bytes_toserver: flow
            .and_then(|f| f.get("bytes_toserver"))
            .and_then(|v| v.as_u64()),
        pkts_toclient: flow
            .and_then(|f| f.get("pkts_toclient"))
            .and_then(|v| v.as_u64()),
        pkts_toserver: flow
            .and_then(|f| f.get("pkts_toserver"))
            .and_then(|v| v.as_u64()),
    }
}

fn normalize_alert(raw: &Value) -> AlertPayload {
    let alert = raw.get("alert");

    AlertPayload {
        signature_id: alert
            .and_then(|a| a.get("signature_id"))
            .and_then(|v| v.as_u64()),
        signature: alert
            .and_then(|a| a.get("signature"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        category: alert
            .and_then(|a| a.get("category"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        severity: alert
            .and_then(|a| a.get("severity"))
            .and_then(|v| v.as_u64())
            .and_then(|n| u8::try_from(n).ok()),
    }
}

fn get_string(raw: &Value, key: &str) -> Option<String> {
    raw.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn get_u64(raw: &Value, key: &str) -> Option<u64> {
    raw.get(key).and_then(|v| v.as_u64())
}

fn get_u16(raw: &Value, key: &str) -> Option<u16> {
    raw.get(key)
        .and_then(|v| v.as_u64())
        .and_then(|n| u16::try_from(n).ok())
}

fn now_fallback() -> String {
    chrono::Utc::now().to_rfc3339()
}