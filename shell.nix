let
  nixpkgsRev = "605f21e375f0065868023874988164766e7896b0";
  nixpkgs = builtins.fetchTarball {
    name = "nixpkgs-${nixpkgsRev}";
    url = "https://github.com/nixos/nixpkgs/archive/${nixpkgsRev}.tar.gz";
    sha256 = "1mbm0nxkysxi9p4d4v2h6p32rni3wl10qm9hlqa2dy25yzamvjm9";
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
    nodejs
    pkgs.ninja
  ] ++ (lib.optionals pkgs.stdenv.isDarwin [
    # Fixes for MacOS Catalina
    # https://github.com/NixOS/nixpkgs/issues/120688
    pkgs.libiconv
    pkgs.darwin.apple_sdk.frameworks.CoreServices
  ]);
}
