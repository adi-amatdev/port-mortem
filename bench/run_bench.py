"""Shared, verification-only benchmark runner for Python Parsimonious and Rust port.

The Python original is an external baseline only. The release-mode Rust probe
is standalone and has no Python linkage. Parser construction is excluded from
the steady-state latency loop and measured separately through process startup.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import platform
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


WORKLOADS = [
    ("literal", "root = 'a'", "a", "simple literal parse"),
    ("choice", "root = 'a' / 'b'", "b", "ordered choice second branch"),
    ("repetition", "root = 'a'+", "aaaa", "one-or-more repetition"),
    ("compound", "root = !'b' ('a' / 'c')+", "acaca", "lookahead plus nested choice/repetition"),
    ("failure", "root = 'a' 'b'", "ax", "parse failure and error-position path"),
]
WARMUP = 200
ITERATIONS = 1_000
SAMPLES = 15
STARTUP_SAMPLES = 12
MEMORY_SECONDS = 2.0


def cargo_command() -> str:
    found = shutil.which("cargo")
    if found:
        return found
    candidate = Path(os.environ.get("USERPROFILE", "")) / ".cargo" / "bin" / "cargo.exe"
    if os.name == "nt" and candidate.is_file():
        return str(candidate)
    raise RuntimeError("Cargo not found on PATH or under USERPROFILE")


def rustc_command() -> str:
    found = shutil.which("rustc")
    if found:
        return found
    cargo = Path(cargo_command())
    candidate = cargo.with_name("rustc.exe" if os.name == "nt" else "rustc")
    return str(candidate) if candidate.is_file() else "rustc"


def command_version(argv: list[str]) -> str:
    try:
        return subprocess.run(argv, text=True, capture_output=True, encoding="utf-8",
                              errors="replace", check=False).stdout.strip()
    except OSError as exc:
        return f"unavailable: {exc}"


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return float("nan")
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def stats(samples_ns: list[float], total_ops: int) -> dict[str, float]:
    total_ns = sum(samples_ns) * ITERATIONS
    return {
        "p50_ns": percentile(samples_ns, 0.50),
        "p95_ns": percentile(samples_ns, 0.95),
        "p99_ns": percentile(samples_ns, 0.99),
        "mean_ns": sum(samples_ns) / len(samples_ns),
        "throughput_ops_s": total_ops / (total_ns / 1_000_000_000),
    }


def python_parser(source: Path, grammar_source: str):
    sys.path.insert(0, str(source))
    try:
        from parsimonious.grammar import Grammar  # type: ignore[import-not-found]
        return Grammar(grammar_source)
    finally:
        sys.path.remove(str(source))


def sample_python(source: Path, grammar_source: str, text: str) -> tuple[float, int, int]:
    grammar = python_parser(source, grammar_source)
    for _ in range(WARMUP):
        try:
            grammar.parse(text)
        except Exception:
            pass
    started = time.perf_counter_ns()
    ok = failed = 0
    for _ in range(ITERATIONS):
        try:
            grammar.parse(text)
            ok += 1
        except Exception:
            failed += 1
    return (time.perf_counter_ns() - started) / ITERATIONS, ok, failed


def sample_rust(probe: Path, grammar_source: str, text: str) -> tuple[float, int, int]:
    proc = subprocess.run(
        [str(probe), "--grammar", grammar_source, "--input", text,
         "--warmup", str(WARMUP), "--iterations", str(ITERATIONS)],
        text=True, capture_output=True, encoding="utf-8", errors="replace",
    )
    if proc.returncode:
        raise RuntimeError(f"Rust probe failed: {proc.stderr}\n{proc.stdout}")
    parts = proc.stdout.strip().split("|")
    if len(parts) != 4 or parts[0] != "RESULT":
        raise RuntimeError(f"Rust probe protocol error: {proc.stdout!r}")
    return int(parts[1]) / ITERATIONS, int(parts[2]), int(parts[3])


def startup_samples(command: list[str]) -> list[float]:
    values = []
    for _ in range(STARTUP_SAMPLES):
        started = time.perf_counter_ns()
        proc = subprocess.run(command, text=True, capture_output=True, encoding="utf-8",
                              errors="replace")
        if proc.returncode:
            raise RuntimeError(f"startup command failed: {proc.stderr}")
        values.append((time.perf_counter_ns() - started) / 1_000_000)
    return values


def sampled_working_set(command: list[str]) -> tuple[int | None, str]:
    """Best-effort Windows WorkingSet64 sampling, not a kernel peak counter."""
    if os.name != "nt" or shutil.which("powershell") is None:
        return None, "RSS unavailable: Windows PowerShell Get-Process was not available"
    proc = subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    values: list[int] = []
    try:
        while proc.poll() is None:
            query = subprocess.run(
                ["powershell", "-NoProfile", "-Command",
                 f"(Get-Process -Id {proc.pid} -ErrorAction SilentlyContinue).WorkingSet64"],
                text=True, capture_output=True, encoding="utf-8", errors="replace",
            )
            try:
                values.append(int(query.stdout.strip()))
            except ValueError:
                pass
            time.sleep(0.05)
    finally:
        proc.wait(timeout=5)
    if not values:
        return None, "RSS unavailable: no WorkingSet64 samples were returned"
    return max(values), "sampled WorkingSet64 via PowerShell Get-Process during a 2s compound parse loop"


def build_probe(worktree: Path) -> Path:
    cargo = cargo_command()
    proc = subprocess.run([cargo, "build", "--release", "--bin", "bench_probe"], cwd=worktree,
                          text=True, capture_output=True, encoding="utf-8", errors="replace")
    if proc.returncode:
        raise RuntimeError(f"bench probe build failed:\n{proc.stdout}\n{proc.stderr}")
    name = "bench_probe.exe" if os.name == "nt" else "bench_probe"
    probe = worktree / "target" / "release" / name
    if not probe.is_file():
        raise RuntimeError(f"bench probe is missing: {probe}")
    return probe


def methodology() -> str:
    return f"""# Benchmark methodology

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
- Per case/language: {WARMUP} warmup parses, {SAMPLES} samples, {ITERATIONS} timed parses/sample.
- Latency percentiles are distributions of **per-operation sample means**; throughput uses all timed operations.
- Startup is wall-clock process launch plus minimal import/probe initialization across {STARTUP_SAMPLES} samples, reported separately.
- Memory is best-effort Windows `WorkingSet64` sampling via `Get-Process` while a 2-second compound parsing loop runs. It is not a kernel peak-RSS counter.

The corpus is representative of the currently verified common parser subset, not a claim of full Python-suite parity.
"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust-worktree", required=True, type=Path)
    parser.add_argument("--python-source", required=True, type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    args = parser.parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    probe = build_probe(args.rust_worktree)
    results: dict[str, Any] = {"workloads": [], "config": {
        "warmup_iterations": WARMUP, "iterations_per_sample": ITERATIONS,
        "samples_per_case": SAMPLES, "startup_samples": STARTUP_SAMPLES,
        "rust_profile": "release",
    }}
    all_samples = {"python": [], "rust": []}
    total_ops = {"python": 0, "rust": 0}
    for name, grammar_source, text, description in WORKLOADS:
        row: dict[str, Any] = {"name": name, "grammar": grammar_source, "input": text,
                               "description": description, "results": {}}
        for language in ("python", "rust"):
            samples: list[float] = []
            ok = failed = 0
            for _ in range(SAMPLES):
                if language == "python":
                    elapsed, passed, errors = sample_python(args.python_source, grammar_source, text)
                else:
                    elapsed, passed, errors = sample_rust(probe, grammar_source, text)
                samples.append(elapsed)
                ok += passed
                failed += errors
            row["results"][language] = {**stats(samples, SAMPLES * ITERATIONS),
                                          "successes": ok, "errors": failed}
            all_samples[language].extend(samples)
            total_ops[language] += SAMPLES * ITERATIONS
        row["p99_speedup_python_over_rust"] = (
            row["results"]["python"]["p99_ns"] / row["results"]["rust"]["p99_ns"]
        )
        results["workloads"].append(row)

    py_start = startup_samples([sys.executable, "-c",
        f"import sys; sys.path.insert(0, {str(args.python_source)!r}); from parsimonious.grammar import Grammar; Grammar(\"root = 'a'\")"])
    rs_start = startup_samples([str(probe), "--startup"])
    compound = WORKLOADS[3]
    py_memory_command = [sys.executable, "-c", (
        "import sys,time;sys.path.insert(0,sys.argv[1]);from parsimonious.grammar import Grammar;"
        "g=Grammar(sys.argv[2]);end=time.monotonic()+float(sys.argv[4]);"
        "\nwhile time.monotonic()<end:\n try:g.parse(sys.argv[3])\n except Exception:pass"
    ), str(args.python_source), compound[1], compound[2], str(MEMORY_SECONDS)]
    rust_memory_command = [str(probe), "--grammar", compound[1], "--input", compound[2],
                           "--warmup", "0", "--iterations", "1", "--memory-seconds", str(MEMORY_SECONDS)]
    py_rss, memory_method = sampled_working_set(py_memory_command)
    rs_rss, rust_memory_method = sampled_working_set(rust_memory_command)
    results["summary"] = {
        "python": stats(all_samples["python"], total_ops["python"]),
        "rust": stats(all_samples["rust"], total_ops["rust"]),
        "p99_speedup_python_over_rust": percentile(all_samples["python"], 0.99)
            / percentile(all_samples["rust"], 0.99),
        "startup_ms": {
            "python": {"p50": percentile(py_start, .5), "p99": percentile(py_start, .99), "mean": sum(py_start) / len(py_start)},
            "rust": {"p50": percentile(rs_start, .5), "p99": percentile(rs_start, .99), "mean": sum(rs_start) / len(rs_start)},
        },
        "rss_bytes": {"python": py_rss, "rust": rs_rss,
                      "method": memory_method, "rust_method": rust_memory_method},
    }
    results["environment"] = {
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "os": platform.platform(), "cpu_count": os.cpu_count(),
        "processor": platform.processor() or os.environ.get("PROCESSOR_IDENTIFIER", "unknown"),
        "python": sys.version, "rustc": command_version([rustc_command(), "--version"]),
        "cargo": command_version([cargo_command(), "--version"]), "rust_profile": "release",
    }
    (args.out_dir / "methodology.md").write_text(methodology(), encoding="utf-8")
    (args.out_dir / "results.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    print("SUMMARY " + json.dumps(results["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
