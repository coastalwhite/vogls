<p align="center">
  <img src="./goal/assets/logo.svg" width=400 />
</p>

Vogls is a full-timing Verilog simulator focused on side-channel analysis,
interactive simulation and gate-level simulation. It can be used as
command-line application, a python library or as a Rust library to embed in
applications and tools.


## Installation

```
cargo install --git https://codeberg.org/coastalwhite/vogls.git
```

## Build

```
cargo build --release vogls
```

## Tests

```
cargo test
cargo run -p vogls-test --release
```