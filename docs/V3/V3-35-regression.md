# V3-35 — Complete regression (executed 2026-11-08T14:52Z on this Mac)

Toolchain: v26.10.0, cargo 1.99.1 (5f94df478 2026-08-27), 1.99.1.

```
$ cargo fmt --manifest-path src-tauri/Cargo.toml --check
exit 0 — no diff

$ cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s

$ cargo test --manifest-path src-tauri/Cargo.toml
Running unittests src/lib.rs (src-tauri/target/debug/deps/claude_config_manager_lib-532bd2b008a156a1)
test result: ok. 236 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1.08s
Running unittests src/main.rs (src-tauri/target/debug/deps/claude_config_manager-d137b7010afa61df)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Running tests/adversarial_paths.rs (src-tauri/target/debug/deps/adversarial_paths-02dc824ab21b7bb1)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
Running tests/crash_recovery.rs (src-tauri/target/debug/deps/crash_recovery-14b8fb2b629503f6)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
Running tests/helper_lifecycle.rs (src-tauri/target/debug/deps/helper_lifecycle-b606245e707ef872)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.82s
Running tests/lifecycle.rs (src-tauri/target/debug/deps/lifecycle-377c9d92bed12c06)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
Running tests/malformed_inputs.rs (src-tauri/target/debug/deps/malformed_inputs-e93f442246d8d347)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
Running tests/merge_properties.rs (src-tauri/target/debug/deps/merge_properties-b1662dcf3d16f2c4)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
Running tests/proxy_loopback.rs (src-tauri/target/debug/deps/proxy_loopback-c274511885e8ee1c)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
Running tests/proxy_security.rs (src-tauri/target/debug/deps/proxy_security-db7b862ac0cc81c4)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.06s
Running tests/secret_and_network_surface.rs (src-tauri/target/debug/deps/secret_and_network_surface-1e649f0804dee586)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
Running tests/v3_regressions.rs (src-tauri/target/debug/deps/v3_regressions-03976a3d52d2a77a)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

$ npx tsc --noEmit
exit 0

$ npm run lint
> eslint .


$ npx vitest run
[2m Test Files [22m [1m[32m25 passed[39m[22m[90m (25)[39m
[2m      Tests [22m [1m[32m196 passed[39m[22m[90m (196)[39m

$ bash scripts/dep-audit.sh

PASS: no advisory affects a dependency of the macOS build

$ npm run tauri build   # re-run after the final clippy fix
BUILD_EXIT=0   ·   DMG: Claude Config Manager_0.1.0_aarch64.dmg (5.93 MiB)
```

## Summary

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean (after removing three unused imports) |
| `cargo test` | **314 passed / 0 failed** (3 live tests ignored by design) |
| `tsc --noEmit` | clean |
| `eslint .` | clean |
| `vitest run` | **196 passed / 25 files** |
| `scripts/dep-audit.sh` | PASS — no advisory affects a dependency |
| `npm run tauri build` | exit 0, bundle + DMG produced |
| GitHub Actions | **not run** — account billing lock (V3-34); no CI signal exists |
