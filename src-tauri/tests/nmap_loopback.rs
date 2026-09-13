//! Explicit integration test. Only 127.0.0.1 and a port owned by this test are scanned.
use nexus_core::core::Api;
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
#[test]
#[ignore = "requires an installed Nmap; run explicitly with --test nmap_loopback -- --ignored --nocapture"]
fn real_nmap_scan_is_reviewable_and_imports_an_owned_loopback_service() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    let server = std::thread::spawn(move || {
        while !signal.load(Ordering::Relaxed) {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
                let mut data = [0; 4096];
                let _ = stream.read(&mut data);
                let _=stream.write_all(b"HTTP/1.1 200 OK\r\nServer: NexusLoopbackTest/1.0\r\nContent-Type: text/plain\r\nContent-Length: 12\r\nConnection: close\r\n\r\nNEXUS test\r\n");
            } else {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    });
    let result = std::panic::catch_unwind(|| {
        let root = tempfile::tempdir().unwrap();
        let mut api = Api::new(root.path().into()).unwrap();
        api.dispatch(
            "create",
            json!({"name":"Local Nmap integration test","kind":"Lab / CTF"}),
        )
        .unwrap();
        api.dispatch(
            "write",
            json!({"table":"scope_rules","data":{"rule":"127.0.0.1","excluded":0}}),
        )
        .unwrap();
        let plan = api
            .dispatch(
                "runner_plan",
                json!({"target":"127.0.0.1","ports":port.to_string()}),
            )
            .unwrap();
        assert_eq!(plan["resolved"], "127.0.0.1");
        api.dispatch(
            "runner_start",
            json!({"target":"127.0.0.1","ports":port.to_string()}),
        )
        .unwrap();
        let start = Instant::now();
        let status = loop {
            let status = api.dispatch("runner_status", json!({})).unwrap();
            if status["done"] == true {
                break status;
            }
            if start.elapsed() > Duration::from_secs(50) {
                let _ = api.dispatch("runner_status", json!({"cancel":true}));
                panic!("Nmap did not finish in 50 seconds");
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        assert_eq!(status["success"], true, "Nmap stderr: {}", status["stderr"]);
        let text = std::fs::read_to_string(status["path"].as_str().unwrap()).unwrap();
        let preview = api
            .dispatch("preview", json!({"tool":"Nmap","text":text}))
            .unwrap();
        assert_eq!(preview["hosts"], 1);
        api.dispatch(
            "import",
            json!({"tool":"Nmap","text":text,"name":"verified-loopback.xml"}),
        )
        .unwrap();
        let services = api.store.as_ref().unwrap().all("services").unwrap();
        assert!(services
            .iter()
            .any(|s| s["port"] == port && s["status"] == "open"));
        eprintln!("LIVE NMAP PASS: owned loopback TCP port {port}, elapsed {:?}, open service imported; review/commit workflow verified",start.elapsed());
    });
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
