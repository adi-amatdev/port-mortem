# Generation provenance

## Summary

This judge-facing package is intended to be submitted as one honest finalized commit. The work was generated through human-gated harness and AI-agent phases; no incremental Git history, commit timestamp, or session timestamp has been invented to make that process look manual or sequential. The public evidence combines state-derived task IDs and UTC timestamps, timestamped benchmark artifacts, frozen-test hashes, verification documents, and summarized private-log metadata. Raw private state and large transcripts are not published.

The completed harness state records kickoff `2026-08-01T17:25:37Z` and freeze `2026-08-03T18:00:00Z`. The final commit does not exist yet in this directory and therefore has no timestamp to report. It must be created normally after kickoff and before freeze without overriding Git's author or committer dates.

## Timeline / phase index

| Phase | Agent/tool | Scope | Evidence/artifacts | Timestamp evidence available | Notes/limitations |
|---|---|---|---|---|---|
| 1. Claude Code harness bootstrap | Claude Code; human-gated bootstrap | Built the Step 0 harness/orchestrator from `COOKBOOK.md`: SQLite state, graph engine, router, guardrails, spawn layer, and initial factory. Stopped before full translation. | Source-workspace `HARNESS.md` and `harness_selftest.py`; subsequent state/log artifacts. `HARNESS.md` SHA-256: `b3c72856b959b65c05189f1283b7b6244bea54e44e0dd13ea4d4bdca84f8bae6`; `harness_selftest.py` SHA-256: `4354b08870ad7d2b091f02ceb2d9417e938f1db0e91b3a5eb8accbb8aa79df07`. | Filesystem mtimes: `HARNESS.md` `2026-08-01T17:48:25.4583181Z`; `harness_selftest.py` `2026-08-01T20:04:57.7093672Z`. | Raw Claude bootstrap transcript is not included; these mtimes are file metadata, not exact session bounds. Derived evidence is the harness files and subsequent state/log artifacts. |
| 2. Codex translation and harness repair | Codex CLI plus harness-dispatched workers | Picked up the harness; fixed the `evaluate_string` false-green/stub check; promoted `utils` to `SEMANTIC_CRITICAL`; repaired the expressions/grammar/nodes SCC continuation; fixed `__init__.py`/public-facade completeness. | [DECISIONS.md](DECISIONS.md) decisions 5–10; [FINAL_VERIFICATION.md](FINAL_VERIFICATION.md); private log basenames `codex_translate_parsimonious_exceptions_779efd2e.jsonl`, `codex_translate_parsimonious_expressions_76a031b8.jsonl`, `codex_translate_parsimonious_expressions_810e7780.jsonl`, `codex_fix_parsimonious_expressions_002fafc1.jsonl`, and `codex_translate_parsimonious_init_ceb5992d.jsonl`. | Private log file mtimes span `2026-08-01T18:12:52.5678948Z` through `2026-08-01T19:17:42.7420599Z`; state-derived decision records begin at `2026-08-01T17:34:21Z`. | Exact raw interactive-session timestamps are unavailable in public deliverables; phase ordering is supported by state task IDs, artifact mtimes, and generated verification artifacts. Private log contents are not published. |
| 3. Codex integration, fuzz, and benchmark | Codex CLI plus deterministic orchestrator commands | Corrected IntegrationTest workdir/log capture, added the five-test smoke adapter, passed integration, built and ran DiffFuzz, and ran the shared benchmark. | Integration `integration:run:5cfa1daa`; fuzz `fuzz:run:b6128d5a`; benchmark `benchmark:run:aa891ae5`; [fuzz/log.txt](fuzz/log.txt); [fuzz/cases.jsonl](fuzz/cases.jsonl); [bench/results.json](bench/results.json); [bench/methodology.md](bench/methodology.md). | Integration `2026-08-01T20:07:01Z`–`20:07:03Z`; fuzz `2026-08-02T08:48:15Z`–`08:49:15Z`; benchmark `2026-08-02T08:57:21Z`–`08:57:27Z`. `bench/results.json` embeds `2026-08-02T08:57:27.713104+00:00`. | The fuzz log records duration and results but no wall-clock field; its exact run bounds come from the state task. The smoke adapter is not full Python-suite parity, and fuzzing covers only the documented common subset. |
| 4. Codex final packaging and QA | Codex CLI; local Cargo/Python/PowerShell verification | Finalized README, decisions, manifest/config, Dockerfile, verification and demo docs; created the clean deliverables package; extracted original-test hashes; ran the extended memory check and final Cargo QA. | [FINAL_VERIFICATION.md](FINAL_VERIFICATION.md); [SUBMISSION_MANIFEST.md](SUBMISSION_MANIFEST.md); [bench/memory_check.md](bench/memory_check.md); [bench/memory_check.json](bench/memory_check.json); original-suite aggregate `8bb30497b81224df1494f235a26979ef75be1edd93ce25608af9ce2fdc9bc495`. | `memory_check.json` embeds `2026-08-02T10:29:52.326752+00:00`; packaged `FINAL_VERIFICATION.md` has filesystem mtime `2026-08-02T10:32:28.0280551Z`. | Exact raw interactive-session timestamp is unavailable in public deliverables; phase ordering is supported by artifact mtimes and generated verification artifacts. Build/test results are recorded, while generated `target/` output is excluded. |
| 5. Human final commit, demo, and submission | Human operator | Supplied gated instructions between phases; reviews the package; creates one final public commit; records the demo using operator-only notes retained outside this repository; pushes and submits. | This file and the future real Git commit metadata. | No commit timestamp exists yet. It must be created after `2026-08-01T17:25:37Z` and before `2026-08-03T18:00:00Z`. | The human did not fabricate incremental commits. Historical blocked/failed tasks remain summarized in [DECISIONS.md](DECISIONS.md). |

## Agent/tool disclosure

- **Claude Code:** used for the initial harness bootstrap and harness-dispatched work. Private log metadata records `claude-sonnet-5` and `claude-opus-5`; state task labels also include `sonnet`, `opus`, and `claude-fable-5`.
- **Codex CLI:** used for translation/harness repair, integration/fuzz/benchmark work, and final packaging/QA. GPT-5.6 Terra/Sol were configured Codex tiers in the harness routing documentation, but the public artifacts do not establish the exact tier used for each interactive Codex session; no per-session tier is inferred here.
- **Opencode GO / MiMo-V2.5:** the state database records two `mimo-v2.5` translation tasks, and the harness router maps that tier to Opencode GO. DeepSeek V4 Flash was configured as a fallback, but the available task state does not prove that it executed, so this document does not claim it did.
- **Deterministic local tools:** Cargo, Rust probes, Python verification runners, PowerShell `Get-Process`, SQLite state queries, hashing, and filesystem/package checks.
- **Human operator:** provided gated instructions, accepted no fabricated history, and retains responsibility for the final commit, demo recording, push, and submission form.

## Evidence index

- [DECISIONS.md](DECISIONS.md) — 16 substantive decisions and recovery history
- [FINAL_VERIFICATION.md](FINAL_VERIFICATION.md) — final commands, guards, and per-file original-test hashes
- [SUBMISSION_MANIFEST.md](SUBMISSION_MANIFEST.md) — public package contents and exclusions
- [fuzz/log.txt](fuzz/log.txt) — 60.003-second, 9,848-case, zero-divergence summary
- [fuzz/cases.jsonl](fuzz/cases.jsonl) — recorded differential cases
- [bench/methodology.md](bench/methodology.md) — shared benchmark and memory methodology
- [bench/results.json](bench/results.json) — timestamped benchmark environment and results
- [bench/memory_check.md](bench/memory_check.md) — extended memory-check summary
- [bench/memory_check.json](bench/memory_check.json) — timestamped extended memory data
- [.port-mortem.toml](.port-mortem.toml) — track, source, guards, task IDs, budget, and aggregate test hash
- [README.md](README.md) — build path, results, scope, and limitations

Original-test integrity: `pinned_file_count = 6`; `original_suite_aggregate_sha256 = 8bb30497b81224df1494f235a26979ef75be1edd93ce25608af9ce2fdc9bc495`.

## What is not included

- no `.env` file or credentials;
- no `.harness` state database;
- no model API keys or authentication tokens;
- no giant raw private model transcripts or internal log corpus;
- no generated Cargo `target/` directory;
- no fabricated, backdated, or rewritten Git history.

The private evidence remains with the operator for audit if requested. Public documentation exposes only the minimum task identifiers, hashes, timestamps, and limitations needed to understand the generation process.

## Commit instruction

Create the final public commit from this deliverables directory after kickoff. Do not rewrite history to simulate incremental work.

Use Git normally without setting `GIT_AUTHOR_DATE`, `GIT_COMMITTER_DATE`, or equivalent overrides. After committing, verify the real timestamp with:

```bash
git show --format=fuller --no-patch HEAD
```
