# The documents (`docs/`, of mdBook), built apart: of `docs/` alone, so built
# anew only as it changes. The web app serves them at `/docs/`.
{
  perSystem =
    { config, pkgs, ... }:
    {
      packages.docs =
        pkgs.runCommand "serval-docs"
          {
            nativeBuildInputs = [ pkgs.mdbook ];
            src = pkgs.lib.fileset.toSource {
              root = ../docs;
              fileset = ../docs;
            };
          }
          ''
            mdbook build "$src" --dest-dir "$out"
          '';

      checks.docs = config.packages.docs;
    };
}
