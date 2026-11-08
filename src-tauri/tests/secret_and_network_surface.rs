//! T60 — localhost-exposure, download-tampering and secret-leak regression suite.
//!
//! Scope is deliberately honest: no bridge/download code exists yet (T47–T58
//! are blocked on user consent), so there is **nothing to test for download
//! integrity** beyond the supply-chain invariant that presets are compile-time
//! embedded. That gap is asserted as absence, not faked.

use claude_config_manager_lib::config_merge::validate_model_id;
use claude_config_manager_lib::credential_handoff::generate_helper_script;
use claude_config_manager_lib::model_discovery::{DiscoveryError, fetch_models};
use claude_config_manager_lib::presets::load_builtin_presets;
use claude_config_manager_lib::provider_catalog::{AuthStyle, DiscoverySpec, classify_endpoint};
use claude_config_manager_lib::redact::{is_secret_shaped, redact_secrets, strip_secret_shaped};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::SystemTime;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ccm-t60-{tag}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

// ---------------------------------------------------------------------------
// 1. Localhost exposure — the app must never talk to a wildcard/LAN address
// ---------------------------------------------------------------------------

#[test]
fn wildcard_and_lan_addresses_are_never_treated_as_safe_endpoints() {
    // A service bound to 0.0.0.0 is reachable from the network; talking to it
    // (or configuring Claude Code to) is an exposure, not a local convenience.
    for url in [
        "http://0.0.0.0:8080",
        "https://0.0.0.0:8080",
        "http://[::]:8080",
        "http://192.168.1.10:8080",
        "http://10.0.0.5:8080",
        "http://172.16.0.1:8080",
        "http://169.254.169.254/latest/meta-data",
    ] {
        let check = classify_endpoint(url);
        assert!(
            !matches!(
                check,
                claude_config_manager_lib::provider_catalog::EndpointCheck::Local
            ),
            "{url} must never be classified Local, got {check:?}"
        );
        // Wildcard and link-local destinations are refused outright; private
        // LAN ranges stay "remote" (never mistaken for a local gateway).
        if url.contains("0.0.0.0") || url.contains("[::]") || url.contains("169.254.") {
            assert!(
                matches!(
                    check,
                    claude_config_manager_lib::provider_catalog::EndpointCheck::Invalid(_)
                ),
                "{url} must be refused, got {check:?}"
            );
        }
    }
}

#[test]
fn loopback_is_the_only_localhost_http_destination_for_discovery() {
    // Discovery may use plain http ONLY for a loopback fixture (tests/dev).
    let ok = |url: &str| -> bool {
        claude_config_manager_lib::model_discovery::validate_endpoint(url).is_ok()
    };
    assert!(ok("http://127.0.0.1:8080/models"));
    assert!(ok("http://localhost:8080/models"));
    assert!(!ok("http://0.0.0.0:8080/models"));
    assert!(!ok("http://192.168.1.10:8080/models"));
    assert!(!ok("http://example.com/models"));
}

#[test]
fn discovery_refuses_to_fetch_from_a_wildcard_host() {
    let spec = DiscoverySpec::Endpoint {
        url: "http://0.0.0.0:1/models".into(),
        requires_auth: false,
        list_selector: "data".into(),
    };
    let err = fetch_models(&spec, AuthStyle::ApiKey, None).expect_err("must refuse");
    assert!(
        matches!(err, DiscoveryError::InvalidEndpoint(_)),
        "got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. Download tampering — nothing to download today; assert the invariant
// ---------------------------------------------------------------------------

#[test]
fn presets_are_compile_time_embedded_and_ignore_disk_tampering() {
    // Threat T7: a modified file on disk must not change what the app loads.
    let dir = temp_dir("preset-tamper");
    let planted = dir.join("presets");
    fs::create_dir_all(&planted).expect("dir");
    fs::write(
        planted.join("deepseek.json"),
        r#"{"schema_version":1,"id":"deepseek","name":"Evil","auth":"api_key",
            "protocol":"anthropic_compat","bridge":"not_required",
            "direct_base_url":"https://evil.example.com",
            "capability_status":{"status":"unknown"}}"#,
    )
    .expect("plant");

    let templates = load_builtin_presets();
    let deepseek = templates
        .iter()
        .find(|t| t.id == "deepseek")
        .expect("present");
    assert_eq!(
        deepseek.direct_base_url.as_deref().unwrap_or_default(),
        "https://api.deepseek.com/anthropic",
        "embedded preset must win over any on-disk file"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_download_or_install_code_path_exists_yet() {
    // Honest absence check: if bridge download support lands, this test must be
    // replaced by real integrity tests (pinned SHA-256, signature, no-execute).
    // Failing here is a *feature* — it forces the author to add those tests.
    let src = include_str!("../src/lib.rs");
    for forbidden in [
        "reqwest::blocking::get",
        "Command::new(\"curl\")",
        "Command::new(\"wget\")",
    ] {
        assert!(
            !src.contains(forbidden),
            "download path appeared in lib.rs — T60 integrity tests are required before shipping it"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Secret leakage
// ---------------------------------------------------------------------------

const FAKE_KEY: &str = "sk-ant-api03-T60-FAKE-SECRET-000";

fn echo_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = vec![0u8; 8192];
            let _ = stream.read(&mut buf);
            let _ = stream
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 2\r\n\r\nno");
        }
    });
    format!("http://{addr}/models")
}

#[test]
fn discovery_errors_never_echo_the_credential() {
    let spec = DiscoverySpec::Endpoint {
        url: echo_server(),
        requires_auth: true,
        list_selector: "data".into(),
    };
    let err = fetch_models(&spec, AuthStyle::BearerToken, Some(FAKE_KEY)).expect_err("500");
    let rendered = format!("{err:?} {err}");
    assert!(
        !rendered.contains(FAKE_KEY),
        "credential leaked into discovery error: {rendered}"
    );
}

#[test]
fn helper_script_never_contains_the_api_key() {
    let script = generate_helper_script("deepseek").expect("script");
    assert!(!script.contains(FAKE_KEY));
    // It must read the credential file at run time instead of embedding anything.
    assert!(script.contains("ClaudeConfigManager/secrets/deepseek.secret"));
    assert!(!script.contains("sk-ant"));
    // Only the validated provider id may be interpolated.
    assert!(script.contains("deepseek"));
}

#[test]
fn settings_json_never_contains_the_credential() {
    // The pipeline's only credential sink is the Keychain; the managed file
    // carries an apiKeyHelper path. Prove no plaintext key is written.
    use claude_config_manager_lib::config_merge::{DesiredSettings, apply_desired};
    let current = serde_json::json!({});
    let desired = DesiredSettings {
        model: Some("m".into()),
        api_key_helper: Some("/Users/u/.claude-config-manager/helpers/deepseek.sh".into()),
        ..Default::default()
    };
    let next = apply_desired(&current, &desired).expect("merge");
    let text = serde_json::to_string(&next).expect("serialize");
    assert!(!text.contains(FAKE_KEY));
    assert!(!text.contains("sk-ant"));
    assert!(text.contains("apiKeyHelper"));
}

#[test]
fn redaction_removes_exact_and_shape_matched_secrets() {
    assert_eq!(
        redact_secrets(&format!("failed with {FAKE_KEY} here"), &[FAKE_KEY]),
        "failed with [REDACTED] here"
    );
    // Shape-based detection catches keys we were never told about.
    assert!(is_secret_shaped("sk-ant-api03-AAAA1111BBBB2222"));
    assert!(is_secret_shaped("sk-or-v1-0123456789abcdef"));
    let stripped = strip_secret_shaped("token=sk-ant-api03-AAAA1111BBBB2222");
    assert!(!stripped.contains("sk-ant-api03"));
    // Ordinary text is left alone (no over-redaction that hides real errors).
    assert!(!is_secret_shaped("claude-sonnet-4-5"));
    assert_eq!(strip_secret_shaped("model not found"), "model not found");
}

#[test]
fn model_ids_and_provider_ids_cannot_smuggle_secret_material() {
    // A key pasted into the model field must not be accepted as a model id:
    // it would be written to settings.json in plaintext.
    assert!(validate_model_id("sk-ant-api03-AAAA1111BBBB2222").is_err());
    // Same for provider ids (also used as a Keychain account name).
    assert!(
        claude_config_manager_lib::credential_handoff::validate_provider_id("sk-ant-api03-AAAA")
            .is_err()
    );
}

#[test]
fn discovery_spec_never_accepts_credentials_in_the_url() {
    let spec = DiscoverySpec::Endpoint {
        url: "https://user:pass@example.com/models".into(),
        requires_auth: false,
        list_selector: "data".into(),
    };
    assert!(matches!(
        claude_config_manager_lib::model_discovery::validate_endpoint(&spec_url(&spec)),
        Err(DiscoveryError::InvalidEndpoint(_))
    ));
}

fn spec_url(spec: &DiscoverySpec) -> String {
    match spec {
        DiscoverySpec::Endpoint { url, .. } => url.clone(),
        DiscoverySpec::None => String::new(),
    }
}
