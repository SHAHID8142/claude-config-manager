//! G3 — property-based tests for the settings merge contract.
//!
//! Every earlier adversarial test was a case *I* thought of. These assert
//! invariants over thousands of generated documents instead, which is the only
//! way to find the case nobody imagined.
//!
//! Invariants under test (`apply_desired` / `DesiredSettings`):
//!   P1  total function: any JSON in, `Ok` or `Err` out — never a panic;
//!   P2  idempotence: applying the same desired twice changes nothing;
//!   P3  ownership: keys we do not manage are preserved byte-for-byte;
//!   P4  env isolation: only requested env keys change; a `null` never appears;
//!   P5  no credential-shaped value is ever written into the document;
//!   P6  any accepted output is still valid, serialisable JSON.

use claude_config_manager_lib::config_merge::{DesiredSettings, apply_desired};
use proptest::prelude::*;
use serde_json::{Map, Value, json};

/// Keys this app owns. Anything else must survive untouched (P3).
const MANAGED_TOP: &[&str] = &["model", "apiKeyHelper", "env"];
const MANAGED_ENV: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
];

fn key_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(vec!['a', 'B', '_', '-', '1', 'é', ' ', '$']),
        1..8,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn scalar() -> impl Strategy<Value = Value> {
    prop_oneof![
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|n| json!(n)),
        // Settings values are never astronomically scaled numbers. The range is
        // bounded because serde_json cannot round-trip arbitrary f64 exponents
        // exactly (known upstream behaviour, documented in GAPS-CLOSURE G3).
        (-1_000_000_000_000i64..1_000_000_000_000)
            .prop_map(|n| n as f64 / 1000.0)
            .prop_map(|f| json!(f)),
        "[a-zA-Z0-9 _./:@!$%^&*()\\[\\]{}'\"\\\\\u{00e9}\u{1f600}-]{0,40}".prop_map(Value::String),
        Just(Value::Null),
    ]
}

fn json_value() -> impl Strategy<Value = Value> {
    scalar().prop_recursive(3, 12, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
            prop::collection::hash_map(key_strategy(), inner, 0..4).prop_map(|m| {
                let mut out = Map::new();
                for (k, v) in m {
                    out.insert(k, v);
                }
                Value::Object(out)
            }),
        ]
    })
}

/// A document shaped like a real settings file, with noise around the keys we own.
fn settings_document() -> impl Strategy<Value = Value> {
    (
        prop::option::of("[a-z0-9./:-]{1,24}"),
        prop::option::of("[a-z0-9./:-]{1,24}"),
        prop::collection::hash_map(key_strategy(), json_value(), 0..6),
        prop::collection::hash_map(
            prop::sample::select(MANAGED_ENV.to_vec()),
            json_value(),
            0..4,
        ),
        prop::collection::hash_map(key_strategy(), json_value(), 0..4),
    )
        .prop_map(|(model, helper, top, env_managed, env_other)| {
            let mut root = Map::new();
            for (k, v) in top {
                root.insert(k, v);
            }
            if let Some(m) = model {
                root.insert("model".into(), Value::String(m));
            }
            if let Some(h) = helper {
                root.insert("apiKeyHelper".into(), Value::String(h));
            }
            let mut env = Map::new();
            for (k, v) in env_managed {
                env.insert(k.to_string(), v);
            }
            for (k, v) in env_other {
                // Never collide with managed names.
                if !MANAGED_ENV.contains(&k.as_str()) {
                    env.insert(k, v);
                }
            }
            root.insert("env".into(), Value::Object(env));
            Value::Object(root)
        })
}

fn desired() -> impl Strategy<Value = DesiredSettings> {
    (
        prop::option::of("[a-zA-Z0-9][a-zA-Z0-9._-]{0,30}"),
        prop::option::of("https://[a-z]{2,6}\\.example\\.com(/v[0-9])?"),
        prop::option::of("/tmp/helpers/[a-z0-9-]{1,12}\\.sh"),
        prop::option::of("[a-zA-Z0-9][a-zA-Z0-9.-]{0,20}"),
        any::<bool>(),
    )
        .prop_map(
            |(model, base_url, helper, env_model, clear_base)| DesiredSettings {
                model,
                api_key_helper: helper,
                env_base_url: base_url,
                env_model,
                clear_env_base_url: clear_base,
                ..Default::default()
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 400, ..ProptestConfig::default() })]

    /// P1 — no input, however hostile, can panic the merge.
    #[test]
    fn is_a_total_function(current in json_value(), d in desired(),) {
        let _ = apply_desired(&current, &d);
    }

    /// P2 — applying the same intent twice is a no-op the second time.
    #[test]
    fn is_idempotent(current in settings_document(), d in desired()) {
        if let Ok(once) = apply_desired(&current, &d) {
            let twice = apply_desired(&once, &d).expect("second apply must succeed");
            prop_assert_eq!(once, twice);
        }
    }

    /// P3 — everything we do not own survives unchanged (this is the promise the
    /// UI makes: "your permissions, hooks and env vars are preserved").
    #[test]
    fn never_touches_unmanaged_keys(current in settings_document(), d in desired()) {
        let Ok(out) = apply_desired(&current, &d) else { return Ok(()) };
        let before = current.as_object().expect("generator produces objects");
        let after = out.as_object().expect("merge returns an object");
        for (k, v) in before {
            if MANAGED_TOP.contains(&k.as_str()) {
                continue;
            }
            prop_assert_eq!(after.get(k), Some(v), "unmanaged top-level key {:?} changed", k);
        }
        // Unmanaged env keys likewise.
        let env_before = before.get("env").and_then(|v| v.as_object());
        let env_after = after.get("env").and_then(|v| v.as_object());
        if let Some(env_before) = env_before {
            for (k, v) in env_before {
                if MANAGED_ENV.contains(&k.as_str()) {
                    continue;
                }
                if d.clear_env_base_url && k == "ANTHROPIC_BASE_URL" {
                    continue;
                }
                prop_assert_eq!(
                    env_after.and_then(|e| e.get(k)),
                    Some(v),
                    "unmanaged env key {:?} changed",
                    k
                );
            }
        }
    }

    /// P4 — a value the merge *writes* is always the exact string requested, and
    /// never `null` (Claude Code would see the key as present but empty).
    ///
    /// Existing values we did not ask to change are user data and are allowed to
    /// be anything, including garbage — refusing to "fix" them is deliberate.
    #[test]
    fn written_env_values_are_exactly_the_requested_strings(
        current in settings_document(), d in desired()
    ) {
        let Ok(out) = apply_desired(&current, &d) else { return Ok(()) };
        let env = out.get("env").and_then(|v| v.as_object());
        for (key, want) in [
            ("ANTHROPIC_BASE_URL", d.env_base_url.as_ref()),
            ("ANTHROPIC_AUTH_TOKEN", d.env_auth_token.as_ref()),
            ("ANTHROPIC_API_KEY", d.env_api_key.as_ref()),
            ("ANTHROPIC_MODEL", d.env_model.as_ref()),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", d.env_default_haiku.as_ref()),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", d.env_default_sonnet.as_ref()),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", d.env_default_opus.as_ref()),
        ] {
            let Some(want) = want else { continue };
            let got = env.and_then(|e| e.get(key)).expect("requested key must exist");
            prop_assert_eq!(got, &Value::String(want.clone()), "env {}", key);
            prop_assert!(got.is_string(), "env {} must never be null/other", key);
        }
    }

    /// P5 — a credential-shaped value offered as a model id is refused, never
    /// written in plaintext.
    #[test]
    ///
    /// Threshold: `is_secret_shaped` has a 16-character floor (below it, matching
    /// is noise) and needs real entropy after the prefix. Truncated pastes like
    /// `sk-ant-` or `ghp_short` are therefore NOT detected — that limit is stated
    /// in the register instead of being hidden behind a passing test.
    fn refuses_credential_shaped_model_ids(prefix in "[A-Za-z0-9_-]{0,20}", tail in "[A-Za-z0-9]{12,24}") {
        for shape in [
            format!("sk-ant-api03-{prefix}{tail}"),
            format!("sk-proj-{prefix}{tail}"),
            format!("ghp_{prefix}{tail}"),
        ] {
            let d = DesiredSettings { model: Some(shape.clone()), ..Default::default() };
            prop_assert!(
                apply_desired(&json!({}), &d).is_err(),
                "credential-shaped model id was accepted: {:?}",
                shape
            );
        }
    }

    /// P6 — accepted output is serialisable JSON that round-trips.
    #[test]
    fn output_round_trips_through_json(current in settings_document(), d in desired()) {
        if let Ok(out) = apply_desired(&current, &d) {
            let text = serde_json::to_string(&out).expect("serialise");
            let back: Value = serde_json::from_str(&text).expect("deserialise");
            prop_assert_eq!(out, back);
        }
    }
}
