"""External differential oracle for the Parsimonious Python-to-Rust port.

This verification program may invoke the Python original, but the Rust crate
does not: its only Rust counterpart is the safe ``fuzz_probe`` binary.  Cases
are intentionally limited to the common text-grammar subset currently exposed
by both implementations.  Dynamic custom rules, token streams, bytes grammars,
and Python-specific regex flags are not silently exercised or claimed here.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


FIXED_GRAMMARS = [
    "root = 'a'",
    "root = 'a'+",
    "root = 'a'*",
    "root = 'a'?",
    "root = 'a' 'b'",
    "root = 'a' / 'b'",
    "root = !'b' 'a'",
    "root = ('a' / 'b')+",
    "root = 'a'{2,3}",
    "root = ~'[ab]+'",
]
INPUTS = ["", "a", "b", "aa", "ab", "ba", "aaa", "bbb", "aab"]


def node_shape(node: Any) -> str:
    """Compare tree structure and spans without Python/Rust display names."""
    return f"{node.start}:{node.end}[{','.join(node_shape(child) for child in node.children)}]"


def normalize_python(source: Path, grammar_source: str, text: str) -> dict[str, Any]:
    sys.path.insert(0, str(source))
    try:
        from parsimonious.exceptions import (  # type: ignore[import-not-found]
            BadGrammar,
            IncompleteParseError,
            LeftRecursionError,
            ParseError,
            UndefinedLabel,
            VisitationError,
        )
        from parsimonious.grammar import Grammar  # type: ignore[import-not-found]

        try:
            node = Grammar(grammar_source).parse(text)
            return {"status": "ok", "shape": node_shape(node)}
        except IncompleteParseError as error:
            return {"status": "error", "type": "IncompleteParse", "offset": error.pos}
        except LeftRecursionError as error:
            return {"status": "error", "type": "LeftRecursion", "offset": error.pos}
        except ParseError as error:
            return {"status": "error", "type": "Parse", "offset": error.pos}
        except BadGrammar:
            return {"status": "error", "type": "BadGrammar", "offset": -1}
        except UndefinedLabel:
            return {"status": "error", "type": "UndefinedLabel", "offset": -1}
        except VisitationError:
            return {"status": "error", "type": "VisitationError", "offset": -1}
    finally:
        try:
            sys.path.remove(str(source))
        except ValueError:
            pass


def normalize_rust(probe: Path, grammar_source: str, text: str) -> dict[str, Any]:
    proc = subprocess.run(
        [str(probe)], input=f"{grammar_source}\n{text}\n", text=True,
        capture_output=True, encoding="utf-8", errors="replace",
    )
    line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
    if proc.returncode != 0 or not line:
        return {"status": "probe_error", "type": "ProbeExit", "offset": -1,
                "stderr": proc.stderr[-1000:], "stdout": proc.stdout[-1000:]}
    parts = line.split("|", 2)
    if parts[0] == "OK" and len(parts) == 2:
        return {"status": "ok", "shape": parts[1]}
    if parts[0] == "ERR" and len(parts) == 3:
        try:
            offset = int(parts[2])
        except ValueError:
            offset = -1
        return {"status": "error", "type": parts[1], "offset": offset}
    return {"status": "probe_error", "type": "ProbeProtocol", "offset": -1,
            "stdout": proc.stdout[-1000:], "stderr": proc.stderr[-1000:]}


def random_grammar(rng: random.Random) -> str:
    left, right = rng.choice("ab"), rng.choice("ab")
    form = rng.randrange(6)
    if form == 0:
        rhs = f"'{left}'{rng.choice(['?', '*', '+', '{1,2}'])}"
    elif form == 1:
        rhs = f"'{left}' '{right}'"
    elif form == 2:
        rhs = f"'{left}' / '{right}'"
    elif form == 3:
        rhs = f"!'{right}' '{left}'"
    elif form == 4:
        rhs = f"('{left}' / '{right}')+"
    else:
        rhs = f"~'[{left}{right}]+'"
    return f"root = {rhs}"


def build_probe(worktree: Path) -> Path:
    cargo = shutil.which("cargo")
    if cargo is None and os.name == "nt":
        candidate = Path(os.environ.get("USERPROFILE", "")) / ".cargo" / "bin" / "cargo.exe"
        if candidate.is_file():
            cargo = str(candidate)
    if cargo is None:
        raise RuntimeError("cargo is not on PATH and no USERPROFILE Cargo installation was found")
    proc = subprocess.run(
        [cargo, "build", "--release", "--bin", "fuzz_probe"], cwd=worktree,
        text=True, capture_output=True, encoding="utf-8", errors="replace",
    )
    if proc.returncode:
        raise RuntimeError(f"fuzz probe build failed:\n{proc.stdout}\n{proc.stderr}")
    name = "fuzz_probe.exe" if sys.platform.startswith("win") else "fuzz_probe"
    probe = worktree / "target" / "release" / name
    if not probe.is_file():
        raise RuntimeError(f"fuzz probe build reported success but {probe} is missing")
    return probe


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust-worktree", required=True, type=Path)
    parser.add_argument("--python-source", required=True, type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument("--duration", required=True, type=float)
    args = parser.parse_args()

    if args.duration < 60:
        raise SystemExit("refusing duration below the 60-second DiffFuzz minimum")
    args.out_dir.mkdir(parents=True, exist_ok=True)
    repro_dir = args.out_dir / "repros"
    repro_dir.mkdir(exist_ok=True)
    log_path = args.out_dir / "log.txt"
    cases_path = args.out_dir / "cases.jsonl"
    probe = build_probe(args.rust_worktree)
    rng = random.Random(0xD1FF)  # deterministic repro stream
    started = time.monotonic()
    counts = {"total": 0, "fixed_grammar_random_input": 0, "random_grammar_fixed_input": 0}
    divergences: list[dict[str, Any]] = []
    divergence_total = 0

    with log_path.open("w", encoding="utf-8") as log, cases_path.open("w", encoding="utf-8") as cases:
        log.write("DiffFuzz verification oracle: Python original external; shipping Rust crate remains Python-free.\n")
        log.write("Subset: text grammars with literals, choice, lookahead, repetition, and simple character-class regex.\n")
        log.write(f"duration_target_s={args.duration:.3f} rust_probe={probe}\n")
        while time.monotonic() - started < args.duration:
            fixed = FIXED_GRAMMARS[counts["fixed_grammar_random_input"] % len(FIXED_GRAMMARS)]
            random_input = rng.choice(INPUTS)
            generated = random_grammar(rng)
            fixed_input = INPUTS[counts["random_grammar_fixed_input"] % len(INPUTS)]
            for category, grammar_source, text in (
                ("fixed_grammar_random_input", fixed, random_input),
                ("random_grammar_fixed_input", generated, fixed_input),
            ):
                python_result = normalize_python(args.python_source, grammar_source, text)
                rust_result = normalize_rust(probe, grammar_source, text)
                equal = python_result == rust_result
                case = {
                    "id": counts["total"], "category": category, "grammar": grammar_source,
                    "input": text, "python": python_result, "rust": rust_result,
                    "equal": equal,
                }
                cases.write(json.dumps(case, sort_keys=True) + "\n")
                counts["total"] += 1
                counts[category] += 1
                if not equal:
                    divergence_total += 1
                    if len(divergences) < 3:
                        divergences.append(case)
                        repro = repro_dir / f"divergence-{len(divergences):02d}.json"
                        repro.write_text(json.dumps(case, indent=2, sort_keys=True) + "\n", encoding="utf-8")
                        log.write(f"DIVERGENCE repro={repro.name} case={case['id']} category={category}\n")
        elapsed = time.monotonic() - started
        summary = {
            "duration_s": round(elapsed, 3), "counts": counts,
            "divergence_count": divergence_total, "saved_repros": len(divergences),
            "fields": ["success/failure", "parse-tree shape", "error type", "error offset"],
        }
        log.write("SUMMARY " + json.dumps(summary, sort_keys=True) + "\n")
    print("SUMMARY " + json.dumps(summary, sort_keys=True))
    return 1 if divergence_total else 0


if __name__ == "__main__":
    raise SystemExit(main())
