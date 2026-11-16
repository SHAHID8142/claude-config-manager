# Complete Sequential Atomic Backlog — Autonomous Execution
The agent executes this entire list from top to bottom without new prompts or per-task user approval. Each item is a SMALL change. For every task: tests + negative case + security/UX check (as relevant) + audit + commit + push. Research tasks are documentation-only. Do not implement future CLI tools.

## Discovery and risk control
T01 Audit repo, workspace, remote and toolchain; record baseline.
T02 Verify current official Claude Code global configuration paths, keys, precedence and auth mechanisms; document matrix.
T03 Evaluate existing open-source bridges: Anthropic→OpenAI translation, streaming, tools, vision, reasoning, license, releases, OS support, maintenance.
T04 Select bridge with evidence and write ADR; if no safe candidate exists, STOP for decision.
T05 Threat model: credentials, config corruption, localhost exposure, download supply chain and OS integration.
T06 Define CLIAdapter, provider catalog, credential storage, config writer and bridge manager boundaries (documentation only).
T07 Establish acceptance fixtures, test strategy and security checklist.

## Foundation
T08 Initialize minimal Tauri 2 + Rust + React + TypeScript + Tailwind application.
T09 Configure formatting, lint, TypeScript checks, Rust checks and unit test runners.
T10 Add CI workflow for available checks, secret scanning and dependency audit.
T11 Define typed UI↔Rust command contracts with mocks.
T12 Create minimal single-window macOS-friendly layout.
T13 Add light/dark system theme and responsive sizing.
T14 Add accessible form labels, keyboard navigation and focus states.
T15 Add clear loading, error and success feedback components.

## Provider and credential UX
T16 Define versioned provider template schema with direct/bridge protocol metadata.
T17 Add ONE verified provider preset with test fixtures.
T18 Add provider selector and custom provider entry.
T19 Add validated editable Base URL field and secure protocol warnings.
T20 Add masked API key entry and reveal/hide interaction.
T21 Implement OS credential vault interface with mock tests.
T22 Implement macOS Keychain-backed vault and error handling.
T23 Implement secret lifecycle (replace/delete) and redaction tests.
T24 Add provider template presets individually ONLY when verified; one provider per commit.
T25 Add explicit capability/compatibility labels (verified, unknown, bridge required).

## Global configuration safety
T26 Implement read-only global Claude Code config locator.
T27 Parse and preserve unrelated settings, with invalid JSON tests.
T28 Implement minimal targeted configuration merge and schema validation.
T29 Implement secure backup with correct file permissions and secret redaction policy.
T30 Implement atomic writes, concurrent modification detection and rollback.
T31 Add symlink/path traversal and permission-denied negative tests.
T32 Implement credential handoff for Claude Code based on verified docs; STOP if secure persistent handoff is unresolved.
T33 Implement Save & Apply for ONE verified direct provider.
T34 Implement read-back status and effective-config/override warnings.
T35 Add restore-last-backup operation and tests.
T36 Add explicit failure handling for missing CLI, incompatible versions and invalid settings.

## Model selection
T37 Implement provider model discovery interface with mock API.
T38 Add discovery for one documented provider.
T39 Add refresh, loading, timeout and error states.
T40 Add searchable model list; NEVER auto-select.
T41 Add manual Model ID entry when discovery unavailable.
T42 Add model metadata with source/unknown labels; no invented capability values.
T43 Add explicit model selection validation and Save & Apply integration.
T44 Add additional model discovery adapters one verified provider per commit.
T45 Add advanced settings panel with default-no-override behavior.
T46 Add only verified Claude Code model-role mapping if needed; no automatic failover.

## Bridge setup and lifecycle
T47 Document selected bridge's supported features and known losses per provider/model.
T48 Add first-run install-consent UI, including source/license/version.
T49 Implement pinned download with integrity verification; no execution yet.
T50 Implement secure install/update/rollback in app-owned directory.
T51 Implement loopback-only bridge configuration and health checks.
T52 Implement minimal bridge helper start/stop/status and failure recovery.
T53 Define GUI-closed lifecycle/startup strategy for VS Code CLI; test or STOP for decision.
T54 Integrate bridge mode with global Claude Code config safely.
T55 Test streaming and tool calling with local fixtures.
T56 Test vision/reasoning capability paths where actually supported; report limitations.
T57 Add error classification, timeout handling and safe diagnostics with no secret leakage.
T58 Add bridge uninstall and clean direct-mode switching.

## Hardening and release
T59 Add malformed URL, JSON, model metadata and command injection regression suite.
T60 Add localhost exposure, download tampering and secret-leak regression suite.
T61 Add config rollback and interrupted-write recovery tests.
T62 Perform keyboard, accessibility, macOS theme and small-window UX audit.
T63 Perform dependency/license/vulnerability audit and remediate critical findings.
T64 Build signed/notarized macOS package if credentials and approval available; otherwise create documented unsigned test build and report limitation.
T65 Perform real macOS clean-install, VS Code terminal direct-mode and bridge-mode smoke tests (request user evidence if machine unavailable).
T66 Test helper restart, GUI closure, clean uninstall and backup restore.
T67 Run complete regression suite and independently review all task acceptance criteria.
T68 Write user guide, troubleshooting, bridge limitations and Windows porting notes.
T69 Write `FINAL_AUDIT.md` with PASS/FAIL/NOT TESTED evidence, security risks, test commands and release blockers.
T70 Push final docs/report, verify clean git state and remote commit; deliver repo URL, branch, commit and exact readiness verdict.

## STOP policy
Stop only for defined material blockers or user-required permission, not for routine approvals. Never falsely claim an unrun macOS test. Never skip a failed task to make progress. After interruption resume using `TASK_PROGRESS.md` and git history.
