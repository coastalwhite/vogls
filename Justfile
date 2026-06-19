test *FLAGS:
    cargo build --release --bin vogls-test
    ./target/release/vogls-test {{FLAGS}}

coverage:
    cargo llvm-cov clean --workspace
    cargo llvm-cov --no-report test
    cargo llvm-cov --no-report run --bin vogls-test -- -I --opt-rounds=2 --skip=aes
    cargo llvm-cov report --html

build-site:
    rm -rf site/pipeline-explorer
    {{just_executable()}} {{justfile_directory()}}/tools/pipeline-explorer/build-site
    cp -r {{justfile_directory()}}/tools/pipeline-explorer/webapp/dist site/pipeline-explorer