{ rustPlatform }:

let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  version = cargoToml.package.version;
in
rustPlatform.buildRustPackage {
  inherit version;
  pname = "onyx";
  src = ./..;
  cargoLock.lockFile = ../Cargo.lock;
}
