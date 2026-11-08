//! Central secret redaction guard (T23) — threat T2 (secrets in logs,
//! diagnostics, errors, backups).
//!
//! Rule: every string that can reach a log, error message, diagnostic dump
//! or UI detail line passes through `redact_secrets` with the live secret
//! set before leaving the process boundary.

/// Replace every occurrence of each secret with `[REDACTED]`.
///
/// Guarantees:
/// - Empty or whitespace-only secrets are ignored (an empty pattern would
///   otherwise rewrite every character of the input).
/// - Longer secrets are replaced first so that overlapping values (e.g. a
///   key and its prefix) always leave the strongest redaction.
/// - The output never contains any accepted secret verbatim (asserted by tests).
pub fn redact_secrets(input: &str, secrets: &[&str]) -> String {
    let mut out = input.to_string();
    let mut live: Vec<&str> = secrets
        .iter()
        .filter(|s| !s.trim().is_empty())
        .copied()
        .collect();
    live.sort_by_key(|s| std::cmp::Reverse(s.len()));
    for secret in live {
        out = out.replace(secret, "[REDACTED]");
    }
    out
}

/// Heuristic: does this string look like a provider credential?
/// Used defensively to strip secret-shaped material from *untrusted* text
/// (e.g. upstream error messages) where we don't hold the original secret.
pub fn is_secret_shaped(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 16 {
        return false;
    }
    // T60: `key=sk-ant-...` / `authorization: sk-...` forms are where secrets
    // actually appear in error text — test the value after the separator too.
    if let Some(core) = t.rsplit(['=', ':']).next()
        && core.len() >= 16
        && core != t
        && is_secret_core(core)
    {
        return true;
    }
    is_secret_core(t)
}

/// Prefix/shape tests for a single candidate token.
fn is_secret_core(t: &str) -> bool {
    if t.len() < 16 {
        return false;
    }
    const PREFIXES: [&str; 6] = [
        "sk-ant-", // Anthropic
        "sk-or-",  // OpenRouter
        "nvapi-",  // NVIDIA NIM
        "sk-",     // generic OpenAI-style
        "r8_",     // Replicate-style
        "ghp_",    // GitHub PAT (defensive)
    ];
    if PREFIXES.iter().any(|p| t.starts_with(p)) {
        return true;
    }
    // Bearer-ish: long token with mixed alphanumerics, no spaces.
    if !t.contains(' ')
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && t.chars().filter(|c| c.is_ascii_digit()).count() >= 3
        && t.chars().filter(|c| c.is_ascii_alphabetic()).count() >= 8
    {
        return true;
    }
    false
}

/// Strip any secret-shaped tokens from untrusted text.
pub fn strip_secret_shaped(input: &str) -> String {
    input
        .split_whitespace()
        .map(|tok| {
            if is_secret_shaped(tok) {
                "[REDACTED]".to_string()
            } else if let Some((prefix, value)) = tok.rsplit_once(['=', ':'])
                && is_secret_core(value.trim())
            {
                // Keep the key name (useful for diagnosis), drop the secret.
                format!("{prefix}=[REDACTED]")
            } else {
                tok.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_A: &str = "sk-ant-api03-FAKE-aaaa-bbbb";
    const KEY_B: &str = "sk-or-v1-FAKE-cccc-dddd";

    #[test]
    fn replaces_all_occurrences_of_all_secrets() {
        let input = format!("key1={KEY_A} key2={KEY_B} retry key1={KEY_A}");
        let out = redact_secrets(&input, &[KEY_A, KEY_B]);
        assert!(!out.contains(KEY_A));
        assert!(!out.contains(KEY_B));
        assert_eq!(out.matches("[REDACTED]").count(), 3);
    }

    #[test]
    fn empty_secrets_are_ignored_not_destructive() {
        // Critical negative: an empty pattern must NOT wipe the input.
        let out = redact_secrets("endpoint https://api.example.com", &["", "   "]);
        assert_eq!(out, "endpoint https://api.example.com");
    }

    #[test]
    fn overlapping_secrets_longest_first() {
        let prefix = "sk-or-v1-FAKE-";
        let input = format!("token={KEY_B}");
        let out = redact_secrets(&input, &[KEY_B, prefix]);
        assert!(!out.contains(KEY_B));
        assert!(!out.contains(prefix), "prefix must also be gone: {out}");
    }

    #[test]
    fn no_secrets_means_input_unchanged() {
        let out = redact_secrets("nothing to hide", &[]);
        assert_eq!(out, "nothing to hide");
    }

    #[test]
    fn secret_shaped_detection() {
        assert!(is_secret_shaped(KEY_A));
        assert!(is_secret_shaped(KEY_B));
        assert!(is_secret_shaped("nvapi-FAKE1234567890abcd"));
        assert!(is_secret_shaped("Abcdefghij1234567890")); // bearer-ish
        assert!(!is_secret_shaped("claude-sonnet-4-5")); // model id
        assert!(!is_secret_shaped("https://api.example.com"));
        assert!(!is_secret_shaped("short"));
        assert!(!is_secret_shaped(""));
    }

    #[test]
    fn strip_secret_shaped_removes_tokens_keeps_rest() {
        let out = strip_secret_shaped("auth failed for sk-or-v1-FAKE-cccc-dddd at /v1/messages");
        assert!(out.contains("[REDACTED]"));
        assert!(out.contains("/v1/messages"));
        assert!(!out.contains("sk-or-v1-FAKE"));
    }
}
