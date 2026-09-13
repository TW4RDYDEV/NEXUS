use nexus_core::core::Api;
use serde_json::json;
use std::time::Instant;
#[test]
fn ten_thousand_hosts_fifty_thousand_services_and_one_hundred_thousand_edges() {
    let directory = tempfile::tempdir().unwrap();
    let mut api = Api::new(directory.path().into()).unwrap();
    api.dispatch(
        "create",
        json!({"name":"Generated scale fixture","kind":"Lab / CTF"}),
    )
    .unwrap();
    let store = api.store.as_mut().unwrap();
    let begin = Instant::now();
    store.conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    {
        let mut host = store
            .conn
            .prepare(
                "INSERT INTO assets(id,name,ip,hostname,first_seen,last_seen) VALUES(?,?,?,?,?,?)",
            )
            .unwrap();
        let mut service=store.conn.prepare("INSERT INTO services(id,asset_id,name,port,first_seen,last_seen) VALUES(?,?,?,?,?,?)").unwrap();
        let mut edge=store.conn.prepare("INSERT INTO relationships(id,source_id,target_id,kind,created_at) VALUES(?,?,?,'NX_CONNECTS_TO',?)").unwrap();
        for i in 0..10000 {
            let aid = format!("host-{i:05}");
            host.execute(rusqlite::params![
                aid,
                format!("LAB-{i:05}"),
                format!("10.0.{}.{}", i / 256, i % 256),
                format!("lab-{i:05}.example.test"),
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z"
            ])
            .unwrap();
            for (port, name) in [
                (22, "ssh"),
                (80, "http"),
                (443, "https"),
                (445, "smb"),
                (8080, "http"),
            ] {
                service
                    .execute(rusqlite::params![
                        format!("{aid}-{port}"),
                        aid,
                        name,
                        port,
                        "2026-01-01T00:00:00Z",
                        "2026-01-01T00:00:00Z"
                    ])
                    .unwrap();
            }
        }
        for i in 0..100000 {
            edge.execute(rusqlite::params![
                format!("edge-{i}"),
                format!("host-{:05}", i % 10000),
                format!("host-{:05}", (i % 10000 + i / 10000 + 1) % 10000),
                "2026-01-01T00:00:00Z"
            ])
            .unwrap();
        }
    }
    store.conn.execute_batch("COMMIT").unwrap();
    let seeded = begin.elapsed();
    let start = Instant::now();
    let page = api
        .dispatch("list", json!({"table":"assets","offset":9950,"limit":50}))
        .unwrap();
    assert_eq!(page["total"], 10000);
    assert_eq!(page["rows"].as_array().unwrap().len(), 50);
    let list_time = start.elapsed();
    let start = Instant::now();
    let coverage = api.dispatch("coverage", json!({"limit":100})).unwrap();
    assert_eq!(coverage["rows"].as_array().unwrap().len(), 100);
    assert!(coverage["total"].as_i64().unwrap() > 200000);
    let coverage_time = start.elapsed();
    let start = Instant::now();
    let search = api
        .dispatch("search", json!({"q":"lab-09999.example.test"}))
        .unwrap();
    assert_eq!(search.as_array().unwrap().len(), 1);
    let search_time = start.elapsed();
    let start = Instant::now();
    let graph = api.dispatch("graph", json!({})).unwrap();
    assert!(graph["nodes"].as_array().unwrap().len() <= 500);
    let graph_time = start.elapsed();
    eprintln!("SCALE BENCHMARK seed={seeded:?} list={list_time:?} coverage={coverage_time:?} search={search_time:?} graph={graph_time:?}");
    assert!(list_time.as_secs() < 10);
    assert!(graph_time.as_secs() < 30);
}
