rec {
  default = onyx;
  onyx = final: prev: {
    onyx = prev.callPackage ./package.nix { };
  };
}
