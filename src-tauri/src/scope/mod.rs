use crate::models::Result;
use ipnet::Ipv4Net;
use std::net::Ipv4Addr;
pub fn validate(rule: &str) -> Result<()> {
    if rule.parse::<Ipv4Addr>().is_ok() || rule.parse::<Ipv4Net>().is_ok() {
        return Ok(());
    }
    let host = rule.strip_prefix("*.").unwrap_or(rule);
    if host.len() <= 253
        && !host.is_empty()
        && host.split('.').all(|p| {
            !p.is_empty()
                && p.len() <= 63
                && !p.starts_with('-')
                && !p.ends_with('-')
                && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        && host.bytes().any(|b| b.is_ascii_alphabetic())
    {
        Ok(())
    } else {
        Err("Enter an IPv4 address, valid CIDR, hostname, or *.domain suffix".into())
    }
}
pub fn matches(rule: &str, target: &str) -> bool {
    let rule = rule.to_ascii_lowercase();
    let target = target.trim_end_matches('.').to_ascii_lowercase();
    if let Ok(net) = rule.parse::<Ipv4Net>() {
        return target
            .parse::<Ipv4Addr>()
            .map(|ip| net.contains(&ip))
            .unwrap_or(false);
    }
    if let Some(domain) = rule.strip_prefix("*.") {
        return target.ends_with(&format!(".{domain}")) && target != domain;
    }
    rule == target
}
pub fn allowed(rules: &[(String, bool)], target: &str) -> bool {
    !rules.iter().any(|(r, e)| *e && matches(r, target))
        && rules.iter().any(|(r, e)| !e && matches(r, target))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cidr_boundaries() {
        assert!(matches("10.1.0.0/24", "10.1.0.255"));
        assert!(!matches("10.1.0.0/24", "10.1.1.0"));
        assert!(validate("10.1.0.0/33").is_err());
    }
    #[test]
    fn exclusions_win() {
        let r = vec![("10.0.0.0/8".into(), false), ("10.1.1.1".into(), true)];
        assert!(!allowed(&r, "10.1.1.1"));
        assert!(allowed(&r, "10.2.1.1"));
        assert!(!allowed(&[], "10.1.1.1"));
    }
    #[test]
    fn domain_boundaries() {
        assert!(matches("*.corp.test", "WEB.corp.test"));
        assert!(!matches("*.corp.test", "evilcorp.test"));
        assert!(!matches("*.corp.test", "corp.test"));
        assert!(validate("-bad.test").is_err());
    }
}
