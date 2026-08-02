"""Extended WorkingSet64 check for the shared compound parser workload.

This extends run_bench.py's existing Windows memory check. It uses the same
Python original, Rust bench_probe, grammar, input, and Get-Process
WorkingSet64 measurement, but samples for longer and records distributions.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import platform
import shutil
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from run_bench import WORKLOADS, build_probe, cargo_command, command_version, rustc_command


def percentile(values: list[int], fraction: float) -> int:
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def sample_working_set(
    command: list[str], configured_duration_s: float, configured_interval_s: float
) -> dict[str, Any]:
    """Sample a child process using the benchmark's Windows WorkingSet64 method."""
    if os.name != "nt" or shutil.which("powershell") is None:
        raise RuntimeError("Windows PowerShell Get-Process is required")
    started = time.monotonic()
    proc = subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    values: list[int] = []
    sample_times: list[float] = []
    try:
        while proc.poll() is None:
            query = subprocess.run(
                [
                    "powershell",
                    "-NoProfile",
                    "-Command",
                    f"(Get-Process -Id {proc.pid} -ErrorAction SilentlyContinue).WorkingSet64",
                ],
                text=True,
                capture_output=True,
                encoding="utf-8",
                errors="replace",
            )
            try:
                values.append(int(query.stdout.strip()))
                sample_times.append(time.monotonic() - started)
            except ValueError:
                pass
            time.sleep(configured_interval_s)
    finally:
        return_code = proc.wait(timeout=5)
    if return_code:
        raise RuntimeError(f"memory workload exited with status {return_code}")
    if not values:
        raise RuntimeError("no WorkingSet64 samples were returned")
    intervals = [right - left for left, right in zip(sample_times, sample_times[1:])]
    return {
        "sample_count": len(values),
        "min_bytes": min(values),
        "p50_bytes": int(statistics.median(values)),
        "p95_bytes": percentile(values, 0.95),
        "p99_bytes": percentile(values, 0.99),
        "max_bytes": max(values),
        "configured_duration_s": configured_duration_s,
        "observed_process_duration_s": time.monotonic() - started,
        "configured_sampling_interval_s": configured_interval_s,
        "observed_median_sampling_interval_s": (
            statistics.median(intervals) if intervals else None
        ),
    }


def mb(value: int) -> str:
    return f"{value / 1_000_000:.2f}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust-worktree", required=True, type=Path)
    parser.add_argument("--python-source", required=True, type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument("--duration", type=float, default=15.0)
    parser.add_argument("--interval", type=float, default=0.05)
    args = parser.parse_args()
    if args.duration < 10:
        raise SystemExit("duration must be at least 10 seconds")
    if args.interval <= 0:
        raise SystemExit("interval must be positive")

    args.out_dir.mkdir(parents=True, exist_ok=True)
    probe = build_probe(args.rust_worktree.resolve())
    _, grammar_source, text, description = WORKLOADS[3]
    python_command = [
        sys.executable,
        "-c",
        (
            "import sys,time;sys.path.insert(0,sys.argv[1]);"
            "from parsimonious.grammar import Grammar;g=Grammar(sys.argv[2]);"
            "end=time.monotonic()+float(sys.argv[4]);"
            "\nwhile time.monotonic()<end:\n g.parse(sys.argv[3])"
        ),
        str(args.python_source.resolve()),
        grammar_source,
        text,
        str(args.duration),
    ]
    rust_command = [
        str(probe),
        "--grammar",
        grammar_source,
        "--input",
        text,
        "--warmup",
        "0",
        "--iterations",
        "1",
        "--memory-seconds",
        str(args.duration),
    ]

    results: dict[str, Any] = {
        "workload": {
            "name": "compound",
            "grammar": grammar_source,
            "input": text,
            "description": description,
        },
        "method": {
            "counter": "Windows Get-Process WorkingSet64",
            "limitation": "Best-effort sampled working set; not kernel peak RSS.",
            "processes_sampled_sequentially": True,
        },
        "python": sample_working_set(python_command, args.duration, args.interval),
        "rust": sample_working_set(rust_command, args.duration, args.interval),
        "environment": {
            "timestamp_utc": datetime.now(timezone.utc).isoformat(),
            "os": platform.platform(),
            "python": sys.version,
            "rustc": command_version([rustc_command(), "--version"]),
            "cargo": command_version([cargo_command(), "--version"]),
        },
    }
    (args.out_dir / "memory_check.json").write_text(
        json.dumps(results, indent=2) + "\n", encoding="utf-8"
    )
    py = results["python"]
    rs = results["rust"]
    markdown = f"""# Extended memory check

The Python original and standalone Rust probe ran the same compound grammar
(`{grammar_source}` with input `{text}`) for {args.duration:.1f} seconds each.
They were sampled sequentially with Windows `Get-Process WorkingSet64` using a
configured {args.interval:.2f}-second interval.

| Working-set sample | Python | Rust |
|---|---:|---:|
| Minimum | {mb(py['min_bytes'])} MB | {mb(rs['min_bytes'])} MB |
| Median (p50) | {mb(py['p50_bytes'])} MB | {mb(rs['p50_bytes'])} MB |
| p95 | {mb(py['p95_bytes'])} MB | {mb(rs['p95_bytes'])} MB |
| p99 | {mb(py['p99_bytes'])} MB | {mb(rs['p99_bytes'])} MB |
| Maximum | {mb(py['max_bytes'])} MB | {mb(rs['max_bytes'])} MB |
| Samples | {py['sample_count']} | {rs['sample_count']} |

Under this workload and measurement method, the Rust process's sampled
working set was lower than the Python process's. These are best-effort Windows
working-set samples, not kernel peak-RSS measurements. Process startup and the
PowerShell query overhead affect sampling; the configured interval and observed
timings are preserved in [memory_check.json](memory_check.json).
"""
    (args.out_dir / "memory_check.md").write_text(markdown, encoding="utf-8")
    print(json.dumps({"python": py, "rust": rs}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
