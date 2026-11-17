//! V2-33/34/35: provider failure classification, cooldown and bounded retry.
//!
//! Rules that matter for real money and real work:
//! * `429` is **rate limiting, not exhausted quota** — the body decides whether
//!   the account is out of credit (402-style wording) or merely too fast.
//! * A request that already started streaming is never replayed; the proxy only
//!   retries before the first response byte reaches Claude Code.
//! * A 4xx caused by the request itself (`400`, `404`, `422`) is not retried:
//!   another provider would reject the same body, and a retry could duplicate a
//!   side effect the caller already paid for.
//! * Cooldowns are time-bounded and cleared by a later success — quota is never
//!   treated as permanent.

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// 401/403: the key is wrong, revoked, or the account cannot use this model.
    Auth,
    /// 402, or a 429 whose body says the account is out of credit/quota.
    Quota,
    /// 429 without quota wording: back off and try again later.
    RateLimit,
    /// 5xx from the provider.
    ServerError,
    /// No answer within the timeout.
    Timeout,
    /// Connection refused / DNS / TLS failure.
    Network,
    /// A structured provider error identifies an unavailable/unknown model.
    Model,
    /// The request itself is wrong; never retried elsewhere.
    Request,
}

impl FailureClass {
    /// Whether trying the next target can plausibly help.
    pub fn retryable(self) -> bool {
        !matches!(self, FailureClass::Request)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            FailureClass::Auth => "auth",
            FailureClass::Quota => "quota",
            FailureClass::RateLimit => "rate_limit",
            FailureClass::ServerError => "server_error",
            FailureClass::Timeout => "timeout",
            FailureClass::Network => "network",
            FailureClass::Request => "request",
            FailureClass::Model => "model_unavailable",
        }
    }
}

/// Quota/exhaustion wording that turns a 429 into a quota failure.
const QUOTA_HINTS: [&str; 6] = [
    "insufficient",
    "quota",
    "credit",
    "exhaust",
    "billing",
    "payment required",
];

/// Classify a provider answer. `body` is used only for the 429 ambiguity; it is
/// never logged or stored (it can echo request content).
pub fn classify(status: u16, body: &str) -> FailureClass {
    match status {
        401 | 403 => FailureClass::Auth,
        402 => FailureClass::Quota,
        429 => {
            let lower = body.to_ascii_lowercase();
            if QUOTA_HINTS.iter().any(|h| lower.contains(h)) {
                FailureClass::Quota
            } else {
                FailureClass::RateLimit
            }
        }
        500..=599 => FailureClass::ServerError,
        400 | 404 | 405 | 409 | 413 | 415 | 422 => FailureClass::Request,
        // 2xx never reaches classification; anything else (3xx, 408, odd codes)
        // is treated as a request-level problem rather than guessed at.
        _ => FailureClass::Request,
    }
}

/// Inspect only a bounded structured error body; never store/echo it. A generic
/// 400/404 (including a wrong endpoint) is not evidence that a model is missing.
pub fn model_unavailable(status: u16, body: &[u8]) -> bool {
    if !matches!(status, 400 | 403 | 404 | 422) {
        return false;
    }
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) else {
        return false;
    };
    let error = v.get("error").unwrap_or(&v);
    let code = error
        .get("code")
        .or_else(|| error.get("type"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    if matches!(
        code,
        "model_not_found"
            | "model_not_available"
            | "model_unavailable"
            | "model_not_supported"
            | "unknown_model"
            | "invalid_model"
    ) {
        return true;
    }
    let message = error
        .get("message")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(code, "not_found_error" | "invalid_request_error")
        && message.contains("model")
        && [
            "not found",
            "does not exist",
            "not available",
            "unavailable",
            "unknown model",
            "invalid model",
        ]
        .iter()
        .any(|s| message.contains(s))
}

/// Retry-After supports delay-seconds and HTTP-date (RFC 9110 §10.2.3).
/// Clamp to ten minutes; expired dates permit an immediate next attempt.
pub fn parse_retry_after(value: Option<&str>) -> Option<Duration> {
    parse_retry_after_at(value, std::time::SystemTime::now())
}
fn parse_retry_after_at(value: Option<&str>, now: std::time::SystemTime) -> Option<Duration> {
    let raw = value?.trim();
    let wait = if raw.chars().all(|c| c.is_ascii_digit()) && !raw.is_empty() {
        Duration::from_secs(raw.parse::<u64>().unwrap_or(u64::MAX))
    } else {
        httpdate::parse_http_date(raw)
            .ok()?
            .duration_since(now)
            .unwrap_or_default()
    };
    Some(wait.min(Duration::from_secs(600)))
}

/// Cooldown for a failed target. Bounded so a stale failure cannot disable it
/// forever: quota is 6 h, not "permanent".
pub fn cooldown_for(class: FailureClass, retry_after: Option<Duration>) -> Duration {
    match class {
        FailureClass::RateLimit => retry_after.unwrap_or(Duration::from_secs(30)),
        FailureClass::ServerError => retry_after.unwrap_or(Duration::from_secs(20)),
        FailureClass::Timeout | FailureClass::Network => Duration::from_secs(10),
        FailureClass::Quota => Duration::from_secs(6 * 60 * 60),
        FailureClass::Auth => Duration::from_secs(6 * 60 * 60),
        FailureClass::Model => retry_after.unwrap_or(Duration::from_secs(300)),
        FailureClass::Request => Duration::from_secs(0),
    }
}

/// Per-target cooldown table. `Instant`-based, so the OS clock cannot un-cool a
/// target; a restart clears everything, which is correct for a local helper.
#[derive(Debug, Default)]
pub struct Cooldowns {
    until: HashMap<String, Instant>,
}

impl Cooldowns {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_cooling(&mut self, target_id: &str, now: Instant) -> bool {
        match self.until.get(target_id) {
            Some(t) if *t > now => true,
            Some(_) => {
                self.until.remove(target_id);
                false
            }
            None => false,
        }
    }

    pub fn mark(
        &mut self,
        target_id: &str,
        class: FailureClass,
        retry_after: Option<Duration>,
        now: Instant,
    ) {
        let wait = cooldown_for(class, retry_after);
        if wait.is_zero() {
            return;
        }
        self.until.insert(target_id.to_string(), now + wait);
    }

    /// A success clears the target's cooldown immediately.
    pub fn clear(&mut self, target_id: &str) {
        self.until.remove(target_id);
    }

    pub fn remaining_seconds(&self, target_id: &str, now: Instant) -> Option<u64> {
        self.until
            .get(target_id)
            .filter(|until| **until > now)
            .map(|until| until.duration_since(now).as_secs().saturating_add(1))
    }

    pub fn prune(&mut self, now: Instant) {
        self.until.retain(|_, t| *t > now);
    }

    #[cfg(test)]
    pub fn active(&self) -> usize {
        self.until.len()
    }
}

/// Ordered candidates for one request: enabled targets, skipping cooling ones,
/// preserving configured order. All-cooling means temporarily unavailable;
/// callers can show cooldown deadlines or request an explicit helper restart.
pub fn candidates(ids: &[(String, bool)], cooldowns: &mut Cooldowns, now: Instant) -> Vec<String> {
    let enabled: Vec<&(String, bool)> = ids.iter().filter(|(_, on)| *on).collect();
    let ready: Vec<String> = enabled
        .iter()
        .filter(|(id, _)| !cooldowns.is_cooling(id, now))
        .map(|(id, _)| id.clone())
        .collect();
    ready
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_distinguishes_rate_limit_from_exhausted_quota() {
        assert_eq!(classify(429, "slow down"), FailureClass::RateLimit);
        assert_eq!(
            classify(
                429,
                r#"{"error":{"message":"You exceeded your current quota"}}"#
            ),
            FailureClass::Quota
        );
        assert_eq!(classify(402, ""), FailureClass::Quota);
        assert_eq!(classify(401, ""), FailureClass::Auth);
        assert_eq!(classify(403, ""), FailureClass::Auth);
        assert_eq!(classify(503, ""), FailureClass::ServerError);
        // A malformed request must not be replayed against another provider.
        assert_eq!(classify(400, ""), FailureClass::Request);
        assert_eq!(classify(422, ""), FailureClass::Request);
        assert!(!FailureClass::Request.retryable());
        assert!(FailureClass::Quota.retryable());
    }

    #[test]
    fn retry_after_is_bounded_and_accepts_http_dates() {
        assert_eq!(parse_retry_after(Some("12")), Some(Duration::from_secs(12)));
        assert_eq!(
            parse_retry_after(Some("86400")),
            Some(Duration::from_secs(600)),
            "a day-long Retry-After must not disable failover"
        );
        let now = std::time::UNIX_EPOCH + Duration::from_secs(1792567680);
        let future = httpdate::fmt_http_date(now + Duration::from_secs(45));
        assert_eq!(
            parse_retry_after_at(Some(&future), now),
            Some(Duration::from_secs(45))
        );
        assert_eq!(
            parse_retry_after_at(Some(&future), now + Duration::from_secs(60)),
            Some(Duration::ZERO)
        );
        assert_eq!(parse_retry_after(None), None);
        assert_eq!(parse_retry_after(Some("-5")), None);
    }

    #[test]
    fn cooldowns_are_bounded_and_cleared_by_success() {
        let now = Instant::now();
        assert_eq!(
            cooldown_for(FailureClass::Quota, None),
            Duration::from_secs(6 * 3600)
        );
        assert_eq!(
            cooldown_for(FailureClass::Auth, None),
            Duration::from_secs(6 * 3600)
        );
        assert!(cooldown_for(FailureClass::Request, None).is_zero());

        let mut c = Cooldowns::new();
        c.mark(
            "a",
            FailureClass::RateLimit,
            Some(Duration::from_secs(5)),
            now,
        );
        assert!(c.is_cooling("a", now));
        assert!(!c.is_cooling("a", now + Duration::from_secs(6)));
        c.mark("b", FailureClass::ServerError, None, now);
        c.clear("b");
        assert!(!c.is_cooling("b", now));
        assert_eq!(c.active(), 0);
    }

    #[test]
    fn candidates_respect_cooldowns_when_every_key_is_unavailable() {
        let now = Instant::now();
        let ids = vec![
            ("a".to_string(), true),
            ("b".to_string(), false),
            ("c".to_string(), true),
        ];
        let mut c = Cooldowns::new();
        assert_eq!(candidates(&ids, &mut c, now), ["a", "c"]);

        c.mark("a", FailureClass::RateLimit, None, now);
        assert_eq!(candidates(&ids, &mut c, now), ["c"]);

        c.mark("c", FailureClass::RateLimit, None, now);
        // Respect every pause; a subsequent request must not hammer exhausted keys.
        assert!(candidates(&ids, &mut c, now).is_empty());
        assert_eq!(
            candidates(&ids, &mut c, now + Duration::from_secs(31)),
            ["a", "c"]
        );
    }
}

#[cfg(test)]
mod model_failure_tests {
    use super::*;
    #[test]
    fn structured_model_errors_are_retryable_but_generic_bad_requests_are_not() {
        for status in [400, 403, 404, 422] {
            assert!(model_unavailable(
                status,
                br#"{"error":{"code":"model_not_found","message":"Hidden upstream content"}}"#
            ));
            assert!(!model_unavailable(status,br#"{"error":{"type":"invalid_request_error","message":"messages must be an array"}}"#));
            assert!(!model_unavailable(status, b"model not found"));
            assert!(!model_unavailable(
                status,
                br#"{"error":{"message":"model not found"}}"#
            ));
        }
        assert!(model_unavailable(404,br#"{"error":{"type":"not_found_error","message":"model: unavailable-model not found"}}"#));
        assert!(!model_unavailable(
            500,
            br#"{"error":{"code":"model_not_found"}}"#
        ));
        assert!(FailureClass::Model.retryable());
    }
}
