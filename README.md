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

### How judges can verify the original tests

From the repository root, this PowerShell block hashes every public copy in `tests/original`, compares it with the six values pinned by the harness before porting, and recomputes the deterministic aggregate suite hash:

```powershell
$Expected = [ordered]@{
  '__init__.py'         = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
  'benchmarks.py'       = '0cb18347da7d70aae5d514d1c4af6c4cc6b69dfeb46f053d0f9613313152bbe8'
  'test_benchmarks.py'  = 'd78747ac28a0c6cc284ab859edbd5f2519daf83998a27455c860460e879b6b06'
  'test_expressions.py' = '28aab7fdb6514183e911413e89f4d2591744b097de7db60f399a0b6c04c7d47e'
  'test_grammar.py'     = '953b5fdf23a2ac860ba8dd86efeb0f753124f66d7ac5cb83c9ecfc8f77c40e49'
  'test_nodes.py'       = '4f933d0166615e4d9309919058ac92297da69f515d6ca39d7adb92be90a9039b'
}

$TestRoot = Join-Path (Get-Location) 'tests\original'
$Failed = $false
$Rows = foreach ($Name in $Expected.Keys) {
  $Actual = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $TestRoot $Name)).Hash.ToLowerInvariant()
  $MatchesPinned = $Actual -eq $Expected[$Name]
  if (-not $MatchesPinned) { $Failed = $true }
  [pscustomobject]@{ File = $Name; MatchesPinnedHash = $MatchesPinned; SHA256 = $Actual }
}
$Rows | Format-Table -AutoSize

$Material = -join ($Expected.Keys | Sort-Object | ForEach-Object {
  "$($_)`n$($Expected[$_])`n"
})
$Hasher = [System.Security.Cryptography.SHA256]::Create()
try {
  $Aggregate = [BitConverter]::ToString(
    $Hasher.ComputeHash([Text.Encoding]::UTF8.GetBytes($Material))
  ).Replace('-', '').ToLowerInvariant()
} finally {
  $Hasher.Dispose()
}

$ExpectedAggregate = '8bb30497b81224df1494f235a26979ef75be1edd93ce25608af9ce2fdc9bc495'
"Aggregate: $Aggregate"
"Aggregate matches: $($Aggregate -eq $ExpectedAggregate)"
if ($Failed -or $Aggregate -ne $ExpectedAggregate) {
  throw 'Original-test hash verification failed.'
}
```

Every `MatchesPinnedHash` value and `Aggregate matches` should print `True`. This verifies that the submitted copies still match the hashes captured before translation.

Judges can also compare those copies with the exact upstream Python baseline, commit `eb79639859a9697a86c0992a045174a8856b5fb0`. The authoritative hashes above cover the raw frozen Windows working-tree bytes, including CRLF line endings. Git may check out the upstream files with LF on Linux/macOS, so the cross-platform upstream comparison below normalizes only CRLF to LF before hashing; it does not otherwise alter the source. After preparing `$OriginalSource` using the [reproduction instructions](#reproducing-the-verification-evidence), run:

```powershell
$PackagedTests = Join-Path (Get-Location) 'tests\original'
$UpstreamTests = Join-Path $OriginalSource 'parsimonious\tests'
$Names = @('__init__.py', 'benchmarks.py', 'test_benchmarks.py',
           'test_expressions.py', 'test_grammar.py', 'test_nodes.py')
$Mismatch = $false

function Get-NormalizedSourceHash([string]$Path) {
  $Text = [Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes($Path))
  $Bytes = [Text.Encoding]::UTF8.GetBytes($Text.Replace("`r`n", "`n"))
  $Hasher = [System.Security.Cryptography.SHA256]::Create()
  try {
    return [BitConverter]::ToString($Hasher.ComputeHash($Bytes)).Replace('-', '').ToLowerInvariant()
  } finally {
    $Hasher.Dispose()
  }
}

foreach ($Name in $Names) {
  $PackagedHash = Get-NormalizedSourceHash (Join-Path $PackagedTests $Name)
  $UpstreamHash = Get-NormalizedSourceHash (Join-Path $UpstreamTests $Name)
  $Same = $PackagedHash -eq $UpstreamHash
  [pscustomobject]@{ File = $Name; MatchesUpstreamAfterEolNormalization = $Same }
  if (-not $Same) { $Mismatch = $true }
}

if ($Mismatch) { throw 'A packaged original test differs from upstream.' }
```

All six `MatchesUpstreamAfterEolNormalization` values should print `True`. These Python files are integrity evidence; Cargo executes the separate Rust smoke adapter at `tests/original.rs`.

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

This directory contains the Rust source, Cargo metadata, Dockerfile, frozen original tests and hashes, fuzz evidence, benchmark evidence, decision log, provenance index, and final verification checklist. Private harness databases, model logs/transcripts, credentials, caches, generated build outputs, and operator-only presentation notes are deliberately excluded.

See [DECISIONS.md](DECISIONS.md) for 16 substantive engineering decisions and [SUBMISSION_MANIFEST.md](SUBMISSION_MANIFEST.md) for the package inventory.

This port was developed with an agent harness using configured coding-model backends. The public package contains summarized decisions and reproducible evidence, not private runtime state or transcripts.

See [PROVENANCE.md](PROVENANCE.md) for the generation/audit trail and why this submission uses one finalized commit rather than fabricated incremental history.
