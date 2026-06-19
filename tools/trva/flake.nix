{
  description = "A basic Nix Flake for eachDefaultSystem";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
  };

  outputs =
    inputs@{
      flake-parts,
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
            riscv32-toolchain = import inputs.nixpkgs {
              localSystem = system;
              crossSystem.config = "riscv32-none-elf";
            };
          in
          {
            devShells.default = pkgs.mkShell {
              packages = [
                riscv32-toolchain.buildPackages.gcc
                riscv32-toolchain.gdb
              ];
            };
          };
      }
    );
}
