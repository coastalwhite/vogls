#!/usr/bin/env python3
"""Turn the `log-<tool>` files a benchmark's Justfile recipes wrote into `timings.csv`.

    logs_to_csv.py <benchmark> <directory> <tool>...

Every iteration of every tool becomes one row: `iteration,tool,build_seconds,run_seconds,ok`.
Icarus and Verilator recipes print two bash `time` blocks per iteration, the build then the run.
The vogls recipes print a `# Timings Overview:` block: `run` is its `simulation` phase and `build`
is every other phase added up. `ok` is false when the recipe failed or, where the benchmark has a
known answer, when the output did not contain it; the times are then left empty.
"""
import csv
import re
import sys
from pathlib import Path

# bash `time` keyword: `real\t0m1.234s`; GNU time: `0.05user 0.01system 0:01.23elapsed`.
BASH_REAL = re.compile(r"^real\s+(\d+)m([\d.]+)s")
GNU_ELAPSED = re.compile(r"(?:(\d+):)?(\d+):([\d.]+)elapsed")
PHASE = re.compile(r"^(\w+): ([\d.]+)s$")

# canright prints nothing when it passes, so it is checked the other way round: its
# `COMPAT` self-check says "Mismatch!" and calls $finish, which still leaves two well
# formed `time` blocks behind, so the times alone would look fine.
FORBIDDEN = {
    "canright": re.compile(r"^Mismatch!", re.M),
}

EXPECTED = {
    "prime": re.compile(r"Prime =\s+7919\b"),
    "uart-aes": re.compile(r"Output = 7aca0fd9bcd6ec7c9f97466616e6a282"),
}


def wall_times(chunk: str) -> list[float]:
    times = []
    for line in chunk.splitlines():
        m = BASH_REAL.match(line)
        if m:
            times.append(int(m.group(1)) * 60 + float(m.group(2)))
            continue
        m = GNU_ELAPSED.search(line)
        if m:
            h = int(m.group(1) or 0)
            times.append(h * 3600 + int(m.group(2)) * 60 + float(m.group(3)))
    return times


def parse_external(chunk: str):
    times = wall_times(chunk)
    if len(times) != 2:
        return None
    return times[0], times[1]


def parse_vogls(chunk: str):
    phases = {}
    for line in chunk.splitlines():
        m = PHASE.match(line.strip())
        if m:
            phases[m.group(1)] = float(m.group(2))
    if "simulation" not in phases:
        return None
    build = sum(v for k, v in phases.items() if k != "simulation")
    return build, phases["simulation"]


def main() -> None:
    bench, directory, *tools = sys.argv[1:]
    directory = Path(directory)
    expected = EXPECTED.get(bench)
    forbidden = FORBIDDEN.get(bench)
    rows = []
    for tool in tools:
        log = (directory / f"log-{tool}").read_text(errors="replace")
        chunks = re.split(r"^### run-\S+ iteration \d+\n", log, flags=re.M)[1:]
        parse = parse_vogls if tool.startswith("vogls") else parse_external
        for it, chunk in enumerate(chunks, 1):
            failed = "### FAILED" in chunk
            times = None if failed else parse(chunk)
            ok = (
                times is not None
                and (expected is None or expected.search(chunk) is not None)
                and (forbidden is None or forbidden.search(chunk) is None)
            )
            rows.append(
                {
                    "iteration": it,
                    "tool": tool,
                    "build_seconds": f"{times[0]:.4f}" if ok else "",
                    "run_seconds": f"{times[1]:.4f}" if ok else "",
                    "ok": str(ok).lower(),
                }
            )

    out = directory / "timings.csv"
    with out.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["iteration", "tool", "build_seconds", "run_seconds", "ok"])
        w.writeheader()
        w.writerows(rows)
    print(f"wrote {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
