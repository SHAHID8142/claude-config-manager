# V3-02 — Verification of the seven reported defects

Method: read the current code, then try to reproduce. "Confirmed" means the
defect exists in `9d17575`; "reclassified" means the report described a symptom
whose cause is different. No behaviour was changed in this task.

## 1. The proxy stops when the GUI exits — **CONFIRMED**

`src-tauri/src/lib.rs:168` holds `AppState { proxy: Mutex<Option<ProxyHandle>> }`
and `proxy_start` creates an in-process `ProxyHandle` (`proxy.rs:138`) whose
listener thread belongs to the Tauri process. `Drop for ProxyHandle`
(`proxy.rs:110`) sets the shutdown flag and joins the accept loop. Quitting the
app therefore terminates the proxy by construction: there is no detached
process, no launchd job, and no state file.

Reproduction evidence: the loopback suite runs the proxy inside the test process
(`tests/proxy_loopback.rs`), which is the same lifetime model.

## 2. OpenAI-only providers have no protocol translation — **CONFIRMED (by design in V2)**

`provider_catalog.rs` marks NVIDIA NIM and similar entries
`protocol: open_bridge_only` / `open_ai_only` with `bridge: required_for_open_ai_only`;
`src/components/CapabilityBadge.tsx:58` renders "bridge required (arrives in a
later milestone)". `proxy.rs` forwards `/v1/messages` verbatim and rewrites only
the model name and the auth header — no request or response translation exists.
ADR-003 deliberately excluded it. V3 Phase C owns this; installing a bridge is a
material architecture change and stays blocked until the owner approves one.

## 3. The header says "connected" without verified connectivity — **CONFIRMED**

`src/lib/connectionLabel.ts` derives its label purely from the settings file:
`apiKeyHelper !== null → "connected"`. A saved helper script proves only that a
file exists; it says nothing about whether the key is valid, the provider is
reachable, or the proxy is running. Distinct states (Configured / Checking /
Verified / Unavailable / Proxy stopped / Action required) do not exist. V3-24.

## 4. Global Save & Apply can replace a working routed configuration — **CONFIRMED**

`save_and_apply` (`lib.rs:89`) always writes the direct configuration
(`ANTHROPIC_BASE_URL` = provider endpoint, plaintext-free credential helper),
while `apply_routed_mode` (`lib.rs:323`) writes `ANTHROPIC_BASE_URL =
http://127.0.0.1:<port>`. The footer button in `src/App.tsx` is wired to
`save_and_apply` unconditionally, so pressing it while routed mode is active
silently returns Claude Code to Direct Mode. There is no mode indicator, no
confirmation, and no routing-state check in the handler. V3-25.

## 5. The credentials screen shows the wrong storage path — **CONFIRMED**

`src/tabs/CredentialsTab.tsx:87` claims
`~/Library/Application Support/com.claude-config-manager/secrets/`.
The backend writes to `~/.claude-config-manager/secrets/`
(`file_secret_store.rs:39`, `credential_handoff.rs:78`,
`credential_vault.rs:382`, `save_apply.rs:459`). The path is also hardcoded in
the UI instead of being reported by the backend. V3-26.

## 6. Routing target credentials may be stale — **REPORTED SYMPTOM, root cause different**

`RoutingPanel.tsx:74` filters credentials by the chosen provider id and
`App.tsx` stores the routing provider's list in the *shared* `credentials`
state. A target therefore cannot be saved with another provider's key
(`validate()` in `routing_config.rs` rejects mismatched credential ids), so this
is not a data-integrity bug. It is a UI-state bug: the routing screen overwrites
the list the Provider/Credentials screens are showing, and a failed load
(`.catch(() => setCredentials(null))`) silently yields "No stored key". V3-27
keeps the fix narrow: routing keeps its own per-provider list.

## 7. GitHub CI is failing — **CONFIRMED, and it is not a code failure**

`gh run list` shows every run failing in 2–4 s. `gh run view` prints, for all
three jobs:

> "The job was not started because your account is locked due to a billing issue."

No step ever executes, so there is no build or test signal from CI. This is a
repository-owner billing issue (V3-34) and cannot be fixed in code; CI must not
be described as passing. Local gates remain the only executed evidence.

## Regression tests created for these defects

See `V3-03` and the test files
`src-tauri/tests/v3_regressions.rs` (1, 3, 4, 6, 7) and
`src/v3-regressions.test.tsx` (3, 4, 5, 6).
