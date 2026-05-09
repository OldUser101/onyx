{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { self, nixpkgs }:
    let
      overlays = import ./nix/overlay.nix;

      forAllSystems =
        f:
        nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ overlays.onyx ];
            }
          )
        );
    in
    {
      overlays = {
        default = overlays.onyx;
        onyx = overlays.onyx;
      };

      packages = forAllSystems (pkgs: rec {
        default = onyx;
        onyx = pkgs.onyx;
      });

      devShell = forAllSystems (
        pkgs:
        with pkgs;
        mkShell {
          buildInputs = [
            cargo
            rustc
            rustfmt
            rust-analyzer
            pre-commit
            rustPackages.clippy
          ];

          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        }
      );
    };
}
