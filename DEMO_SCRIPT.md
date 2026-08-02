# Five-minute demo script

## 0:00–0:30 — scope and build path

Open [README.md](README.md). State that this is a Track D Python → Rust port of Parsimonious and that the deliverable is the standalone crate in this directory. Run:

```bash
cargo build --release
```

The Rust crate has no Python runtime dependency.

## 0:30–1:00 — frozen originals and hashes

Open [SUBMISSION_MANIFEST.md](SUBMISSION_MANIFEST.md) and `tests/original`. Show the six copied Python original-test files, their individual SHA-256 values, the aggregate hash, and the recorded 6/6 guard result with zero violations.

## 1:00–2:00 — Rust tests and honest scope

```bash
cargo test --lib
cargo test --test original
```

Show 31 library tests and 5 integration tests. Point to `tests/original.rs`: the Rust adapter covers grammar construction, success/failure, incomplete parse, lookahead, optional/repetition, and the visitor/rule facade. Say explicitly that it is a five-test smoke mirror, not the complete Python suite.

## 2:00–3:00 — differential fuzz evidence

Open [fuzz/log.txt](fuzz/log.txt). Show the 60.003-second, 9,848-case clean run and the compared fields: success/failure, recursive tree shape/spans, error type, and offset. State that the corpus is the documented common text-grammar subset. Show [fuzz/harness.py](fuzz/harness.py) only to demonstrate that Python is an external oracle, not a crate dependency.

## 3:00–4:15 — performance and memory evidence

Open [bench/results.json](bench/results.json) and [bench/methodology.md](bench/methodology.md). Report 12,685.2 ns Python versus 7,937.8 ns Rust aggregate p99 (1.60x), throughput of 241,610 versus 437,921 ops/s, and startup p50/p99 of 58.33/61.44 ms versus 5.22/7.33 ms.

Then open [bench/memory_check.md](bench/memory_check.md). The extended 15-second compound workload sampled Python at 15.96 MB and Rust at 5.29–6.00 MB. State that these are Windows `WorkingSet64` samples—not kernel peak RSS—and that the result applies only to this workload and method.

## 4:15–5:00 — decisions, safety, and limitations

Show `#![forbid(unsafe_code)]` in `src/lib.rs`, then open [DECISIONS.md](DECISIONS.md) and [PROVENANCE.md](PROVENANCE.md). Close with: unsafe count zero, no Python linkage, 16 substantive decisions, one honest post-kickoff commit rather than fabricated history, frozen originals untouched, smoke-adapter scope, fuzz-subset scope, and a measured 1.60x p99 speedup rather than a 10x claim.
