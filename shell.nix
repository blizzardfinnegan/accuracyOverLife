{pkgs ? import <nixpkgs> {} }:
  pkgs.mkShell {
    nativeBuildInputs = with pkgs; [ cargo clang libclang rustc pkg-config libudev-zero tesseract opencv leptonica ];
}
