{ ci-treefmt ? false
}:
let
  pkgs = import ./nixpkgs.nix { };

  inherit (pkgs) stdenv lib;

  fenixRev = "9e3384c61656487b10226a3366a12c37393b21d9";
  fenixPackages = import
    (builtins.fetchTarball {
      name = "fenix-${fenixRev}";
      url = "https://github.com/nix-community/fenix/archive/${fenixRev}.tar.gz";
      sha256 = "10kjfa00fs98cvs137x5kr5dmfblmkz8ya5ribb5l0dnfnpgvf5s";
    })
    { inherit pkgs; };

  rustToolchain = fenixPackages.fromToolchainFile {
    file = ./rust-toolchain.toml;
    sha256 = "sha256-lFKtXaRZQFPdQHIymMDEpQpWhbfUJpRj1+VHSzzVjn4=";
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

  cargo-llvm-cov = pkgs.rustPlatform.buildRustPackage rec {
    pname = "cargo-llvm-cov";
    version = "0.5.0";
    src = pkgs.fetchFromGitHub {
      owner = "taiki-e";
      repo = pname;
      rev = "v${version}";
      sha256 = "sha256-2O0MyL4SF/2AUpgWYUDWQ5dDpa84pwmnKGtAaWi5bwQ=";
    };
    cargoSha256 = "sha256-zQ1wgeKvc7q0pIx7ZWAQIayP/JVQGyFbLB3Iv81mbx0=";
    cargoPatches = [
      ./cargo-llvm-cov-cargo-lock.patch
    ];
    doCheck = false;
  };

  # Don't forget to update .github/actions/setup-haskell
  stack = pkgs.symlinkJoin {
    name = "stack-with-system-ghc";
    paths = [ pkgs.stack ];
    buildInputs = [ pkgs.makeWrapper ];
    postBuild = ''
      wrapProgram $out/bin/stack --add-flags "--system-ghc"
    '';
  };

  # Should match .nvmrc
  # Also see: https://nixos.wiki/wiki/Node.js#Example_nix_shell_for_Node.js_development
  # (but note building Node from source takes aaages)
  nodejs = pkgs.nodejs-18_x;
in
pkgs.mkShell {
  buildInputs = [
    # if `--arg ci-treefmt true` then we only want to include these tools
    # (used for running formatting checks in CI)
    pkgs.treefmt
    #pkgs.deadnix <-- might be nice to use in the future?
    pkgs.nixpkgs-fmt
    pkgs.ormolu
    pkgs.shellcheck
    pkgs.shfmt

  ] ++ lib.optionals (!ci-treefmt) ([
    # The rest of the development shell stuff
    rustToolchain
    rust-analyzer
    pkgs.cargo-nextest
    cargo-llvm-cov
    pkgs.cargo-watch
    pkgs.cargo-udeps
    pkgs.cargo-audit
    pkgs.cargo-outdated
    cargo-benchcmp

    # Haskell stuff
    stack
    pkgs.ghc
    pkgs.ghcid

    nodejs

    pkgs.ninja
    pkgs.openssl
    pkgs.pkg-config
  ]
  # Linux specific stuff
  ++ (lib.optionals (stdenv.isx86_64 && stdenv.isLinux) [ ])
  # MacOS specific stuff
  ++ (lib.optionals pkgs.stdenv.isDarwin [
    # Fixes for MacOS Catalina
    # https://github.com/NixOS/nixpkgs/issues/120688
    pkgs.libiconv
    pkgs.darwin.apple_sdk.frameworks.CoreServices
  ]));
  DITTO_NINJA = "${pkgs.ninja}/bin/ninja";
}
