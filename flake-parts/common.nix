# What the modules of the flake share, as the module argument `common`: the
# Rust toolchain and crane, the source, the arguments of their builds, and the
# Python of the e2e tests.
{
  inputs,
  ...
}:

{
  perSystem =
    { pkgs, ... }:
    let
      rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
      src = pkgs.lib.cleanSource ../.;
    in
    {
      _module.args.common = {
        inherit rustToolchain src;
        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (_: rustToolchain);

        # Dependencies are built apart (`buildDepsOnly`), so they are cached
        commonArgs = {
          pname = "serval";
          version = (fromTOML (builtins.readFile ../Cargo.toml)).workspace.package.version;
          inherit src;
          strictDeps = true;
          nativeBuildInputs = [ pkgs.pkg-config ];
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
        };

        python = pkgs.python3.withPackages (ps: [
          ps.pytest
          ps.pytest-xdist
          ps.playwright
        ]);
        playwrightEnv = {
          PLAYWRIGHT_BROWSERS_PATH = pkgs.playwright-driver.browsers;
          PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
        };
        # For Chromium in a build, which aborts with no fontconfig setup: the
        # generic families (as the app's `sans-serif`) as DejaVu's, not
        # whichever fontconfig finds first
        fontsConf = pkgs.writeText "fonts.conf" ''
          <?xml version="1.0"?>
          <!DOCTYPE fontconfig SYSTEM "fonts.dtd">
          <fontconfig>
            <include>${pkgs.makeFontsConf { fontDirectories = [ pkgs.dejavu_fonts ]; }}</include>
            <alias><family>sans-serif</family><prefer><family>DejaVu Sans</family></prefer></alias>
            <alias><family>serif</family><prefer><family>DejaVu Serif</family></prefer></alias>
            <alias><family>monospace</family><prefer><family>DejaVu Sans Mono</family></prefer></alias>
          </fontconfig>
        '';
      };
    };
}
