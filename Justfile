test *FLAGS:
    cargo build --release --bin vogls-test
    ./target/release/vogls-test {{FLAGS}}

coverage:
    cargo llvm-cov clean --workspace
    cargo llvm-cov --no-report test
    cargo llvm-cov --no-report run --bin vogls-test -- -I --opt-rounds=2 --skip=aes
    cargo llvm-cov report --html