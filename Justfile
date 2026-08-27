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