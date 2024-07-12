{ pkgs ? import <nixpkgs> { } }:
let
  odin-stx = pkgs.gcc13Stdenv.mkDerivation (rec {
    name = "stx";
    src = ./.;
    dontConfigure = true;
    nativeBuildInputs = [ pkgs.git ];
  });
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [     
    bintools
    llvm
    gcc13
    libgcc
  ];
}