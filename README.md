# parsimonious-rs — Port Mortem Track D submission

This is a standalone Rust port of [Parsimonious](https://github.com/erikrose/parsimonious), a Python PEG parser library. It is a Track D Python → Rust submission. The Rust crate does not embed or link to Python; Python is used only as an external baseline by the optional fuzz and benchmark runners.

## One-command build

From this directory:

```bash
cargo build --release
```

The same Rust-only build and verification are available through one Docker command:

```bash
docker build -t parsimonious-rs .
```

Run the verified test commands with:

```bash
cargo test --lib
cargo test --test original
```

The final results are 31 passing library tests and 5 passing integration smoke tests.

## Frozen original tests

Six pinned, byte-identical Python original-test files are copied under [`tests/original`](tests/original). The final harness guard verified 6 files with 0 violations. Their deterministic suite hash is:

```text
original_suite_aggregate_sha256 = 8bb30497b81224df1494f235a26979ef75be1edd93ce25608af9ce2fdc9bc495
```

The aggregate uses the canonical paths stored by the test guard, sorted lexicographically. For each file it hashes `path + "\n" + sha256 + "\n"`. Individual hashes are recorded in [SUBMISSION_MANIFEST.md](SUBMISSION_MANIFEST.md) and [FINAL_VERIFICATION.md](FINAL_VERIFICATION.md).

[`tests/original.rs`](tests/original.rs) is the Cargo integration adapter. Its five tests mirror high-value original-suite themes: grammar construction, parse success/failure, incomplete parses, negative lookahead, optional/repetition, and the visitor/rule facade. It is a **smoke subset**, not literal execution of the full Python suite and not a claim of full Python-suite parity.

## Differential fuzz evidence

The external differential verifier ran the Python original and standalone Rust probe side by side for **60.003 seconds**, covering **9,848 cases** with **0 divergences**. It compared:

- parse success versus failure;
- recursive tree shape and spans;
- error type; and
- error position/offset.

The verified corpus is the documented common text-grammar subset: literals, ordered choice, negative lookahead, repetition, and simple character-class regex. Dynamic custom rules, token streams, bytes grammars, and Python-specific regex flags are outside this claim. See [`fuzz/log.txt`](fuzz/log.txt), [`fuzz/cases.jsonl`](fuzz/cases.jsonl), and [`fuzz/harness.py`](fuzz/harness.py).

## Benchmark

The shared benchmark covers literal, choice, repetition, compound lookahead/choice/repetition, and failure/error-position parses. Rust was built in release mode; both implementations used 200 warmups, 15 samples of 1,000 timed parses per case, and 12 startup samples.

| Metric | Python original | Rust port |
|---|---:|---:|
| Aggregate p99 parse latency | 12,685.2 ns | 7,937.8 ns |
| Throughput | 241,610 ops/s | 437,921 ops/s |
| Startup p50 / p99 | 58.33 / 61.44 ms | 5.22 / 7.33 ms |
| Original 2-second sampled working set | 16.07 MB | 5.51 MB |

The measured aggregate p99 speedup is **1.60x**. This submission does not claim a 10x speedup. Full methodology and raw values are in [`bench/methodology.md`](bench/methodology.md) and [`bench/results.json`](bench/results.json).

### Extended memory check

The same compound grammar (`root = !'b' ('a' / 'c')+`, input `acaca`) was run for 15 seconds per implementation and sampled with Windows `Get-Process WorkingSet64`.

| Working-set sample | Python original | Rust port |
|---|---:|---:|
| Minimum | 15.96 MB | 5.29 MB |
| Median (p50) | 15.96 MB | 6.00 MB |
| p95 / p99 / maximum | 15.96 MB | 6.00 MB |
| Samples | 75 | 75 |

Under this workload and method, Rust's sampled working set was lower. These are best-effort Windows working-set samples, **not kernel peak RSS**. The configured sampling interval was 50 ms; PowerShell query overhead resulted in an observed median interval of about 200 ms. See [`bench/memory_check.md`](bench/memory_check.md), [`bench/memory_check.json`](bench/memory_check.json), and the reproducible extension [`bench/run_memory_check.py`](bench/run_memory_check.py).

## Reproducing the verification evidence

The Rust crate builds and tests without Python. The commands below are optional evidence reruns: they invoke the Python original externally and automatically build the standalone Rust probes from `src/bin/`. Run them from this directory.

The verified Python baseline was the clean upstream checkout at commit:

```text
eb79639859a9697a86c0992a045174a8856b5fb0
```

Prepare that baseline, or point `$OriginalSource` at an existing matching checkout whose root contains the `parsimonious/` package:

```powershell
git clone https://github.com/erikrose/parsimonious.git ..\parsimonious-python
git -C ..\parsimonious-python checkout eb79639859a9697a86c0992a045174a8856b5fb0

$VerifierPython = 'python'
$OriginalSource = (Resolve-Path '..\parsimonious-python').Path
```

Use a Python environment in which the original project's dependencies are installed. The verification runners import the original directly from `$OriginalSource`; Python is not linked into the Rust crate.

### Latency, throughput, startup, and original memory sample

[`bench/run_bench.py`](bench/run_bench.py) runs the shared Python/Rust workload and writes a fresh methodology and results file:

```powershell
& $VerifierPython bench\run_bench.py `
  --rust-worktree . `
  --python-source $OriginalSource `
  --out-dir .\bench-rerun
```

Outputs:

- `bench-rerun/methodology.md`
- `bench-rerun/results.json`

### Extended Windows memory check

[`bench/run_memory_check.py`](bench/run_memory_check.py) uses the same compound workload and Windows `Get-Process WorkingSet64` method. The duration must be at least 10 seconds:

```powershell
& $VerifierPython bench\run_memory_check.py `
  --rust-worktree . `
  --python-source $OriginalSource `
  --out-dir .\memory-rerun `
  --duration 15 `
  --interval 0.05
```

Outputs:

- `memory-rerun/memory_check.md`
- `memory-rerun/memory_check.json`

This check is Windows-specific and remains sampled working-set evidence, not kernel peak RSS.

### Differential fuzz rerun

[`fuzz/harness.py`](fuzz/harness.py) runs the deterministic common-subset differential oracle. It enforces a minimum duration of 60 seconds:

```powershell
& $VerifierPython fuzz\harness.py `
  --rust-worktree . `
  --python-source $OriginalSource `
  --out-dir .\fuzz-rerun `
  --duration 60
```

Outputs:

- `fuzz-rerun/log.txt`
- `fuzz-rerun/cases.jsonl`
- `fuzz-rerun/repros/` if divergences are found

The separate `*-rerun` directories deliberately preserve the submitted evidence under `bench/` and `fuzz/`. For the complete non-expensive build/test and hash checklist, see [FINAL_VERIFICATION.md](FINAL_VERIFICATION.md).

## Safety and runtime boundary

- Unsafe count: **0**; [`src/lib.rs`](src/lib.rs) uses `#![forbid(unsafe_code)]`.
- No source-language runtime: **pass**; the shipping crate has no dependencies and no PyO3, CPython, or Python runtime linkage.
- Frozen-original guard: **6 pinned / 0 violations**.
- Final recorded budget: **$18.260540 / $200.00**.

## Package contents and limitations

This directory contains the Rust source, Cargo metadata, Dockerfile, frozen original tests and hashes, fuzz evidence, benchmark evidence, decision log, demo script, and final verification checklist. Private harness databases, model logs/transcripts, credentials, caches, and generated build outputs are deliberately excluded.

See [DECISIONS.md](DECISIONS.md) for 16 substantive engineering decisions, [SUBMISSION_MANIFEST.md](SUBMISSION_MANIFEST.md) for the package inventory, and [DEMO_SCRIPT.md](DEMO_SCRIPT.md) for the five-minute presentation flow.

This port was developed with an agent harness using configured coding-model backends. The public package contains summarized decisions and reproducible evidence, not private runtime state or transcripts.

See [PROVENANCE.md](PROVENANCE.md) for the generation/audit trail and why this submission uses one finalized commit rather than fabricated incremental history.
