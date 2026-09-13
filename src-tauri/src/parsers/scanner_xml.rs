//! Passive report adapters. Imported scanner claims remain draft findings.
use super::*;
fn xml(text: &str) -> Result<roxmltree::Document<'_>> {
    // Burp's static DTD is accepted by the XML parser without entity expansion.
    // No external document retrieval is performed. Entity declarations are rejected.
    if text.contains("<!ENTITY") {
        return Err("XML entity declarations are not accepted".into());
    }
    roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .map_err(|e| format!("Malformed scanner XML: {e}"))
}
fn child(node: roxmltree::Node<'_, '_>, tag: &str) -> String {
    node.children()
        .find(|n| n.has_tag_name(tag))
        .and_then(|n| n.text())
        .unwrap_or("")
        .to_string()
}
pub fn nessus(text: &str) -> Result<Parsed> {
    let doc = xml(text)?;
    if !doc.root_element().has_tag_name("NessusClientData_v2") {
        return Err("Expected a Nessus v2 XML report".into());
    }
    let mut out = Parsed::default();
    for node in doc.descendants().filter(|n| n.has_tag_name("ReportHost")) {
        let prop = |key: &str| {
            node.descendants()
                .find(|n| n.has_tag_name("tag") && n.attribute("name") == Some(key))
                .and_then(|n| n.text())
                .unwrap_or("")
                .to_string()
        };
        let address = prop("host-ip");
        let name = node.attribute("name").unwrap_or("");
        let address = if address.is_empty() {
            name.to_string()
        } else {
            address
        };
        let mut host = DiscoveryRecord::default();
        if address.parse::<Ipv4Addr>().is_ok() {
            host.ip = address;
        } else if !name.is_empty() && !name.contains(['/', ':', ' ']) {
            host.hostname = name.to_ascii_lowercase();
        } else {
            out.warnings
                .push("Skipped a Nessus host without a supported address or hostname".into());
            continue;
        }
        let fqdn = prop("host-fqdn");
        if !fqdn.is_empty() {
            host.hostname = fqdn.to_ascii_lowercase();
        }
        host.os = prop("operating-system");
        for item in node.children().filter(|n| n.has_tag_name("ReportItem")) {
            let port = item
                .attribute("port")
                .unwrap_or("0")
                .parse::<u16>()
                .map_err(|_| "Invalid Nessus port")?;
            let protocol = item.attribute("protocol").unwrap_or("tcp");
            if !["tcp", "udp"].contains(&protocol) {
                out.warnings
                    .push("Skipped unsupported Nessus transport".into());
                continue;
            }
            if port > 0
                && !host
                    .services
                    .iter()
                    .any(|s| s.port == port && s.protocol == protocol)
            {
                host.services.push(ObservedService {
                    port,
                    protocol: protocol.into(),
                    name: item.attribute("svc_name").unwrap_or("unknown").into(),
                    status: "open".into(),
                    ..Default::default()
                });
            }
            let severity = match item.attribute("severity").unwrap_or("0") {
                "4" => "Critical",
                "3" => "High",
                "2" => "Medium",
                "1" => "Low",
                _ => "Info",
            };
            let plugin = item
                .attribute("pluginID")
                .ok_or("Nessus item is missing a plugin ID")?;
            host.findings.push(json!({"name":item.attribute("pluginName").unwrap_or("Nessus observation"),"severity":severity,"description":child(item,"description"),"impact":child(item,"synopsis"),"remediation":child(item,"solution"),"refs":child(item,"see_also"),"cve":item.children().filter(|n|n.has_tag_name("cve")).filter_map(|n|n.text()).collect::<Vec<_>>().join(", "),"template_id":format!("nessus:{plugin}"),"reproduction":format!("Observed at {protocol}:{port}\n{}",child(item,"plugin_output")),"observed_port":port,"observed_protocol":protocol,"raw":text[item.range()].to_string()}));
        }
        out.hosts.push(host);
    }
    Ok(out)
}
pub fn burp(text: &str) -> Result<Parsed> {
    let doc = xml(text)?;
    if !doc.root_element().has_tag_name("issues") {
        return Err("Expected a Burp issues XML report".into());
    }
    let mut out = Parsed::default();
    for item in doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("issue"))
    {
        let raw = child(item, "host");
        let url = url::Url::parse(&raw).map_err(|_| "Burp issue has an invalid host URL")?;
        if !["http", "https"].contains(&url.scheme()) {
            return Err("Burp host URL must use HTTP or HTTPS".into());
        }
        let host_name = url.host_str().ok_or("Burp issue has no hostname")?;
        let mut host = DiscoveryRecord::default();
        let ip = item
            .children()
            .find(|n| n.has_tag_name("host"))
            .and_then(|n| n.attribute("ip"))
            .unwrap_or("");
        if ip.parse::<Ipv4Addr>().is_ok() {
            host.ip = ip.into();
        }
        if host_name.parse::<Ipv4Addr>().is_ok() {
            host.ip = host_name.into();
        } else {
            host.hostname = host_name.into();
        }
        let port = url
            .port_or_known_default()
            .ok_or("Unknown Burp service port")?;
        host.services.push(ObservedService {
            port,
            protocol: "tcp".into(),
            name: url.scheme().into(),
            status: "open".into(),
            url: raw.clone(),
            tls: url.scheme() == "https",
            ..Default::default()
        });
        let severity = child(item, "severity");
        let severity = match severity.to_ascii_lowercase().as_str() {
            "high" => "High",
            "medium" => "Medium",
            "low" => "Low",
            _ => "Info",
        };
        let issue_type = child(item, "type");
        if issue_type.is_empty() {
            return Err("Burp issue has no type identifier".into());
        }
        let issue_path = child(item, "path");
        let location = url
            .join(&issue_path)
            .map_err(|_| "Burp issue has an invalid location")?;
        if location.origin() != url.origin() {
            return Err("Burp issue location disagrees with its reported host".into());
        }
        host.findings.push(json!({"name":child(item,"name"),"severity":severity,"description":format!("{}\n{}",child(item,"issueBackground"),child(item,"issueDetail")),"remediation":child(item,"remediationBackground"),"template_id":format!("burp:{issue_type}"),"reproduction":format!("{}\nScanner confidence: {}",location,child(item,"confidence")),"observed_port":port,"observed_protocol":"tcp","raw":text[item.range()].to_string()}));
        out.hosts.push(host);
    }
    Ok(out)
}
