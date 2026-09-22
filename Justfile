lint:
	cargo clippy --workspace

format:
	cargo fmt --check

format-fix:
	cargo fmt

check:
    RUSTFLAGS="-D warnings" cargo check --workspace

precommit: check lint format

test-bytecode *FLAGS:
    {{just_executable()}} --justfile {{justfile()}} test -B

test-cranelift *FLAGS:
    {{just_executable()}} --justfile {{justfile()}} test --cranelift

test *FLAGS:
    cargo build --bin vogls-test --profile=fast-dev
    ./target/fast-dev/vogls-test {{FLAGS}}

coverage:
    cargo llvm-cov clean --workspace
    # cargo llvm-cov --no-report test
    cargo llvm-cov --no-report run --bin vogls-test -- --skip aes
    cargo llvm-cov report --html

build-site: build-site-pipeline-explorer build-site-python-docs build-site-docs

build-site-docs:
    rm -rf site/docs
    cd docs && mdbook build --dest-dir ../site/docs
    {{just_executable()}} {{justfile_directory()}}/tools/pipeline-explorer/build-site
    cp -r {{justfile_directory()}}/tools/pipeline-explorer/webapp/dist site/pipeline-explorer

build-site-python-docs:
    rm -rf site/py-docs
    pdoc vogls --math -n --output-directory site/py-docs

build-site-pipeline-explorer:
    rm -rf site/pipeline-explorer
    {{just_executable()}} {{justfile_directory()}}/tools/pipeline-explorer/build-site
    cp -r {{justfile_directory()}}/tools/pipeline-explorer/webapp/dist site/pipeline-explorer
# --- Benchmarks -------------------------------------------------------------

# Where the benchmark `vogls` is built. Kept out of ./target so that switching between
# the stable toolchain development uses and the nightly one below does not rebuild the
# world each time.
bench_target := justfile_directory() / "target/bench"

# Build the `vogls` that the bench/*/Justfile recipes invoke by name: the tail-calling
# bytecode interpreter, which only nightly can build, plus the Cranelift backend behind
# `-C`, which is the default `native` feature.
[doc("Build the nightly, tail-calling `vogls` that the benchmarks run")]
bench-vogls:
    CARGO_TARGET_DIR={{bench_target}} \
        nix develop {{justfile_directory()}}#nightly --command \
        cargo build --release -p vogls-cli --features tailcall

# Run every benchmark under bench/ against each of its simulators and write one
# timings.csv per benchmark -- bench/{canright,prime,uart-aes}/timings.csv -- with a row
# per iteration and tool giving that run's build and simulation seconds. Every simulator
# is invoked through that benchmark's own Justfile recipes.
#
#     just bench                    # 10 iterations of all three benchmarks
#     just bench --iterations 1     # a quick smoke run
#     just bench --filter uart-aes  # a single benchmark
[doc("Run every benchmark and write bench/<name>/timings.csv")]
bench *ARGS: bench-vogls
    PATH="{{bench_target}}/release:$PATH" \
        ./bench/run-all.sh --root {{justfile_directory()}} {{ARGS}}
