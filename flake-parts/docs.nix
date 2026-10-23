# The documents (`docs/`, of mdBook): with the videos and screenshots of the
# app (`docs/media/capture.py`), recorded as they are built, or with none, as
# the checks build them; and with the plugins' API reference at `api/`, of the
# docstrings of their Rune modules (`tools/serval-rune`).
{
  imports = [ ./common.nix ];

  perSystem =
    {
      config,
      pkgs,
      common,
      ...
    }:
    let
      inherit (pkgs.lib) fileset optionalString;
      inherit (common) craneLib commonArgs;

      # Of `docs/` alone, so built anew only as it changes
      book = fileset.toSource {
        root = ../docs;
        fileset = ../docs/book.toml;
      };
      pages = fileset.toSource {
        root = ../docs;
        fileset = ../docs/src;
      };

      # Rune's CLI with the plugins' modules: a workspace of its own, of its own
      # `Cargo.lock`
      runeArgs = commonArgs // {
        pname = "serval-rune";
        cargoToml = ../tools/serval-rune/Cargo.toml;
        cargoLock = ../tools/serval-rune/Cargo.lock;
        postUnpack = ''
          cd "$sourceRoot/tools/serval-rune"
          sourceRoot=.
        '';
        cargoExtraArgs = "--locked";
        doCheck = false;
      };
      serval-rune = craneLib.buildPackage (
        runeArgs // { cargoArtifacts = craneLib.buildDepsOnly runeArgs; }
      );

      mkDocs =
        media:
        pkgs.runCommand "serval-docs" { nativeBuildInputs = [ pkgs.mdbook ]; } ''
          cp -r --no-preserve=mode ${book} book
          cp -r --no-preserve=mode ${pages}/src book/src
          cp -r --no-preserve=mode ${config.packages.docs-api} book/src/api
          ${optionalString (media != null) "cp -r --no-preserve=mode ${media} book/src/media"}
          mdbook build book --dest-dir "$out"
        '';
    in
    {
      packages = {
        # Recorded of the app as it is built, in Chromium
        docs-media =
          pkgs.runCommand "serval-docs-media"
            (
              common.playwrightEnv
              // {
                nativeBuildInputs = [
                  common.python
                  pkgs.ffmpeg
                ];
                FONTCONFIG_FILE = common.fontsConf;
                # The script and its layouts, and the e2e tests' mock port
                src = fileset.toSource {
                  root = ../.;
                  fileset = fileset.unions [
                    ../docs/media
                    ../e2e/mock_serial.js
                  ];
                };
              }
            )
            ''
              export HOME="$TMPDIR"
              python "$src/docs/media/capture.py" ${config.packages.web-app}/share/serval-web "$out"
            '';

        # The plugins' API reference, as `docs/src/api`: in an empty Rune
        # workspace (`Rune.toml`), of `std` and `serval::*` alone
        docs-api = pkgs.runCommand "serval-docs-api" { } ''
          cp ${../tools/serval-rune/Rune.toml} Rune.toml
          ${serval-rune}/bin/serval-rune doc --output "$out"
        '';

        docs = mkDocs config.packages.docs-media;
        docs-without-media = mkDocs null;
      };

      checks.docs = config.packages.docs-without-media;
    };
}
