# Benchmark methodology

This is a shared parser workload comparison of the Python original and the standalone Rust port. Python is invoked only by this verification runner; the Rust crate and probe have no Python runtime dependency.

## Workloads

- literal grammar parse
- ordered-choice parse
- repetition parse
- compound negative-lookahead / choice / repetition parse
- failing parse that exercises the error path

## Sampling

- Build profile: Rust `--release`.
- Parser construction is outside steady-state timing for both implementations.
- Per case/language: 200 warmup parses, 15 samples, 1000 timed parses/sample.
- Latency percentiles are distributions of **per-operation sample means**; throughput uses all timed operations.
- Startup is wall-clock process launch plus minimal import/probe initialization across 12 samples, reported separately.
- Memory is best-effort Windows `WorkingSet64` sampling via `Get-Process` while a 2-second compound parsing loop runs. It is not a kernel peak-RSS counter.

The corpus is representative of the currently verified common parser subset, not a claim of full Python-suite parity.

## Extended memory check

The original benchmark result retains its 2-second maximum WorkingSet64 sample for reproducibility. A separate longer check in [memory_check.md](memory_check.md) and [memory_check.json](memory_check.json) runs the same compound grammar and input for 15 seconds per implementation. It records minimum, median, p95, p99, and maximum WorkingSet64 values. The configured sampling interval is 50 ms; because each sample invokes a PowerShell `Get-Process` query, the observed median interval was about 200 ms. This remains a best-effort Windows working-set measurement, not kernel peak RSS.
