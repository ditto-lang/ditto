# nix repl ./shell-nixpkgs.nix
let
  nixpkgsRev = "c1da6fc4ce95fe59f2c0c8e7cee580a37e0bb94b";
  nixpkgs = builtins.fetchTarball {
    name = "nixpkgs-${nixpkgsRev}";
    url = "https://github.com/nixos/nixpkgs/archive/${nixpkgsRev}.tar.gz";
    sha256 = "15s8cg7n6b7l8721s912733y7qybjvj73n5gsjx31707b3qn38gn";
  };
in import nixpkgs
