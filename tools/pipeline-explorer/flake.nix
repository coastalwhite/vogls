{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    inputs@{
      flake-parts,
      rust-overlay,
      ...
    }:
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
            ...
          }:
          let
            buildTarget = "wasm32-unknown-unknown";
            rustToolchain = pkgs.rust-bin.stable.latest.default.override {
              targets = [ buildTarget ];
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
                nodejs_22
                wabt
                minify
								just
                rustToolchain
                wasm-bindgen-cli_0_2_114

                fusesoc
                haskellPackages.sv2v
              ];
            };
            packages.wasm = rustPlatform.buildRustPackage {
              name = "pipeline-explorer-wasm";
              src = ./.;

              cargoLock.lockFile = ./Cargo.lock;

              buildPhase = ''
                cargo build --release -p pipeline-explorer --target=${buildTarget}
              '';

              installPhase = ''
                mkdir -p $out/lib
                cp target/${buildTarget}/release/*.wasm $out/lib/
              '';

              doCheck = false;
            };
          };
      }
    );
}
