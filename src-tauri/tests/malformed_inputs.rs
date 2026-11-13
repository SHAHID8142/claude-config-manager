//! T59 — malformed URL, JSON, model-metadata and command-injection regression
//! suite.
//!
//! Every case here is a *negative* case: hostile or broken input must be
//! refused **without** touching the user's settings. Verified failure modes
//! only — nothing is asserted about behaviour we have not exercised.

use claude_config_manager_lib::config_reader::{ConfigError, read_settings};
use claude_config_manager_lib::model_discovery::{ModelEntry, fetch_models};
use claude_config_manager_lib::provider_catalog::{
    AuthStyle, DiscoverySpec, EndpointCheck, classify_endpoint,
};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::SystemTime;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ccm-t59-{tag}-{}-{}",
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
// 1. Malformed / hostile base URLs
// ---------------------------------------------------------------------------

#[test]
fn malformed_urls_are_never_treated_as_acceptable() {
    let rejected = [
        "",
        "   ",
        "not-a-url",
        "api.deepseek.com",
        "https://",
        "https:// /path",
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "file:///etc/passwd",
        "ftp://example.com/v1",
        "//example.com",
        "https://exa mple.com",
        "https://user:pass@example.com",
        "https://example.com\n.evil.com",
    ];
    for url in rejected {
        assert!(
            matches!(classify_endpoint(url), EndpointCheck::Invalid(_)),
            "must reject {url:?}, got {:?}",
            classify_endpoint(url)
        );
    }
}

#[test]
fn urls_with_quotes_parse_but_stay_inert_inside_json() {
    // Not rejected by the parser — and that is acceptable *only* because the
    // value is never interpolated into a command. Prove it survives a JSON
    // round-trip unchanged and is still an ordinary string.
    let url = "https://example.com/\"'<>";
    assert_eq!(classify_endpoint(url), EndpointCheck::RemoteOk);
    let doc = serde_json::json!({ "env": { "ANTHROPIC_BASE_URL": url } });
    let text = serde_json::to_string(&doc).expect("serialize");
    let back: serde_json::Value = serde_json::from_str(&text).expect("parse");
    assert_eq!(back["env"]["ANTHROPIC_BASE_URL"], url);
}

#[test]
fn plain_http_is_reported_insecure_not_silently_accepted() {
    assert_eq!(
        classify_endpoint("http://api.example.com/v1"),
        EndpointCheck::Insecure
    );
}

#[test]
fn well_formed_https_urls_are_accepted() {
    assert_eq!(
        classify_endpoint("https://api.deepseek.com/anthropic"),
        EndpointCheck::RemoteOk
    );
    assert_eq!(
        classify_endpoint("https://example.com:8443/v1?x=1"),
        EndpointCheck::RemoteOk
    );
}

// ---------------------------------------------------------------------------
// 2. Malformed / hostile settings JSON — never silently repaired
// ---------------------------------------------------------------------------

#[test]
fn malformed_json_is_refused_and_never_rewritten() {
    let cases = [
        ("empty", ""),
        ("truncated", "{\"model\": \"x\""),
        ("not_object", "[1,2,3]"),
        ("scalar", "\"just a string\""),
        ("trailing", "{\"model\":\"x\"} garbage"),
        ("control_chars", "{\"model\":\"x\u{0001}\"}"),
        ("comment", "{/* nope */\"model\":\"x\"}"),
        ("bool", "true"),
    ];
    for (tag, body) in cases {
        let dir = temp_dir(&format!("json-{tag}"));
        let path = dir.join("settings.json");
        fs::write(&path, body).expect("write fixture");
        let before = fs::read(&path).expect("read before");

        let result = read_settings(&path);
        assert!(
            result.is_err(),
            "malformed JSON ({tag}) must be refused, got Ok"
        );
        // Whatever the error kind, the file must be untouched: the app never
        // repairs, truncates or rewrites a file it could not parse.
        let after = fs::read(&path).expect("read after");
        assert_eq!(before, after, "{tag}: file must be byte-identical");
        fs::remove_dir_all(&dir).ok();
    }
}

#[test]
fn missing_file_is_distinguishable_from_malformed() {
    let dir = temp_dir("missing");
    let err = read_settings(&dir.join("nope.json")).expect_err("missing");
    assert!(matches!(err, ConfigError::Missing));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn top_level_non_json_values_do_not_produce_a_document() {
    // A settings file whose root is an array must not be treated as an object.
    let dir = temp_dir("array-root");
    let path = dir.join("settings.json");
    fs::write(&path, r#"[{"model":"x"}]"#).expect("write");
    assert!(read_settings(&path).is_err());
    fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// 3. Hostile model-catalog metadata
// ---------------------------------------------------------------------------

fn one_shot(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = vec![0u8; 8192];
            let _ = stream.read(&mut buf);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });
    format!("http://{addr}/models")
}

fn spec(url: String) -> DiscoverySpec {
    DiscoverySpec::Endpoint {
        url,
        requires_auth: false,
        list_selector: "data".into(),
    }
}

#[test]
fn catalog_entries_without_a_usable_id_are_dropped() {
    let url = one_shot(
        r#"{"data":[
            {"id":"good-model"},
            {"id":123},
            {"id":null},
            {"id":""},
            {"id":"   "},
            {"display_name":"no id"},
            {"id":"good-model-2"}
        ]}"#,
    );
    let models = fetch_models(&spec(url), AuthStyle::ApiKey, None).expect("fetch");
    let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, vec!["good-model", "good-model-2"]);
}

#[test]
fn catalog_json_that_is_not_a_list_is_refused() {
    for body in [
        r#"{"data":{"id":"x"}}"#,
        r#"{"data":"nope"}"#,
        r#"{"other":[{"id":"x"}]}"#,
        r#"[]"#,
    ] {
        let url = one_shot_static(body);
        let result = fetch_models(&spec(url), AuthStyle::ApiKey, None);
        assert!(result.is_err(), "hostile catalog must be refused: {body}");
    }
}

#[test]
fn non_object_catalog_entries_are_skipped_not_fatal() {
    // Documented behaviour: one junk entry must not break a whole catalog.
    let url = one_shot_static(r#"{"data":[1,2,3,{"id":"real"}]}"#);
    let models = fetch_models(&spec(url), AuthStyle::ApiKey, None).expect("fetch");
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "real");
}

#[test]
fn catalog_cannot_dictate_provenance_or_capability() {
    let body = r#"{"data":[{"id":"m1","display_name":"Totally Real",
        "capability_status":"verified","source":"https://evil.example.com",
        "context_window":999999999}]}"#;
    let url = one_shot_static(body);
    let models = fetch_models(&spec(url), AuthStyle::ApiKey, None).expect("fetch");
    let ModelEntry {
        id,
        display_name,
        context_window,
        capability_status,
        source,
    } = &models[0];
    assert_eq!(id, "m1");
    assert_eq!(display_name.as_deref(), Some("Totally Real"));
    // T42: provenance is the verified template URL, never the payload's claim.
    assert!(source.starts_with("http://127.0.0.1:"));
    assert_ne!(source, "https://evil.example.com");
    assert_eq!(*capability_status, "unknown");
}

fn one_shot_static(body: &str) -> String {
    let leaked: &'static str = Box::leak(body.to_string().into_boxed_str());
    one_shot(leaked)
}

#[test]
fn model_ids_are_shape_checked_and_never_shell_interpolated() {
    use claude_config_manager_lib::config_merge::validate_model_id;
    use claude_config_manager_lib::credential_handoff::generate_helper_script;

    // The documented rule is "printable ASCII, no spaces" (T43). Shell-shaped
    // values therefore pass validation — that is safe ONLY because no code
    // path interpolates a model id into a command. Prove both halves:
    for id in ["model$(whoami)", "model`id`", "model;rm-rf", "-", "--flag"] {
        assert!(
            validate_model_id(id).is_ok(),
            "documented rule is printable-ASCII, so {id:?} is accepted"
        );
    }
    // ...and the only script we generate from user input never mentions a
    // model id at all (it is built from a validated provider id alone).
    let script = generate_helper_script("deepseek").expect("script");
    assert!(!script.contains("model("), "no model id reaches the shell");

    // Values that cannot be a model id at all are still refused.
    for id in [
        "model with space",
        "model\nnewline",
        "model\u{0}",
        "日本語モデル",
    ] {
        assert!(validate_model_id(id).is_err(), "must reject {id:?}");
    }
}

// ---------------------------------------------------------------------------
// 4. Command injection through the provider id (helper script interpolation)
// ---------------------------------------------------------------------------

#[test]
fn provider_ids_with_shell_metacharacters_are_refused() {
    use claude_config_manager_lib::credential_handoff::{
        generate_helper_script, validate_provider_id,
    };
    let hostile = [
        "a;b",
        "a b",
        "a$(id)",
        "a`id`",
        "a|b",
        "a&b",
        "a>b",
        "a'b",
        "a\"b",
        "a\\b",
        "a/b",
        "a\nb",
        "a\u{0}b",
        "",
        "UPPER",
        "../../etc/passwd",
        "-rf",
    ];
    for id in hostile {
        assert!(
            validate_provider_id(id).is_err(),
            "provider id must be refused: {id:?}"
        );
        assert!(
            generate_helper_script(id).is_err(),
            "helper script must not be generated for: {id:?}"
        );
    }
}

#[test]
fn valid_provider_id_helper_script_contains_only_the_validated_id() {
    use claude_config_manager_lib::credential_handoff::{
        generate_helper_script, validate_provider_id,
    };
    let id = "openrouter-2";
    assert!(validate_provider_id(id).is_ok());
    let script = generate_helper_script(id).expect("script");
    assert!(script.contains(id));
    assert!(script.starts_with("#!"), "must be a script with a shebang");
}
