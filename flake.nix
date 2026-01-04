{
  description = "A basic Nix Flake for eachDefaultSystem";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
  };

  outputs =
    inputs@{ flake-parts, ... }:
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
          }: {
            devShells.default = pkgs.mkShell {
              packages = with pkgs; [
								(yosys.withPlugins [yosys-ghdl])
                gtkwave
                ghdl
                verilator
                iverilog
                libelf
                nodejs_24
              ];
            };
          };
      });
}