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
          }: 
          let
            pythonPlatform = lib.recursiveUpdate {
              python = pkgs.python311;
            } pkgs.python311Packages;
						stdenv = pkgs.stdenv;
          in {
            devShells.default = pkgs.mkShell {
              packages = with pkgs; [
                pythonPlatform.python
                pythonPlatform.venvShellHook
                pythonPlatform.build
            
                samply
                uv
								(yosys.withPlugins [yosys-ghdl])
                gtkwave
                ghdl
                verilator
                iverilog
                libelf
                nodejs_24
              ];

              postVenvCreation = ''
                unset CONDA_PREFIX 
                uv pip install -r pyproject.toml
								export LD_LIBRARY_PATH="${stdenv.cc.cc.lib}/lib:$PYTHON_SHARED_LIB"
              '';
              venvDir = ".venv";
            };
          };
      });
}