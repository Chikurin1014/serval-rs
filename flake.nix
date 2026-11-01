{
  description = "Cross-Platform Serial visualizer";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
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
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        ./flake-parts/desktop.nix
        ./flake-parts/devshell.nix
        ./flake-parts/web.nix
      ];

      systems = [ "x86_64-linux" ];

      perSystem =
        {
          system,
          pkgs,
          config,
          ...
        }:
        let
          treefmt = inputs.treefmt-nix.lib.evalModule pkgs {
            projectRootFile = "flake.nix";
            programs = {
              nixfmt.enable = true;
              ruff = {
                enable = true;
                format = true;
              };
              rustfmt.enable = true; # Rust
              taplo.enable = true; # TOML
              # JS/JSON/MD/CSS/YAML
              dprint = {
                enable = true;
                includes = [
                  "**/*.css"
                  "**/*.js"
                  "**/*.json"
                  "**/*.md"
                  "**/*.mjs"
                  "**/*.yml"
                ];
                settings = {
                  lineWidth = 80;
                  plugins = pkgs.dprint-plugins.getPluginList (
                    plugins: with plugins; [
                      dprint-plugin-json
                      dprint-plugin-markdown
                      dprint-plugin-typescript
                      g-plane-malva
                      g-plane-pretty_yaml
                    ]
                  );
                };
              };
            };
            settings.global.excludes = [
              # Generated
              "packages/desktop/third-party-licenses.html"
              "packages/web/public/third-party-licenses.html"
              "docs/book/**"
              "packages/web/public/docs/**"
              "packages/ui/src/components/**"
              "packages/ui/assets/dx-components-theme.css"
              "CHANGELOG.md"
              # Third-party
              "packages/ui/assets/vendor/**"
              "docs/mermaid.min.js"
              "docs/mermaid-init.js"
            ];
          };
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.rust-overlay.overlays.default
            ];
          };

          packages.default = config.packages.web;
          apps.default = config.apps.desktop;

          formatter = treefmt.config.build.wrapper;
          checks.formatting = treefmt.config.build.check inputs.self;
        };
    };
}
