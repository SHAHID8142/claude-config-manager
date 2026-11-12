# V3-34 — GitHub Actions investigation

## Finding

Every workflow run fails in 2–4 seconds. `gh run view <id>` reports the same
annotation for all three jobs (`Frontend checks`, `Rust checks`,
`Secrets & dependencies`):

> The job was not started because your account is locked due to a billing issue.

No step runs. `gh run view --log-failed` returns `log not found` because no logs
exist.

## Conclusion

- This is **not** a build, test or code failure. There is no failing test, lint
  error or compile error behind it.
- It cannot be fixed in the repository: it needs the account owner to resolve
  GitHub billing for Actions.
- CI therefore provides **no signal at all**, and V3 must not describe CI as
  passing. All verification claims in this phase rest on locally executed
  commands, which are recorded with their outputs in `docs/V3/V3-35-regression.md`.

## What the workflow would run once billing is restored

`.github/workflows/ci.yml` on `ubuntu-latest`:

1. Node 26 — install, `tsc --noEmit`, `eslint`, `vitest run`.
2. Rust stable — `cargo fmt --check`, `cargo clippy -- -D warnings`,
   `cargo test`.
3. `gitleaks` secret scan.

All three pass locally on macOS; the Ubuntu runner is unverified because it has
never executed. Two known portability caveats: the helper uses `launchctl` and
`ps`/`kill` (macOS/Unix only, guarded behind named functions), and the Tauri
build job does not exist in CI at all.
