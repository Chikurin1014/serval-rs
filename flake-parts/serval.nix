{
  inputs,
  ...
}:

{
  perSystem =
    { config, pkgs, ... }:
    let
      inherit (builtins)
        head
        listToAttrs
        readFile
        ;

      package-types = [ "web" ];
      package-names = [ "default" ] ++ package-types;
      rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (_: rustToolchain);
      src = pkgs.lib.cleanSource ../.;

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

      # Dependencies are built apart (`buildDepsOnly`), so they are cached
      commonArgs = {
        pname = "serval";
        version = (fromTOML (readFile ../Cargo.toml)).workspace.package.version;
        inherit src buildInputs;
        strictDeps = true;
        nativeBuildInputs = [ pkgs.pkg-config ];
      };

      python = pkgs.python3.withPackages (ps: [
        ps.pytest
        ps.playwright
      ]);
      playwrightEnv = {
        PLAYWRIGHT_BROWSERS_PATH = pkgs.playwright-driver.browsers;
        PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
      };

      mkPackage =
        package-type:
        let
          args = commonArgs // {
            pname = "serval-${package-type}";
            nativeBuildInputs =
              commonArgs.nativeBuildInputs
              ++ (with pkgs; [
                dioxus-cli
                wasm-bindgen-cli_0_2_126
                binaryen # `wasm-opt`, which `dx` would otherwise download
              ]);
            # `wasm-opt` aborts on the debug symbols `dx` adds by default
            buildPhaseCargoCommand = "dx build --release --debug-symbols false --package ${package-type}";
            doCheck = false;
          };
          dependencies = craneLib.buildDepsOnly (
            if package-type == "web" then
              removeAttrs args [ "src" ]
              // {
                # `wasm-bindgen` fails on an app that does nothing
                dummySrc = craneLib.mkDummySrc {
                  inherit src;
                  extraDummyScript = ''
                    chmod +w $out/packages/web/src/main.rs
                    cat > $out/packages/web/src/main.rs <<'EOF'
                    fn main() {
                        dioxus::launch(|| dioxus::prelude::VNode::empty());
                    }
                    EOF
                  '';
                };
              }
            else
              args
          );
        in
        craneLib.buildPackage (
          args
          // {
            cargoArtifacts = dependencies;
            doNotPostBuildInstallCargoBinaries = true;
            installPhaseCommand =
              if package-type == "web" then
                ''
                  mkdir -p $out/share
                  cp -r target/dx/web/release/web/public $out/share/serval-web
                ''
              else
                # Where Dioxus looks for a Linux app's assets
                ''
                  app=target/dx/${package-type}/release/linux/app
                  mkdir -p $out/bin $out/lib/serval-${package-type}
                  cp $app/${package-type} $out/bin/serval-${package-type}
                  cp -r $app/assets $out/lib/serval-${package-type}/
                '';
          }
        );

      uiArgs = commonArgs // {
        pname = "serval-ui";
        cargoExtraArgs = "--locked --package ui";
      };
      uiArtifacts = craneLib.buildDepsOnly uiArgs;

      webArgs = commonArgs // {
        pname = "serval-web-lint";
        cargoExtraArgs = "--locked --package web --target wasm32-unknown-unknown";
        doCheck = false;
      };
    in
    {
      packages = {
        default = mkPackage (head package-types);
      }
      // listToAttrs (
        map (name: {
          inherit name;
          value = mkPackage name;
        }) package-types
      );

      checks = {
        rust-tests = craneLib.cargoTest (uiArgs // { cargoArtifacts = uiArtifacts; });

        clippy-ui = craneLib.cargoClippy (
          uiArgs
          // {
            cargoArtifacts = uiArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          }
        );

        clippy-web = craneLib.cargoClippy (
          webArgs
          // {
            cargoArtifacts = craneLib.buildDepsOnly webArgs;
            cargoClippyExtraArgs = "-- --deny warnings";
          }
        );

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
                # Chromium aborts without a fontconfig setup
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

      devShells = listToAttrs (
        map (name: {
          inherit name;
          value = pkgs.mkShell (
            playwrightEnv
            // {
              inputsFrom = [ config.packages.${name} ];
              packages = [
                rustToolchain
                pkgs.cargo-about # `third-party-licenses.html` (see `about.toml`)
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
