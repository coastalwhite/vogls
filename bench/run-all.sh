#!/usr/bin/env bash
# Run every benchmark under bench/ against all its simulators, the way each directory's
# run_benchmarks.sh does -- through that directory's own Justfile recipes -- and write one
# `timings.csv` per benchmark.
#
# Expects `vogls`, `just`, `iverilog`, `verilator`, `yosys` and `python3` on PATH; `nix run .#bench`
# provides them, with a vogls built on nightly with the `tailcall` and `native` features, so
# `run-vogls-interpret` is the tail-calling interpreter and `run-vogls-compile` is Cranelift.
#
# Generated inputs (prime.hex, uart-aes/build/gtl.v) are made once if missing and then kept; only
# simulator outputs are removed between iterations.
set -euo pipefail

iterations=10
filter=""
root=""
bench_root=""
usage() {
    echo "usage: $0 [--iterations N] [--filter SUBSTRING] [--root REPO | --bench-root DIR]" >&2
    exit 2
}
while [ $# -gt 0 ]; do
    case $1 in
        --iterations) iterations=$2; shift 2 ;;
        --filter) filter=$2; shift 2 ;;
        --root) root=$2; shift 2 ;;
        --bench-root) bench_root=$2; shift 2 ;;
        *) usage ;;
    esac
done

if [ -z "$bench_root" ]; then
    if [ -z "$root" ]; then
        root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
    fi
    bench_root="$root/bench"
fi
[ -d "$bench_root" ] || { echo "no benchmark directory at $bench_root" >&2; exit 1; }
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

# The recipes time their steps with bash's `time` keyword, so run them under bash whatever `sh` is.
JUST=(just --shell bash --shell-arg -cu)

run_bench() {
    local name=$1; shift
    local tools=("$@")
    local dir="$bench_root/$name"
    cd "$dir"

    case $name in
        prime)
            # `just build` assembles through `cargo run -p trva-cli`; with the assembler already on
            # PATH (as in the nix build, which has no cargo) do the same two steps directly.
            if [ ! -f prime.hex ]; then
                if command -v trva > /dev/null; then
                    trva prime.S --isa rv32im --text 0x0 --data 0x200 --rodata 0x300 --bss 0x400 \
                        -o prime.trva
                    python3 createmem.py
                else
                    "${JUST[@]}" build
                fi
            fi ;;
        uart-aes)
            [ -f build/gtl.v ] || "${JUST[@]}" build-gtl ;;
    esac

    for t in "${tools[@]}"; do : > "log-$t"; done
    for it in $(seq 1 "$iterations"); do
        echo "### $name iteration $it/$iterations $(date +%T)" >&2
        rm -rf obj_dir icarus dump.vcd
        for t in "${tools[@]}"; do
            echo "### run-$t iteration $it" >> "log-$t"
            if ! "${JUST[@]}" "run-$t" >> "log-$t" 2>&1; then
                echo "### FAILED run-$t iteration $it" >> "log-$t"
                echo "    run-$t failed (see $dir/log-$t)" >&2
            fi
        done
        rm -rf obj_dir icarus dump.vcd
    done

    python3 "$here/logs_to_csv.py" "$name" "$dir" "${tools[@]}"
}

for spec in \
    "canright verilator icarus vogls-interpret vogls-compile" \
    "prime verilator icarus vogls-interpret vogls-compile" \
    "uart-aes icarus vogls-interpret vogls-compile"
do
    # shellcheck disable=SC2086
    set -- $spec
    name=$1; shift
    if [ -n "$filter" ] && [[ $name != *"$filter"* ]]; then
        continue
    fi
    run_bench "$name" "$@"
done
