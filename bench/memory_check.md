# Extended memory check

The Python original and standalone Rust probe ran the same compound grammar
(`root = !'b' ('a' / 'c')+` with input `acaca`) for 15.0 seconds each.
They were sampled sequentially with Windows `Get-Process WorkingSet64` using a
configured 0.05-second interval.

| Working-set sample | Python | Rust |
|---|---:|---:|
| Minimum | 15.96 MB | 5.29 MB |
| Median (p50) | 15.96 MB | 6.00 MB |
| p95 | 15.96 MB | 6.00 MB |
| p99 | 15.96 MB | 6.00 MB |
| Maximum | 15.96 MB | 6.00 MB |
| Samples | 75 | 75 |

Under this workload and measurement method, the Rust process's sampled
working set was lower than the Python process's. These are best-effort Windows
working-set samples, not kernel peak-RSS measurements. Process startup and the
PowerShell query overhead affect sampling; the configured interval and observed
timings are preserved in [memory_check.json](memory_check.json).
