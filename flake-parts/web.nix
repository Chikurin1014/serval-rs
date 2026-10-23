# The web app (the `web` package of the workspace, of `ui`), with the documents
# at `/docs/`: its build, served (`nix run`), its checks and its shell.
{
  imports = [
    ./common.nix
    ./docs.nix
  ];

  perSystem =
    {
      config,
      pkgs,
      common,
      ...
    }:
    let
      inherit (common)
        craneLib
        commonArgs
        playwrightEnv
        python
        rustToolchain
        src
        ;

      args = commonArgs // {
        pname = "serval-web";
        nativeBuildInputs =
          commonArgs.nativeBuildInputs
          ++ (with pkgs; [
            dioxus-cli
            wasm-bindgen-cli_0_2_126
            binaryen # `wasm-opt`, which `dx` would otherwise download
          ]);
        # `wasm-opt` aborts on the debug symbols `dx` adds by default
        buildPhaseCargoCommand = "dx build --release --debug-symbols false --package web";
        doCheck = false;
      };
      dependencies = craneLib.buildDepsOnly (
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
      );

      uiArgs = commonArgs // {
        pname = "serval-ui";
        cargoExtraArgs = "--locked --package ui";
      };
      uiArtifacts = craneLib.buildDepsOnly uiArgs;

      # The app with `docs` at `/docs/`, as `dx` serves them of `public/docs`
      withDocs =
        docs:
        pkgs.runCommand "serval-web-${commonArgs.version}" { } ''
          mkdir -p $out/share
          cp -r --no-preserve=mode ${config.packages.web-app}/share/serval-web $out/share/serval-web
          cp -r --no-preserve=mode ${docs} $out/share/serval-web/docs
        '';

      webArgs = commonArgs // {
        pname = "serval-web-lint";
        cargoExtraArgs = "--locked --package web --target wasm32-unknown-unknown";
        doCheck = false;
      };
    in
    {
      packages = {
        # The app alone
        web-app = craneLib.buildPackage (
          args
          // {
            cargoArtifacts = dependencies;
            doNotPostBuildInstallCargoBinaries = true;
            installPhaseCommand = ''
              mkdir -p $out/share
              cp -r target/dx/web/release/web/public $out/share/serval-web
            '';
          }
        );
        # With the documents, their videos and screenshots recorded as it is
        # built: as it is deployed (on a release), not as the checks build it
        web = withDocs config.packages.docs;
      };

      # The app as it is built, served on localhost (a secure context, as Web
      # Serial needs): of the flake, so from anywhere (`nix run
      # github:Chikurin1014/serval-rs`), on the port given (8080 if none)
      apps.web = {
        type = "app";
        program = pkgs.lib.getExe (
          pkgs.writeShellApplication {
            name = "serval-web";
            runtimeInputs = [ pkgs.python3 ];
            text = ''
              port=''${1:-8080}
              echo "Serval at http://localhost:$port/"
              exec python3 ${./serve.py} "$port" ${config.packages.web}/share/serval-web
            '';
          }
        );
      };

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
                FONTCONFIG_FILE = common.fontsConf;
                # Of the documents with no videos: none recorded as the checks run
                SERVAL_E2E_APP = "${withDocs config.packages.docs-without-media}/share/serval-web";
              }
            )
            ''
              export HOME="$TMPDIR"
              cp -r ${src}/e2e e2e
              # What `e2e/app.py` takes the app's initial maps from
              mkdir -p packages/ui/src/elements
              cp ${src}/packages/ui/src/elements/initial_layout.json packages/ui/src/elements/
              pytest e2e -p no:cacheprovider
              touch $out
            '';
      };

      devShells = {
        web = pkgs.mkShell (
          playwrightEnv
          // {
            inputsFrom = [ config.packages.web-app ];
            packages = [
              rustToolchain
              pkgs.cargo-about # `third-party-licenses.html` (see `about.toml`)
              pkgs.mdbook # `docs/`
              pkgs.ffmpeg # `docs/media/capture.py`
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
      };
    };
}
