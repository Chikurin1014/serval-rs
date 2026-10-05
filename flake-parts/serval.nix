{
  ...
}:

{
  perSystem =
    { config, pkgs, ... }:
    let
      package-types = [ "web" ];
      package-names = [ "default" ] ++ package-types;
      rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };
      src = pkgs.lib.cleanSource ../.;

      cargoLock = {
        lockFile = ../Cargo.lock;
        # Git dependencies have no checksum in Cargo.lock, so Nix needs their hashes.
        # Both crates come from the same checkout of DioxusLabs/components.
        # Update these whenever that revision changes in Cargo.lock.
        outputHashes = {
          "dioxus-attributes-0.1.0" = "sha256-y4aeyIKJxwdi4L+D9Gm+jM5eXaBGZCKfcliyoYnypE4=";
          "dioxus-primitives-0.0.1" = "sha256-y4aeyIKJxwdi4L+D9Gm+jM5eXaBGZCKfcliyoYnypE4=";
        };
      };

      # ref: https://github.com/DioxusLabs/dioxus/blob/main/flake.nix
      buildInputs =
        with pkgs;
        [
          openssl
          libiconv
          pkg-config
        ]
        ++ lib.optionals pkgs.stdenv.isLinux [
          glib
          gtk3
          libsoup_3
          libudev-zero
          webkitgtk_4_1
          xdotool
        ]
        ++ lib.optionals pkgs.stdenv.isDarwin [
          apple-sdk
          libiconv
        ];

      python = pkgs.python3.withPackages (ps: [
        ps.pytest
        ps.playwright
      ]);
      playwrightEnv = {
        # The browsers matching the Playwright above, instead of a download
        PLAYWRIGHT_BROWSERS_PATH = pkgs.playwright-driver.browsers;
        PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
      };

      mkPackage =
        package-type:
        rustPlatform.buildRustPackage {
          pname = "serval-${package-type}";
          version = "0.1.0";
          inherit src cargoLock buildInputs;
          nativeBuildInputs = with pkgs; [
            rustToolchain
            pkg-config
            dioxus-cli
            wasm-bindgen-cli_0_2_126
            binaryen # `wasm-opt`, which `dx` would otherwise download
          ];
          # `wasm-opt` aborts on the debug symbols `dx` adds by default
          buildPhase = ''
            dx build --release --debug-symbols false --package ${package-type}
          '';
          installPhase =
            if package-type == "web" then
              # Static files to serve, not programs: in `share`, as data
              ''
                mkdir -p $out/share
                cp -r target/dx/web/release/web/public $out/share/serval-web
              ''
            else
              # The program in `bin`, and its assets where Dioxus looks for a
              # Linux app's: `lib/<name>/assets`, beside `bin`
              ''
                app=target/dx/${package-type}/release/linux/app
                mkdir -p $out/bin $out/lib/serval-${package-type}
                cp $app/${package-type} $out/bin/serval-${package-type}
                cp -r $app/assets $out/lib/serval-${package-type}/
              '';
          doCheck = false; # Disable tests to avoid building deps for them
        };
    in
    {
      packages = {
        default = mkPackage (builtins.head package-types);
      }
      // builtins.listToAttrs (
        map (name: {
          inherit name;
          value = mkPackage name;
        }) package-types
      );

      checks = {
        rust-tests = rustPlatform.buildRustPackage {
          pname = "serval-rust-tests";
          version = "0.1.0";
          inherit src cargoLock buildInputs;
          nativeBuildInputs = [
            rustToolchain
            pkgs.pkg-config
          ];
          dontCargoBuild = true; # `cargo test` builds what it needs
          cargoTestFlags = [
            "--package"
            "ui"
          ];
          installPhase = "touch $out";
        };

        js-tests = pkgs.runCommand "serval-js-tests" { nativeBuildInputs = [ pkgs.nodejs ]; } ''
          cd ${src}
          node --test 'packages/**/*.test.mjs'
          touch $out
        '';

        e2e-tests =
          pkgs.runCommand "serval-e2e-tests"
            (
              playwrightEnv
              // {
                nativeBuildInputs = [ python ];
                # Chromium aborts without a fontconfig setup, which the sandbox lacks
                FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = [ pkgs.dejavu_fonts ]; };
                SERVAL_E2E_APP = "${config.packages.web}/share/serval-web";
              }
            )
            ''
              export HOME="$TMPDIR"
              cp -r ${src}/e2e e2e
              pytest e2e -p no:cacheprovider
              touch $out
            '';
      };

      devShells = builtins.listToAttrs (
        map (name: {
          inherit name;
          value = pkgs.mkShell (
            playwrightEnv
            // {
              inputsFrom = [ config.packages.${name} ];
              packages = [
                pkgs.nodejs
                pkgs.prettier
                pkgs.pyright
                pkgs.ruff
                pkgs.taplo
                pkgs.vscode-css-languageserver
                pkgs.vscode-json-languageserver
                python
              ];
            }
          );
        }) package-names
      );
    };
}
