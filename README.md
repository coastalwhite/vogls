<p align="center">
  <img src="./goal/assets/logo.svg" width=400 />
</p>

Vogls is a full-timing Verilog simulator focused on side-channel analysis,
interactive simulation and gate-level simulation. It can be used as
command-line application, a python library or as a Rust library to embed in
applications and tools.

> [!WARNING]
> 
> Although we use Vogls for our own research, it is still very much *alpha*
> software. We hope to develop in the open and collaborate to resolve bugs and
> implement missing features.

This repository contains the source code, tests and documentation for Vogls. Alongside that, there are several tools we have built using Vogls, including:
- [A Python library to write side-channel analysis plans](./crates/vogls-python/README.md)
- [A web application visualizing RISC-V processor pipelines](https://vogls.nl/pipeline-explorer)

## Getting started

To get started and install the Vogls Command-Line Interface:

```bash
git clone https://codeberg.org/coastalwhite/vogls
cd vogls

# Requires a Rust toolchain.
cargo install --path crates/vogls-cli
```

This will install the `vogls` binary, which allows for simulating Verilog design like:

```bash
vogls path/to/file1.v path/to/file2.v
```

There are a particularly interesting flags that you might want to use:

| Flag | Description | Effect |
|------|-------------|--------|
| `-C` | Compile to native code instead of interpreting | Faster runtime, longer preparation time |
| `-F` | Use four-value logic for the wires and registers instead of two-value logic. | Slower runtime, but necessary for certain designs. |
| `--opt-rounds=1` | Optimize the intermediate representation | Faster runtime, slightly slower runtime |

## Build

None of the tool artifacts are available in package repositories yet as we work
on creating better APIs for release. For now, you can build the tools yourself.
There are recipes for each target. Below is how to build each artifact alongside the output location.

```bash
just build-cli                    # target/release/vogls
just build-python                 # target/wheels/*.whl
just build-site-pipeline-explorer # site/pipeline-explorer
just build-site-docs              # site/docs
just build-site-python-docs       # site/py-docs
just build-site                   # site
```

## Tests

The Verilog tests are located in `crates/vogls-test/tests`. They can be ran with the following command.

```bash
just test
```

# Contribution and AI Policy

Contributions in the form of patches, pull-requests, issues and discussion are very welcome. We ask people to be respectful and provide a welcoming environment for all. If you are seeking to become a new contributor, consider starting by submitting patches containing tests or documentation.

Contributions made with the help of generative artifical intelligence or large-language models are allowed given that the submitter takes full responsibility for those contributions. At the time of writing, the Vogls core library is fully handwritten and the aim is to keep reading and writing code work for humans. Tools using Vogls may benefit more from prototyping using generative AI.

## License

Licensed under an [MIT](./LICENSE) license.