{ pkgs ? import <nixpkgs> {} }:
with pkgs;
mkShell {
  nativeBuildInputs = [ cargo rustc rustfmt libiconv pkgconfig ];
}
