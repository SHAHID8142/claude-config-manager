//! Bridge manifest (T49): the committed pin for the third-party bridge.
//!
//! Every field here is supply-chain fact, recorded at pin time and never
//! fetched at runtime:
//! - `integrity_sha512_b64` is npm's `dist.integrity` for the exact tarball
//!   (subresource-integrity format: `sha512-<base64>`).
//! - The upstream license is verified MIT (GitHub API + package.json).
//! - `min_node_major` comes from the package's own `engines` field.
//!
//! Changing ANY field is a deliberate re-pin and must ship with a new audit
//! entry (ADR-001 discipline).

use serde::Serialize;

/// The pinned bridge package and its supply-chain facts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BridgePin {
    pub package: &'static str,
    pub version: &'static str,
    pub tarball_url: &'static str,
    /// npm `dist.integrity` — `sha512-<base64>` of the tarball bytes.
    pub integrity_sha512_b64: &'static str,
    pub min_node_major: u32,
    pub license: &'static str,
    pub license_source: &'static str,
    /// Human-facing size of the tarball, from the registry, for the consent UI.
    pub tarball_bytes: u64,
}

pub const PINNED: BridgePin = BridgePin {
    package: "@musistudio/claude-code-router",
    version: "3.1.3",
    tarball_url: "https://registry.npmjs.org/@musistudio/claude-code-router/-/claude-code-router-3.1.3.tgz",
    integrity_sha512_b64: "sha512-dVHRu9Sb/xG9dgr8nHJmvd5XVIQ5yrEeKmMoPUEBc9xtgykfRT2z76NRI+fylpeiMzYC60z6vrV6x/3IY52pvg==",
    min_node_major: 22,
    license: "MIT",
    license_source: "https://github.com/musistudio/claude-code-router (LICENSE, verified via GitHub API 2026-11-08)",
    tarball_bytes: 6_529_968, // exact size, downloaded and sha512-verified at pin time (2026-11-13)
};

/// Loopback ports the bridge uses (both bind 127.0.0.1 only — T51).
pub const GATEWAY_PORT: u16 = 3456;
pub const MGMT_PORT: u16 = 3458;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_is_complete_and_wellformed() {
        assert!(PINNED.package.starts_with('@'));
        assert!(
            PINNED
                .tarball_url
                .starts_with("https://registry.npmjs.org/")
        );
        assert!(PINNED.tarball_url.contains(PINNED.version));
        assert!(PINNED.integrity_sha512_b64.starts_with("sha512-"));
        // Base64 part decodes to exactly 64 bytes (sha512).
        use base64::Engine as _;
        let b64 = PINNED
            .integrity_sha512_b64
            .strip_prefix("sha512-")
            .expect("prefix");
        let raw = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .expect("valid base64");
        assert_eq!(raw.len(), 64, "sha512 digest must be 64 bytes");
        assert!(PINNED.min_node_major >= 22);
        assert_eq!(PINNED.license, "MIT");
    }

    #[test]
    fn ports_are_loopback_plan_only() {
        // The ports are documented loopback services; nothing here binds them.
        assert_ne!(GATEWAY_PORT, MGMT_PORT);
    }
}
