# Submission manifest

## Identity and scope

- Track: **D — Python → Rust**
- Source project: **Parsimonious**
- Source repository: <https://github.com/erikrose/parsimonious.git>
- Public deliverable: standalone safe Rust crate in this directory
- One-command build: `cargo build --release`
- Docker verification: `docker build -t parsimonious-rs .`

The crate contains no Python dependency or runtime linkage. Python is used only by optional external verification runners. The Rust integration adapter is a five-test smoke subset, not literal execution of or full parity with the Python original suite.

## Included files

- `Cargo.toml`, `Cargo.lock`, `LICENSE`
- `src/`, including the safe `bench_probe` and `fuzz_probe` binaries
- `tests/original.rs` — five-test Rust smoke adapter
- `tests/original/` — six byte-identical pinned Python tests
- `README.md`, `DECISIONS.md`, `PROVENANCE.md`, `FINAL_VERIFICATION.md`, `DEMO_SCRIPT.md`
- `.port-mortem.toml`, `Dockerfile`, `.gitignore`
- `fuzz/harness.py`, `fuzz/log.txt`, `fuzz/cases.jsonl`
- `bench/run_bench.py`, `bench/methodology.md`, `bench/results.json`
- `bench/run_memory_check.py`, `bench/memory_check.md`, `bench/memory_check.json`
- `SUBMISSION_MANIFEST.md`

## Original-test integrity

The completed harness guard verified 6 pinned files with 0 violations. The public copies were rehashed after copying and match the authoritative `test_guard.pinned_hashes` values.

| Canonical pinned path | SHA-256 |
|---|---|
| `__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `benchmarks.py` | `0cb18347da7d70aae5d514d1c4af6c4cc6b69dfeb46f053d0f9613313152bbe8` |
| `test_benchmarks.py` | `d78747ac28a0c6cc284ab859edbd5f2519daf83998a27455c860460e879b6b06` |
| `test_expressions.py` | `28aab7fdb6514183e911413e89f4d2591744b097de7db60f399a0b6c04c7d47e` |
| `test_grammar.py` | `953b5fdf23a2ac860ba8dd86efeb0f753124f66d7ac5cb83c9ecfc8f77c40e49` |
| `test_nodes.py` | `4f933d0166615e4d9309919058ac92297da69f515d6ca39d7adb92be90a9039b` |

- `pinned_file_count = 6`
- `original_suite_aggregate_sha256 = 8bb30497b81224df1494f235a26979ef75be1edd93ce25608af9ce2fdc9bc495`

Aggregate algorithm: sort the canonical pinned paths lexicographically using forward slashes; concatenate `path + "\n" + sha256 + "\n"` for each entry; SHA-256 the resulting UTF-8 text.

## Recorded verification

- Release build: passed
- Library tests: 31 passed
- Rust integration smoke adapter: 5 passed
- Differential fuzz: 60.003 seconds, 9,848 cases, 0 divergences on the documented common subset
- Aggregate p99: Python 12,685.2 ns; Rust 7,937.8 ns; measured speedup 1.60x
- Throughput: Python 241,610 ops/s; Rust 437,921 ops/s
- Startup p50/p99: Python 58.33/61.44 ms; Rust 5.22/7.33 ms
- Original 2-second sampled working set: Python 16.07 MB; Rust 5.51 MB
- Extended 15-second sampled working set: Python 15.96 MB; Rust 5.29–6.00 MB
- Unsafe count: 0
- Python runtime linkage: none
- `DECISIONS.md`: 16 substantive entries

The submission does not claim a 10x speedup. Memory figures are best-effort Windows `Get-Process WorkingSet64` samples, not kernel peak RSS. The extended result states only that Rust measured lower under the recorded workload and method.

The public fuzz log's probe path was normalized from the original machine-specific absolute path to the equivalent package-relative `target\\release\\fuzz_probe.exe`; duration, counts, comparisons, and divergence results are unchanged.

## Generation provenance and commit guidance

[PROVENANCE.md](PROVENANCE.md) indexes the human-gated Claude Code, Codex, orchestrator, and packaging phases; the real state task timestamps; and the limitations of the public evidence. The recorded harness run bounds are kickoff `2026-08-01T17:25:37Z` and freeze `2026-08-03T18:00:00Z`.

The public repository should contain one normally created finalized commit made from this directory after kickoff and before freeze. No incremental history or timestamps should be fabricated, backdated, or rewritten. After committing, use `git show --format=fuller --no-patch HEAD` to display the real author and committer timestamps.

The original-test integrity remains:

- `pinned_file_count = 6`
- `original_suite_aggregate_sha256 = 8bb30497b81224df1494f235a26979ef75be1edd93ce25608af9ce2fdc9bc495`

Private harness state and raw model logs are excluded intentionally; their relevant task IDs, timestamp evidence, and derived decisions are summarized in the public documentation.

## Intentionally excluded

- `.env` and credentials
- `.harness/`, `.harness-demo/`, `.harness-factory/`
- state databases, private runtime logs, and model transcripts
- `target/` and other generated build outputs
- `wt-pm1-upstream-backup/`
- `__pycache__/`, `.pytest_cache/`, and bytecode caches

No Git repository was initialized and nothing was pushed by the packaging process.
