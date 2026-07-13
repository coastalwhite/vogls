{
  description = "A basic Nix Flake for eachDefaultSystem";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    inputs@{ flake-parts, rust-overlay, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } (
      { ... }:
      {
        systems = [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ];
        perSystem =
          {
            system,
            pkgs,
            lib,
            self',
            ...
          }:
          let
            pythonPlatform = lib.recursiveUpdate {
              python = pkgs.python311;
            } pkgs.python311Packages;
            stdenv = pkgs.stdenv;
            riscv32-toolchain = import inputs.nixpkgs {
              localSystem = system;
              crossSystem.config = "riscv32-none-elf";
            };

            buildTarget = "wasm32-unknown-unknown";
            rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
              targets = [ buildTarget ];
              extensions = [ "rust-src" ];
            };
            rustPlatform = pkgs.makeRustPlatform {
              cargo = rustToolchain;
              rustc = rustToolchain;
            };
          in
          {
            _module.args.pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            };

            devShells.default = pkgs.mkShell {
              packages = with pkgs; [
                (pythonPlatform.python.withPackages (python-pkgs: [
                  python-pkgs.numpy
                ]))
                pythonPlatform.venvShellHook
                pythonPlatform.build

                cargo-llvm-cov
                rustToolchain
                wasm-bindgen-cli_0_2_114

                maturin
                just
                samply
                uv
                (yosys.withPlugins [ yosys-ghdl ])
                gtkwave
                ghdl
                verilator
                iverilog
                libelf

                stdenv.cc.cc.lib
                # riscv32-toolchain.buildPackages.gcc
              ];

              postVenvCreation = ''
                unset CONDA_PREFIX 
                uv pip install -r crates/vogls-python/pyproject.toml
                export NIX_LD_LIBRARY_PATH="${
                  pkgs.lib.makeLibraryPath [
                    stdenv.cc.cc.lib
                  ]
                }:$PYTHON_SHARED_LIB"
                export LD_LIBRARY_PATH="${stdenv.cc.cc.lib}/lib:$PYTHON_SHARED_LIB"
              '';
              venvDir = ".venv";
            };

            devShells.bench = pkgs.mkShell {
              packages = [
                riscv32-toolchain.buildPackages.gcc
              ];
            };

            devShells.docs = pkgs.mkShell {
              packages = with pkgs; [
                pythonPlatform.python
                pythonPlatform.venvShellHook
                pythonPlatform.build

                just
                rustToolchain
                wasm-bindgen-cli_0_2_114

                mdbook
              ];

              postVenvCreation = ''
                unset CONDA_PREFIX 
                uv pip install -r crates/vogls-python/pyproject.toml
                export LD_LIBRARY_PATH="${stdenv.cc.cc.lib}/lib:$PYTHON_SHARED_LIB"
              '';
              venvDir = ".venv";
            };

            packages.default = self'.packages.vogls;
            packages.vogls = rustPlatform.buildRustPackage {
              name = "vogls";
              src = ./.;
              cargoBuildFlags = "--bin vogls";
              meta.mainProgram = "vogls";
              cargoLock.lockFile = ./Cargo.lock;
              doCheck = false;
            };
            packages.site = pkgs.stdenv.mkDerivation {
              name = "site";
              buildInputs = [
                rustToolchain
              ];
              src = ./.;
              buildPhase = ''
                ${pkgs.just}/bin/just build-site
              '';
              installPhase = ''
                cp -r site $out
              '';
            };
          };
      }
    );
}
