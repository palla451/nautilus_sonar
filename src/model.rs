use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub hostname: String,
    pub ip: String,
    pub os: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeInfo {
    pub probe_id: String,
    pub sensor_name: String,
    pub version: String,
    pub host: HostInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeEvent {
    pub timestamp: String,
    pub probe: ProbeInfo,

    #[serde(default)]
    pub in_iface: Option<String>,

    #[serde(default)]
    pub flow_id: Option<u64>,

    pub event_type: String,

    #[serde(default)]
    pub src_ip: Option<String>,

    #[serde(default)]
    pub src_port: Option<u16>,

    #[serde(default)]
    pub dest_ip: Option<String>,

    #[serde(default)]
    pub dest_port: Option<u16>,

    #[serde(default)]
    pub proto: Option<String>,

    #[serde(default)]
    pub app_proto: Option<String>,

    pub payload: ProbePayload,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_event: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProbePayload {
    Dns(DnsPayload),
    Http(HttpPayload),
    Tls(TlsPayload),
    Flow(FlowPayload),
    Alert(AlertPayload),
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsPayload {
    #[serde(default)]
    pub dns_type: Option<String>,
    #[serde(default)]
    pub rcode: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub query_type: Option<String>,
    #[serde(default)]
    pub answers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpPayload {
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub http_method: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub status: Option<u16>,
    #[serde(default)]
    pub length: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsPayload {
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub issuerdn: Option<String>,
    #[serde(default)]
    pub sni: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub notbefore: Option<String>,
    #[serde(default)]
    pub notafter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPayload {
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub bytes_toclient: Option<u64>,
    #[serde(default)]
    pub bytes_toserver: Option<u64>,
    #[serde(default)]
    pub pkts_toclient: Option<u64>,
    #[serde(default)]
    pub pkts_toserver: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertPayload {
    #[serde(default)]
    pub signature_id: Option<u64>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub severity: Option<u8>,
}