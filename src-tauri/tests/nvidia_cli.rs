//! Optional real Claude Code + Read tool against a local NVIDIA fixture.
//! No production settings, keys, or paid endpoints. Explicitly run with --ignored.
use claude_config_manager_lib::{
    credential_vault::CredentialVault,
    file_secret_store::FileSecretStore,
    helper, nvidia_setup, routing_config,
    save_apply::{RoleOverrides, SaveApplyService},
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
struct Fixture {
    home: PathBuf,
    quit: Arc<AtomicBool>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.quit.store(true, Ordering::SeqCst);
        let _ = helper::stop(&self.home);
        let _ = std::fs::remove_dir_all(&self.home);
    }
}
#[test]
#[ignore = "requires installed Claude Code; disposable home and loopback fixtures only"]
fn save_nvidia_then_real_cli_reads_a_file_and_finishes_through_key_failover() {
    let home = std::env::temp_dir().join(format!("ccm nvidia cli 'quoted' {}", std::process::id()));
    std::fs::create_dir_all(home.join(".claude")).unwrap();
    let quit = Arc::new(AtomicBool::new(false));
    let fixture = Fixture {
        home: home.clone(),
        quit: quit.clone(),
    };
    let file = home.join("fixture.txt");
    std::fs::write(&file, "LOCAL_READ_PROOF_4182").unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let seen = Arc::new(Mutex::new(Vec::<Value>::new()));
    let seen_c = seen.clone();
    let server = std::thread::spawn(move || {
        let mut successful = 0;
        while !quit.load(Ordering::SeqCst) {
            let (mut sock, _) = match listener.accept() {
                Ok(s) => s,
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
            };
            sock.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut reader = BufReader::new(sock.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let path = line.split_whitespace().nth(1).unwrap_or("").to_owned();
            let mut length = 0;
            let mut auth = String::new();
            loop {
                let mut h = String::new();
                reader.read_line(&mut h).unwrap();
                if h.trim().is_empty() {
                    break;
                }
                if let Some((k, v)) = h.split_once(':') {
                    if k.eq_ignore_ascii_case("content-length") {
                        length = v.trim().parse::<usize>().unwrap();
                    }
                    if k.eq_ignore_ascii_case("authorization") {
                        auth = v.trim().to_owned();
                    }
                }
            }
            let mut raw = vec![0; length];
            reader.read_exact(&mut raw).unwrap();
            let body: Value = serde_json::from_slice(&raw).unwrap();
            seen_c
                .lock()
                .unwrap()
                .push(json!({"path":path,"body":body,"firstKey":auth=="Bearer fixture-first"}));
            if auth == "Bearer fixture-first" {
                sock.write_all(b"HTTP/1.1 401 Unauthorized\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}").unwrap();
                continue;
            }
            let (delta, finish) = if successful == 0 {
                let tools = body["tools"].as_array().expect("CLI sends tools");
                let name = tools
                    .iter()
                    .find_map(|t| {
                        t["function"]["name"]
                            .as_str()
                            .filter(|n| n.ends_with("Read"))
                    })
                    .expect("Read tool exists");
                (
                    json!({"tool_calls":[{"index":0,"id":"call_local_read","type":"function","function":{"name":name,"arguments":json!({"file_path":file}).to_string()}}]}),
                    "tool_calls",
                )
            } else {
                (json!({"content":"NVIDIA_TOOL_LOOPBACK_OK"}), "stop")
            };
            successful += 1;
            let chunks = [
                json!({"id":"chat_fixture","model":"nvidia/nemotron-3-ultra-550b-a55b","choices":[{"index":0,"delta":delta,"finish_reason":null}]}),
                json!({"choices":[{"index":0,"delta":{},"finish_reason":finish}],"usage":{"prompt_tokens":20,"completion_tokens":10}}),
            ];
            let mut wire = chunks
                .into_iter()
                .map(|v| format!("data: {v}\n\n"))
                .collect::<String>();
            wire.push_str("data: [DONE]\n\n");
            write!(sock,"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",wire.len(),wire).unwrap();
        }
    });
    let vault = CredentialVault::for_home(&home);
    vault.add("nvidia-nim", "first", "fixture-first").unwrap();
    vault.add("nvidia-nim", "second", "fixture-second").unwrap();
    let mut svc =
        SaveApplyService::for_production(&home, Arc::new(FileSecretStore::for_home(&home)));
    svc.config_dir_override = None;
    let binary = std::env::var_os("CCM_HELPER_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_claude-config-manager")));
    let outcome = nvidia_setup::apply(
        &svc,
        "nvidia/nemotron-3-ultra-550b-a55b",
        None,
        &RoleOverrides::default(),
        None,
        false,
        || helper::start_with_exe(&home, &binary).map(|s| s.port),
    )
    .unwrap();
    // Replace only fixture policy destination before any request. Save's generated
    // settings, helper script, credentials and token are used without alteration.
    let mut policy = routing_config::load(&home).unwrap();
    policy.targets[0].base_url = format!("http://127.0.0.1:{port}");
    routing_config::save(&home, &policy).unwrap();
    let mut cli = Command::new("claude");
    cli.args([
        "--bare",
        "-p",
        "Use Read to read fixture.txt, then respond with NVIDIA_TOOL_LOOPBACK_OK",
        "--model",
        "claude-sonnet-4-5",
        "--tools",
        "Read",
        "--allowedTools",
        "Read",
        "--no-session-persistence",
        "--strict-mcp-config",
        "--setting-sources",
        "",
        "--settings",
        &outcome.settings_path,
    ]);
    cli.current_dir(&home)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .env("HOME", &home)
        .env("CLAUDE_CONFIG_DIR", home.join(".claude"))
        .env("TMPDIR", &home)
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("DISABLE_TELEMETRY", "1")
        .env("DISABLE_ERROR_REPORTING", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cli.spawn().expect("installed CLI");
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(45) {
            let _ = child.kill();
            panic!("CLI fixture timed out");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "CLI failed: {stderr}");
    assert!(
        stdout.contains("NVIDIA_TOOL_LOOPBACK_OK"),
        "{stdout}; {stderr}"
    );
    let requests = seen.lock().unwrap();
    assert_eq!(
        requests.len(),
        3,
        "first key 401, second key tool call, second key tool result"
    );
    assert!(requests[0]["firstKey"].as_bool().unwrap());
    assert!(!requests[1]["firstKey"].as_bool().unwrap());
    assert!(!requests[2]["firstKey"].as_bool().unwrap());
    assert!(requests.iter().all(|r| r["path"] == "/v1/chat/completions"));
    assert!(
        requests[2]["body"]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["role"] == "tool"
                && m["content"]
                    .as_str()
                    .is_some_and(|s| s.contains("LOCAL_READ_PROOF_4182")))
    );
    println!(
        "Save -> generated helper -> NVIDIA A1 401 -> A2 tool call -> real CLI Read -> A2 final SSE: PASS"
    );
    drop(requests);
    drop(fixture);
    server.join().unwrap();
}
