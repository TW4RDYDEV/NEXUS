use crate::domain::DomainEvent;
use crate::{
    db::Store,
    models::{id, s, Result},
};
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    net::ToSocketAddrs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Instant,
};
pub struct Job {
    pub child: Child,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub started: Instant,
    pub done: bool,
    pub success: bool,
    pub id: String,
}
impl Drop for Job {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
pub fn executable(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for suffix in if cfg!(windows) {
            vec![".exe", ""]
        } else {
            vec![""]
        } {
            let p = dir.join(format!("{name}{suffix}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    if name == "nmap" && cfg!(windows) {
        for path in [
            r"C:\Program Files (x86)\Nmap\nmap.exe",
            r"C:\Program Files\Nmap\nmap.exe",
        ] {
            let p = PathBuf::from(path);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}
pub fn integrations() -> Value {
    json!([("Nmap",executable("nmap")),("httpx",executable("httpx")),("Nuclei",executable("nuclei")),("NetExec",executable("netexec").or_else(||executable("nxc")))].iter().map(|(name,path)|json!({"name":name,"installed":path.is_some(),"path":path,"runner":*name=="Nmap"})).collect::<Vec<_>>())
}
pub fn plan(store: &Store, target: &str, ports: &str) -> Result<Value> {
    crate::scope::validate(target)?;
    if target.contains('/') || target.starts_with("*.") {
        return Err("The integrated runner accepts one host at a time".into());
    }
    let rules = store
        .all("scope_rules")?
        .iter()
        .map(|r| (s(r, "rule").to_string(), r["excluded"].as_i64() == Some(1)))
        .collect::<Vec<_>>();
    let ips = if let Ok(ip) = target.parse::<std::net::Ipv4Addr>() {
        vec![ip]
    } else {
        (target, 0)
            .to_socket_addrs()
            .map_err(|_| "Hostname resolution failed")?
            .filter_map(|a| {
                if let std::net::IpAddr::V4(ip) = a.ip() {
                    Some(ip)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };
    if ips.len() != 1 {
        return Err(
            "Resolve to one IPv4 address and use that exact address as the scan target".into(),
        );
    }
    let ip = ips[0].to_string();
    if !crate::scope::allowed(&rules, &ip)
        || rules
            .iter()
            .any(|(r, e)| *e && crate::scope::matches(r, target))
    {
        return Err("Scope Guard blocked execution. The resolved IPv4 target must be explicitly in scope, with no matching exclusion.".into());
    }
    let ports = ports
        .split(',')
        .map(|p| {
            p.trim()
                .parse::<u16>()
                .map_err(|_| "Ports must be comma-separated integers")
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if ports.is_empty() || ports.len() > 100 || ports.contains(&0) {
        return Err("Choose between 1 and 100 valid ports".into());
    }
    let ports = ports
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let args = vec![
        "-sT",
        "--unprivileged",
        "-n",
        "-sV",
        "--version-light",
        "-Pn",
        "--host-timeout",
        "120s",
        "-p",
        &ports,
        "-oX",
        "-",
        "--",
        &ip,
    ];
    let binary = executable("nmap").ok_or("Nmap is not installed or could not be found in PATH")?;
    Ok(json!({"target":target,"resolved":ip,"binary":binary,"args":args}))
}
pub fn start(store: &Store, target: &str, ports: &str) -> Result<Job> {
    let plan = plan(store, target, ports)?;
    let job_id = id();
    let stdout = store
        .path
        .join("imports")
        .join(format!("runner-{job_id}.xml"));
    let stderr = store
        .path
        .join("imports")
        .join(format!("runner-{job_id}.stderr.txt"));
    let args = plan["args"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let mut command = Command::new(s(&plan, "binary"));
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            File::create(&stdout).map_err(|e| e.to_string())?,
        ))
        .stderr(Stdio::from(
            File::create(&stderr).map_err(|e| e.to_string())?,
        ));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let child = command
        .spawn()
        .map_err(|e| format!("Nmap could not start: {e}"))?;
    store.event(
        DomainEvent::ScanStarted,
        Some(&job_id),
        &format!(
            "Nmap started for {} with ports {ports}",
            s(&plan, "resolved")
        ),
        "Nmap",
    )?;
    Ok(Job {
        child,
        stdout,
        stderr,
        started: Instant::now(),
        done: false,
        success: false,
        id: job_id,
    })
}
impl Job {
    pub fn status(&mut self, store: &Store, cancel: bool) -> Result<Value> {
        if !self.done {
            if cancel
                || self.started.elapsed().as_secs() > 180
                || fs::metadata(&self.stdout)
                    .map(|m| m.len() > 32 * 1024 * 1024)
                    .unwrap_or(false)
            {
                self.child.kill().map_err(|e| e.to_string())?;
            }
            if let Some(status) = self.child.try_wait().map_err(|e| e.to_string())? {
                self.done = true;
                self.success = status.success();
                store.event(
                    DomainEvent::ScanFinished,
                    Some(&self.id),
                    if self.success {
                        "Nmap completed; output ready for import review"
                    } else {
                        "Nmap stopped or failed; inspect stderr"
                    },
                    "Nmap",
                )?;
            }
        }
        let read = |p: &PathBuf| {
            fs::read_to_string(p)
                .unwrap_or_default()
                .chars()
                .take(200_000)
                .collect::<String>()
        };
        Ok(
            json!({"id":self.id,"done":self.done,"success":self.success,"stdout":read(&self.stdout),"stderr":read(&self.stderr),"path":self.stdout,"elapsed":self.started.elapsed().as_secs()}),
        )
    }
}
