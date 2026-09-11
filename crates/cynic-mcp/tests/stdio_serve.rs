//! Live stdio proof: spawn the built `cynic-mcp` binary, run a real MCP
//! handshake over its stdin/stdout, and assert the slice-1 tools answer.
//! No mocks, no canned responses — the binary executes.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

/// Run `commands` (one JSON-RPC message per line) through a fresh server
/// instance and collect exactly `expect_responses` reply lines. A watchdog
/// kills the child so a wedged server fails the test instead of hanging it.
fn handshake(commands: &[&str], expect_responses: usize) -> Vec<String> {
    let path = env!("CARGO_BIN_EXE_cynic-mcp");
    let mut spawned = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cynic-mcp");
    let mut stdin = spawned.stdin.take().expect("stdin");
    let stdout = spawned.stdout.take().expect("stdout");
    let child = Arc::new(Mutex::new(spawned));
    let done = Arc::new(AtomicBool::new(false));
    std::thread::scope(|scope| {
        let killer = Arc::clone(&child);
        let done_watch = Arc::clone(&done);
        scope.spawn(move || {
            for _ in 0..200 {
                if done_watch.load(Ordering::SeqCst) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            if let Ok(mut guard) = killer.lock() {
                let _ = guard.kill();
            }
        });
        for command in commands {
            writeln!(stdin, "{command}").expect("write");
        }
        drop(stdin);
        let lines: Vec<String> = BufReader::new(stdout)
            .lines()
            .take(expect_responses)
            .map(|line| line.expect("read"))
            .collect();
        // MCP servers have no goodbye: EOF does not reliably stop the
        // transport loop, so reap explicitly once the evidence is read.
        if let Ok(mut guard) = child.lock() {
            let _ = guard.kill();
            let _ = guard.wait();
        }
        done.store(true, Ordering::SeqCst);
        lines
    })
}

#[test]
fn stdio_serve_lists_slice1_tools() {
    let lines = handshake(
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        ],
        2,
    );
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"protocolVersion\":\"2026-07-28\""));
    assert!(lines[1].contains("cynic_mcp_ping"));
    assert!(lines[1].contains("cynic_protocol_versions"));
    assert!(lines[1].contains("cynic_workspace_inventory"));
}

#[test]
fn stdio_serve_calls_ping_live() {
    let lines = handshake(
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"cynic_mcp_ping"}}"#,
        ],
        2,
    );
    assert_eq!(lines.len(), 2);
    assert!(lines[1].contains("\"server\":\"cynic-mcp\""));
    assert!(lines[1].contains("\"isError\":false"));
}
