# The desktop app (the `desktop` package of the workspace, of `ui`): its build
# and its app (`nix run`).
{
  imports = [
    ./common.nix
  ];

  perSystem =
    {
      config,
      pkgs,
      common,
      ...
    }:
    let
      inherit (common) craneLib commonArgs;

      args = commonArgs // {
        pname = "serval-desktop";
        nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ pkgs.dioxus-cli ];
        buildPhaseCargoCommand = "dx build --platform desktop --release --profile desktop-release --package desktop";
        doCheck = false;
      };
    in
    {
      packages.desktop = craneLib.buildPackage (
        args
        // {
          cargoArtifacts = craneLib.buildDepsOnly args;
          nativeBuildInputs = args.nativeBuildInputs ++ [ pkgs.wrapGAppsHook3 ];
          doNotPostBuildInstallCargoBinaries = true;
          # One file, as it is handed out
          installPhaseCommand = ''
            mkdir -p $out/bin
            cargo run --locked --package embed-assets -- target/dx/serval/release/linux/app/serval $out/bin/serval
          '';
          # Its assets are at the very end of the binary, which these would
          # cut off. It is stripped as it is built, and has its rpath
          dontStrip = true;
          dontPatchELF = true;
          # The save dialogs where there is no portal for them
          preFixup = ''
            gappsWrapperArgs+=(--suffix PATH : ${pkgs.lib.makeBinPath [ pkgs.zenity ]})
          '';
          meta.mainProgram = "serval";
        }
      );

      apps.desktop = {
        type = "app";
        program = pkgs.lib.getExe config.packages.desktop;
      };
    };
}
