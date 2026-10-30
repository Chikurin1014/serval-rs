# The shell to work on the apps in (`nix develop`), with all the tools.
{
  imports = [
    ./common.nix
    ./web.nix
  ];

  perSystem =
    {
      config,
      pkgs,
      common,
      ...
    }:
    {
      devShells.default = pkgs.mkShell (
        common.playwrightEnv
        // {
          # What the apps build with: the web app's tools, and the libraries
          # both of them use
          inputsFrom = [ config.packages.web-app ];
          packages = [
            common.rustToolchain
            pkgs.cargo-about # `third-party-licenses.html` of each app (see `about.toml`)
            pkgs.mdbook # `docs/`
            pkgs.mdbook-mermaid # its diagrams
            pkgs.ffmpeg # `docs/media/capture.py`
            pkgs.nodejs
            pkgs.prettier
            pkgs.pyright
            pkgs.ruff
            pkgs.taplo
            pkgs.vscode-css-languageserver
            pkgs.vscode-json-languageserver
            common.python
          ];
        }
      );
    };
}
