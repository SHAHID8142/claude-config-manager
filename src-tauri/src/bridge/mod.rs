//! Bridge subsystem (ADR-001, T47–T58): the pinned third-party translator for
//! OpenAI-only providers.
//!
//! Module map:
//! - `manifest` — the committed supply-chain pin (version, sha512, license).
//! - `installer` — T49/T50: download → integrity → extract → versioned
//!   install → rollback. Never executes.
//! - `runtime` — T51/T52/T53: loopback-only config generation, detached
//!   `ccr serve` lifecycle, consent marker.

pub mod installer;
pub mod manifest;
pub mod runtime;
