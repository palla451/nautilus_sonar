use std::env;

pub struct Config {
    pub eve_path: String,
    pub sensor_name: String,
    pub interface: String,
    pub nmea_enabled: bool,
    pub nmea_multicast_ip: String,
    pub nmea_port: u16,
    pub backend_enabled: bool,
    pub backend_url: String,
    pub bearer_token: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            eve_path: env::var("SURICATA_EVE_FILE")
                .unwrap_or_else(|_| "/var/log/suricata/eve.json".to_string()),

            sensor_name: env::var("SENSOR_NAME")
                .unwrap_or_else(|_| "nautilus-sonar".to_string()),

            interface: env::var("INTERFACE")
                .unwrap_or_else(|_| "enp0s5".to_string()),

            nmea_enabled: env::var("NMEA_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),

            nmea_multicast_ip: env::var("NMEA_MULTICAST_IP")
                .unwrap_or_else(|_| "239.0.1.1".to_string()),

            nmea_port: env::var("NMEA_PORT")
                .unwrap_or_else(|_| "10110".to_string())
                .parse()
                .unwrap_or(10110),

            backend_enabled: env::var("BACKEND_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),

            backend_url: env::var("BACKEND_URL")
                .unwrap_or_else(|_| "http://backend.local/api/events".to_string()),

            bearer_token: env::var("BEARER_TOKEN")
                .unwrap_or_else(|_| "test-token".to_string()),
        }
    }
}