pub mod provenance;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub type Result<T> = std::result::Result<T, String>;
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}
pub fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    let value = s(v, key).trim();
    if value.is_empty() {
        Err(format!("{key} is required"))
    } else if value.len() > 100_000 {
        Err(format!("{key} is too long"))
    } else {
        Ok(value)
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryRecord {
    pub ip: String,
    pub hostname: String,
    pub os: String,
    pub services: Vec<ObservedService>,
    pub findings: Vec<Value>,
    pub auth: Vec<Value>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservedService {
    pub port: u16,
    pub protocol: String,
    pub name: String,
    pub product: String,
    pub version: String,
    pub status: String,
    pub url: String,
    pub title: String,
    pub tls: bool,
    pub banner: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Parsed {
    pub hosts: Vec<DiscoveryRecord>,
    pub warnings: Vec<String>,
}
