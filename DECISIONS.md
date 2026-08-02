# Decisions and audit trail

This public submission summary is derived from the completed private harness state and the saved public verification artifacts. Runtime state databases, model transcripts, and internal logs are intentionally excluded from this judge-facing package; this document preserves the substantive decisions without exposing those internals.

## Porting strategy

The deliverable is this standalone, safe Rust parser crate. The harness treated the Python source and frozen original tests as verification inputs, not as runtime dependencies. Progress and continuation eligibility came from the state database and guardrail artifacts rather than a claimed status in documentation.

## Generation provenance and commit history

This submission intentionally uses one finalized public commit rather than a fabricated sequence of incremental commits. The harness and AI agents produced the port through human-gated phases during the hackathon window; the public audit trail is the state-derived task identifiers, timestamped verification artifacts, this decision log, and [PROVENANCE.md](PROVENANCE.md). Git history was not backfilled, rewritten, or assigned invented dates to simulate a development sequence.

The completed harness state records a run kickoff of `2026-08-01T17:25:37Z` and a freeze of `2026-08-03T18:00:00Z`. The final public commit must be created normally from this deliverables directory after that kickoff and before the freeze; no author or committer date should be overridden. Until that commit is made, the public artifacts prove generation ordering and verification, not a commit timestamp that does not yet exist.

Claude Code bootstrapped the Step 0 harness/orchestrator from `COOKBOOK.md`, including SQLite state, the graph engine, router, guardrails, spawn layer, and initial factory, then stopped before full translation. Codex continued through translation and harness repair, integration/fuzz/benchmark, and final packaging/QA. Human instructions gated those transitions. Exact raw interactive-session start/end timestamps are not available in this public directory; phase ordering is supported by state task IDs, artifact timestamps, source-file mtimes, and generated verification artifacts. Raw private transcripts, state databases, credentials, and large model logs are intentionally not published.

Historical blocked and failed attempts remain summarized in substantive decisions 9, 11, and 15 rather than being hidden. [FINAL_VERIFICATION.md](FINAL_VERIFICATION.md) records the final passing state, while [PROVENANCE.md](PROVENANCE.md) indexes the available generation evidence and its limitations.

## Substantive decisions

1. **Ship a Rust-only crate.** The crate has no Python interpreter, PyO3, or CPython linkage. Python is used only by the external differential-fuzz and benchmark baseline runners; the no-source-runtime guard passed on the final run.

2. **Forbid unsafe code.** `src/lib.rs` sets `#![forbid(unsafe_code)]`; the final unsafe guard passed with an unsafe count of zero.

3. **Freeze the original tests before work.** The harness pins six `tests/original` files. The final test-hash guard recorded six pinned files and zero violations. Byte-identical public copies and their hashes are retained at `tests/original`.

4. **Treat harness state as the audit authority.** Module readiness, guard outcomes, spend, tasks, results, decisions, and events were read from the private harness state database. Public documentation records those results; it does not replace them.

5. **Reject semantic stubs.** A previous `utils.evaluate_string` stub produced a false green. Completeness checking was strengthened so stubs cannot satisfy a module verification result, and `utils` was reverified as semantic-critical work.

6. **Promote host-language-sensitive work.** `utils` was promoted from a leaf to `SEMANTIC_CRITICAL` because Python evaluation/escaping behavior had to be reimplemented rather than delegated to a source runtime.

7. **Translate the expressions/grammar/nodes cycle atomically.** The source graph identified an import SCC involving `expressions`, `grammar`, and `nodes`; it was handled as one pinned phase so the Rust crate would not be validated as a partial cycle.

8. **Record structural rather than fictional one-to-one mappings.** Where Python classes such as mixins, token-specific classes, or visitor helpers had no direct Rust type, completeness recorded the divergence and verification focused on the replacement behavior instead of inventing a nominal copy.

9. **Use audited continuation after the SCC retry cap.** A blocked Fix task remains preserved. A targeted repair changed the completeness signature and a fresh verification task was created with the reason and affected files recorded in state.

10. **Repair the public facade after an init false green.** Initial completeness ignored `__init__.py` re-exports. The check was repaired, the Rust crate-root facade was corrected, and a guarded continuation reverified it; the earlier state remains visible as historical evidence.

11. **Recover integration without overwriting prior attempts.** Earlier blocked and failed IntegrationTest tasks, including the wrong-workdir Cargo failure, were preserved. `continue-integration` checked all module/guard prerequisites and created a fresh task; integration subsequently ran from the state-selected Rust worktree.

12. **Describe integration accurately.** `cargo test --test original` passes five Rust smoke-mirror tests. It exercises grammar construction, success/failure, incomplete parses, lookahead, optional/repetition, and the visitor/rule facade. It is not literal execution of, or full parity with, the Python original suite.

13. **Use Python only as an external differential oracle.** DiffFuzz compared shared text-grammar cases for success/failure, recursive tree shape/spans, error type, and error offset. The clean result is 60.003 seconds, 9,848 cases, and zero divergences, limited to the documented common subset.

14. **Benchmark a shared multi-case workload and state measurement limits.** The comparison includes literal, choice, repetition, compound lookahead/choice/repetition, and failure cases with identical warmup and sample settings. Windows memory uses sampled `Get-Process WorkingSet64`, not kernel peak RSS; the measured aggregate p99 ratio is 1.60x, not a 10x claim.

15. **Keep failures as engineering evidence.** State retains three blocked and two failed integration tasks, two failed fuzz tasks, blocked Fix tasks, and related events. The final passing tasks do not delete those records; they show the recovery path and its guardrails.

16. **Submit one honest finalized commit rather than fabricated history.** The harness and agents produced the port through gated phases, and judges clarified that generated work does not require invented incremental commits. The submission therefore uses one normal post-kickoff commit, with provenance recorded in [PROVENANCE.md](PROVENANCE.md), [FINAL_VERIFICATION.md](FINAL_VERIFICATION.md), [SUBMISSION_MANIFEST.md](SUBMISSION_MANIFEST.md), the fuzz/benchmark artifacts, and this decision log. No timestamp or commit sequence is backdated or synthesized.

## Final state references

- Integration: `integration:run:5cfa1daa`, five smoke tests passed.
- Differential fuzz: `fuzz:run:b6128d5a`, 9,848 cases and zero divergences.
- Benchmark: `benchmark:run:aa891ae5`, documented in [bench/results.json](bench/results.json).
- Final DecisionLog: one passed `decision` task; private runtime state and logs are intentionally excluded from this package.
- Generation audit index: [PROVENANCE.md](PROVENANCE.md).
