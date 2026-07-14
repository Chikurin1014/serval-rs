{
  description = "Cross-Platform Serial visualizer";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    let
      package-types = [
        "web"
      ];
      package-names = [ "default" ] ++ package-types;
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        ./flake-parts/serval.nix
      ];

      systems = [ "x86_64-linux" ];

      perSystem =
        {
          config,
          system,
          pkgs,
          ...
        }:
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.rust-overlay.overlays.default
            ];
          };

          # packages = {
          #   default = import ./nix/serval.nix {
          #     package-type = builtins.head package-types;
          #     inherit pkgs;
          #   };
          # }
          # // builtins.listToAttrs (
          #   map (name: {
          #     inherit name;
          #     value = import ./nix/serval.nix {
          #       package-type = name;
          #       inherit pkgs;
          #     };
          #   }) package-types
          # );
          #
          # devShells = builtins.listToAttrs (
          #   map (name: {
          #     inherit name;
          #     value = pkgs.mkShell {
          #       inputsFrom = [ config.packages.${name} ];
          #     };
          #   }) package-names
          # );

          formatter = inputs.treefmt-nix.lib.mkWrapper pkgs {
            projectRootFile = "flake.nix";
            programs = {
              nixfmt.enable = true;
              rustfmt.enable = true; # Rust
              taplo.enable = true; # TOML
              prettier.enable = true; # JS/TS/JSON/MD/...
            };
          };
        };
    };
}
