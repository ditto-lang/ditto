let
  nixpkgsRev = "c1da6fc4ce95fe59f2c0c8e7cee580a37e0bb94b";
  nixpkgs = builtins.fetchTarball {
    name = "nixpkgs-${nixpkgsRev}";
    url = "https://github.com/nixos/nixpkgs/archive/${nixpkgsRev}.tar.gz";
    sha256 = "15s8cg7n6b7l8721s912733y7qybjvj73n5gsjx31707b3qn38gn";
  };
  pkgs = import nixpkgs { };
  lib = pkgs.lib;

  fenixRev = "9e3384c61656487b10226a3366a12c37393b21d9";
  fenixPackages = import (builtins.fetchTarball {
    name = "fenix-${fenixRev}";
    url = "https://github.com/nix-community/fenix/archive/${fenixRev}.tar.gz";
    sha256 = "10kjfa00fs98cvs137x5kr5dmfblmkz8ya5ribb5l0dnfnpgvf5s";
  }) { };

  rustToolchain = fenixPackages.fromToolchainFile {
    file = ./rust-toolchain.toml;
    sha256 = "5LJRZfmLYjC5UZBmZoLomMz9P5OTjP+te5TI9RX8gZI=";
  };
  inherit (fenixPackages) rust-analyzer;

  cargo-benchcmp = pkgs.rustPlatform.buildRustPackage rec {
    pname = "cargo-benchcmp";
    version = "0.4.3";
    src = pkgs.fetchFromGitHub {
      owner = "BurntSushi";
      repo = pname;
      rev = version;
      sha256 = "sha256-nD/qFqq1DOmNZGW4g9Xjpwob/T7d6egFdFMNFG+N+f0=";
    };
    cargoSha256 = "sha256-frNoGzeOPo/gUksaquiFdRhUd486BABcoznW29FzIK8=";
    doCheck = false;
  };

  # Should match .nvmrc
  # Also see: https://nixos.wiki/wiki/Node.js#Example_nix_shell_for_Node.js_development
  # (but note building Node from source takes aaages)
  nodejs = pkgs.nodejs-16_x;
in pkgs.mkShell {
  buildInputs = [
    rustToolchain
    rust-analyzer
    pkgs.cargo-watch
    pkgs.cargo-udeps
    pkgs.cargo-audit
    pkgs.cargo-outdated
    pkgs.cargo-tarpaulin
    cargo-benchcmp
    nodejs
    pkgs.ninja
    pkgs.openssl
    pkgs.pkg-config
  ] ++ (lib.optionals pkgs.stdenv.isDarwin [
    # Fixes for MacOS Catalina
    # https://github.com/NixOS/nixpkgs/issues/120688
    pkgs.libiconv
    pkgs.darwin.apple_sdk.frameworks.CoreServices
  ]);
}
