mod scanner_xml;
use crate::models::{s, DiscoveryRecord, ObservedService, Parsed, Result};
use serde_json::{json, Value};
use std::net::Ipv4Addr;
pub const MAX_IMPORT: usize = 32 * 1024 * 1024;
pub trait Adapter {
    fn parse(&self, text: &str) -> Result<Parsed>;
}
pub fn parse(tool: &str, text: &str) -> Result<Parsed> {
    if text.len() > MAX_IMPORT {
        return Err(
            crate::errors::ErrorCode::ImportTooLarge.message("Import exceeds the 32 MiB limit")
        );
    }
    if text.trim().is_empty() {
        return Err("The import file is empty".into());
    }
    let p = match tool {
        "Nessus" => scanner_xml::nessus(text),
        "Burp" => scanner_xml::burp(text),
        "Nmap" => Nmap.parse(text),
        "httpx" => JsonAdapter(false).parse(text),
        "Nuclei" => JsonAdapter(true).parse(text),
        "NetExec" => NetExec.parse(text),
        _ => Err("Unsupported import adapter".into()),
    }?;
    if p.hosts.is_empty() {
        return Err("No reliable host observations found. Check the selected format.".into());
    }
    Ok(p)
}
struct Nmap;
impl Adapter for Nmap {
    fn parse(&self, text: &str) -> Result<Parsed> {
        // Live Nmap emits this inert declaration. Accept only that exact form;
        // external identifiers, internal subsets and entities remain rejected.
        let text = text.replace("<!DOCTYPE nmaprun>", "");
        if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
            return Err("XML DTDs and entity declarations are not accepted".into());
        }
        let doc =
            roxmltree::Document::parse(&text).map_err(|e| format!("Malformed Nmap XML: {e}"))?;
        if !doc.root_element().has_tag_name("nmaprun") {
            return Err("Expected an nmaprun XML document".into());
        }
        let mut out = Parsed::default();
        for node in doc.descendants().filter(|n| n.has_tag_name("host")) {
            let mut h = DiscoveryRecord::default();
            for a in node.children().filter(|n| n.has_tag_name("address")) {
                if a.attribute("addrtype") == Some("ipv4") {
                    h.ip = a.attribute("addr").unwrap_or("").to_string()
                }
            }
            if h.ip.parse::<Ipv4Addr>().is_err() {
                out.warnings
                    .push("Skipped a host without a valid IPv4 address".into());
                continue;
            }
            h.hostname = node
                .descendants()
                .find(|n| n.has_tag_name("hostname"))
                .and_then(|n| n.attribute("name"))
                .unwrap_or("")
                .to_ascii_lowercase();
            h.os = node
                .descendants()
                .find(|n| n.has_tag_name("osmatch"))
                .and_then(|n| n.attribute("name"))
                .unwrap_or("")
                .into();
            for p in node.descendants().filter(|n| n.has_tag_name("port")) {
                let port = p
                    .attribute("portid")
                    .unwrap_or("")
                    .parse::<u16>()
                    .map_err(|_| "Invalid Nmap port")?;
                if port == 0 {
                    return Err("Port zero is invalid".into());
                }
                let svc = p.children().find(|n| n.has_tag_name("service"));
                let attr = |k| svc.and_then(|n| n.attribute(k)).unwrap_or("").to_string();
                h.services.push(ObservedService {
                    port,
                    protocol: p.attribute("protocol").unwrap_or("tcp").into(),
                    name: attr("name"),
                    product: attr("product"),
                    version: attr("version"),
                    status: p
                        .children()
                        .find(|n| n.has_tag_name("state"))
                        .and_then(|n| n.attribute("state"))
                        .unwrap_or("unknown")
                        .into(),
                    tls: attr("tunnel") == "ssl",
                    ..Default::default()
                });
            }
            out.hosts.push(h);
        }
        Ok(out)
    }
}
struct JsonAdapter(bool);
impl Adapter for JsonAdapter {
    fn parse(&self, text: &str) -> Result<Parsed> {
        let trimmed = text.trim();
        let rows: Vec<Value> = if trimmed.starts_with('[') {
            serde_json::from_str(trimmed).map_err(|e| format!("Invalid JSON array: {e}"))?
        } else if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            vec![v]
        } else {
            trimmed
                .lines()
                .enumerate()
                .filter(|(_, l)| !l.trim().is_empty())
                .map(|(i, l)| {
                    serde_json::from_str(l)
                        .map_err(|e| format!("Invalid JSON on line {}: {e}", i + 1))
                })
                .collect::<Result<_>>()?
        };
        let mut out = Parsed::default();
        for row in rows {
            let raw = if self.0 {
                s(&row, "matched-at")
            } else {
                s(&row, "url")
            };
            let raw = if raw.is_empty() { s(&row, "host") } else { raw };
            let parsed = url::Url::parse(raw)
                .or_else(|_| url::Url::parse(&format!("http://{raw}")))
                .map_err(|_| "Invalid URL in observation")?;
            let host = parsed
                .host_str()
                .ok_or("Missing host in observation")?
                .to_ascii_lowercase();
            let ip = [s(&row, "ip"), s(&row, "host"), host.as_str()]
                .into_iter()
                .find(|s| s.parse::<Ipv4Addr>().is_ok())
                .unwrap_or("")
                .to_string();
            let ip = if ip.is_empty() {
                row.get("a")
                    .and_then(Value::as_array)
                    .and_then(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .find(|s| s.parse::<Ipv4Addr>().is_ok())
                    })
                    .unwrap_or("")
                    .into()
            } else {
                ip
            };
            if ip.is_empty() && crate::scope::validate(&host).is_err() {
                return Err("Invalid target host".into());
            }
            let mut h = DiscoveryRecord {
                ip,
                hostname: if host.parse::<Ipv4Addr>().is_err() {
                    host
                } else {
                    String::new()
                },
                ..Default::default()
            };
            let port = parsed.port_or_known_default().ok_or("Missing URL port")?;
            h.services.push(ObservedService{port,protocol:"tcp".into(),name:parsed.scheme().into(),url:parsed.to_string(),title:s(&row,"title").into(),product:row.get("tech").map(|v|v.to_string()).unwrap_or_default(),tls:parsed.scheme()=="https",status:"open".into(),banner:if self.0{String::new()}else{json!({"status_code":row.get("status_code"),"redirect":row.get("location"),"tls":row.get("tls")}).to_string()},..Default::default()});
            if self.0 {
                let info = row.get("info").unwrap_or(&Value::Null);
                let severity = s(info, "severity");
                h.findings.push(json!({"name":s(info,"name"),"severity":match severity {"critical"=>"Critical","high"=>"High","medium"=>"Medium","low"=>"Low",_=>"Info"},"description":s(info,"description"),"template_id":s(&row,"template-id"),"refs":info.get("reference").map(Value::to_string).unwrap_or_default(),"reproduction":format!("Matched: {}\nMatcher: {}\nExtracted: {}",raw,s(&row,"matcher-name"),row.get("extracted-results").unwrap_or(&Value::Null)),"raw":row}));
            }
            out.hosts.push(h);
        }
        Ok(out)
    }
}
struct NetExec;
impl Adapter for NetExec {
    fn parse(&self, text: &str) -> Result<Parsed> {
        let ansi = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        let text = ansi.replace_all(text, "");
        let re=regex::Regex::new(r"^(SMB|WINRM|SSH|RDP|LDAP|MSSQL)\s+(\d+\.\d+\.\d+\.\d+)\s+(\d+)\s+(\S+)\s+\[([+*\-])\]\s+(.*)$").unwrap();
        let mut out = Parsed::default();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let Some(c) = re.captures(line.trim()) else {
                out.warnings
                    .push("Skipped an unrecognized NetExec line".into());
                continue;
            };
            if c[2].parse::<Ipv4Addr>().is_err() {
                return Err("Invalid NetExec IPv4 address".into());
            }
            let port = c[3].parse::<u16>().map_err(|_| "Invalid NetExec port")?;
            let mut h = DiscoveryRecord {
                ip: c[2].into(),
                hostname: if c[4] == *"NONE" {
                    String::new()
                } else {
                    c[4].to_ascii_lowercase()
                },
                ..Default::default()
            };
            h.services.push(ObservedService {
                port,
                protocol: "tcp".into(),
                name: c[1].to_ascii_lowercase(),
                status: "open".into(),
                ..Default::default()
            });
            if c[5] == *"+" || c[5] == *"-" {
                let message = &c[6];
                if let Some((identity, secret)) = message.split_once(':') {
                    let (domain, user) = identity.split_once('\\').unwrap_or(("", identity));
                    if !user.is_empty() && !user.contains(' ') {
                        h.auth.push(json!({"username":user,"context":domain,"secret":secret.split(" (Pwn3d!)").next().unwrap_or(secret).trim(),"result":if c[5]==*"+"{"Valid"}else{"Invalid"},"privilege":if message.contains("(Pwn3d!)"){"Administrator"}else{"Unknown"}}));
                    } else {
                        out.warnings
                            .push("Authentication identity was ambiguous; not recorded".into())
                    }
                } else {
                    out.warnings
                        .push("Authentication line had no reliable identity; not recorded".into())
                }
            }
            out.hosts.push(h);
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixtures_parse() {
        for (tool, text) in [
            ("Nmap", include_str!("../../../fixtures/small_lab/nmap.xml")),
            (
                "httpx",
                include_str!("../../../fixtures/small_lab/httpx.jsonl"),
            ),
            (
                "Nuclei",
                include_str!("../../../fixtures/small_lab/nuclei.jsonl"),
            ),
            (
                "NetExec",
                include_str!("../../../fixtures/internal_ad_lab/netexec.txt"),
            ),
        ] {
            assert!(!parse(tool, text).unwrap().hosts.is_empty(), "{tool}")
        }
    }
    #[test]
    fn malformed_rejected() {
        for (tool, text) in [
            ("Nmap", "<nmaprun>"),
            ("Nmap", "<!DOCTYPE x><nmaprun/>"),
            ("httpx", "{oops}"),
            ("Nuclei", "{}"),
            ("NetExec", "not a reliable observation"),
        ] {
            assert!(parse(tool, text).is_err())
        }
    }
}
