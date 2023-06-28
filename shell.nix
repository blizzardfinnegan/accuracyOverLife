{pkgs ? import <nixpkgs> {} }:
  pkgs.mkShell {
    buildInputs = with pkgs; [ lld rustup ];
    RUSTUP_HOME = toString ./.rustup;
    CARGO_HOME = toString ./.cargo;
    RUSTUP_TOOLCHAIN = "stable";
    HOST_ARCH = "x86_64-unknown-linux-gnu";
    CARGO_BUILD_TARGET = "aarch64-unknown-linux-musl";
    shellHook = ''
      export PATH=$PATH:${CARGO_HOME}/bin
      export PATH=$PATH:${RUSTUP_HOME}/toolchains/${RUSTUP_TOOLCHAIN}-${HOST_ARCH}/bin/

      rustup target add "${HOST_ARCH}"
      rustup target add "${CARGO_BUILD_TARGET}"
      '';
}
