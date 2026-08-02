# Final verification checklist

Run from this directory. The Rust build and tests require no Python runtime.

```bash
cargo build --release
cargo test --lib
cargo test --test original
```

Recorded final results:

- release build: passed;
- library tests: 31 passed, 0 failed;
- integration adapter: 5 passed, 0 failed (smoke subset, not full Python-suite parity);
- unsafe guard: passed, zero unsafe constructs;
- no-source-runtime guard: passed, no Python runtime linkage; and
- original-test guard: 6 pinned files verified, 0 violations.

## Frozen original-test hashes

The authoritative `test_guard.pinned_hashes` record was read from the completed harness state in SQLite read-only mode. Every copied file was rehashed and matched its pinned SHA-256.

| Canonical pinned path | Public copy | SHA-256 |
|---|---|---|
| `__init__.py` | `tests/original/__init__.py` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `benchmarks.py` | `tests/original/benchmarks.py` | `0cb18347da7d70aae5d514d1c4af6c4cc6b69dfeb46f053d0f9613313152bbe8` |
| `test_benchmarks.py` | `tests/original/test_benchmarks.py` | `d78747ac28a0c6cc284ab859edbd5f2519daf83998a27455c860460e879b6b06` |
| `test_expressions.py` | `tests/original/test_expressions.py` | `28aab7fdb6514183e911413e89f4d2591744b097de7db60f399a0b6c04c7d47e` |
| `test_grammar.py` | `tests/original/test_grammar.py` | `953b5fdf23a2ac860ba8dd86efeb0f753124f66d7ac5cb83c9ecfc8f77c40e49` |
| `test_nodes.py` | `tests/original/test_nodes.py` | `4f933d0166615e4d9309919058ac92297da69f515d6ca39d7adb92be90a9039b` |

Pinned file count: **6**.

Aggregate suite SHA-256:

```text
8bb30497b81224df1494f235a26979ef75be1edd93ce25608af9ce2fdc9bc495
```

Aggregate algorithm: sort the canonical pinned paths lexicographically using forward slashes; concatenate `path + "\n" + sha256 + "\n"` for each entry; SHA-256 the resulting UTF-8 text.

## Evidence checklist

- [x] [`fuzz/log.txt`](fuzz/log.txt): 60.003 seconds, 9,848 cases, zero divergences, documented common subset.
- [x] [`fuzz/cases.jsonl`](fuzz/cases.jsonl): 9,848 recorded cases.
- [x] [`bench/results.json`](bench/results.json): 1.60x aggregate p99 result and original 2-second working-set samples.
- [x] [`bench/methodology.md`](bench/methodology.md): workload, sampling, and Windows limitation.
- [x] [`bench/memory_check.md`](bench/memory_check.md) and [`bench/memory_check.json`](bench/memory_check.json): extended 15-second WorkingSet64 distributions.
- [x] README and demo identify the Rust adapter as a five-test smoke subset.
- [x] README and fuzz log limit DiffFuzz to the documented common subset.
- [x] README makes no 10x performance claim.
- [x] No `.env`, `.harness*`, `target`, `__pycache__`, state database, or model transcript belongs in the public package.
