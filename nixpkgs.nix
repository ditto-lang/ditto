let
  nixpkgsRev = "4a66f421319b72adf90c79d5b9ac8b909b29a771";
  nixpkgs = builtins.fetchTarball {
    name = "nixpkgs-${nixpkgsRev}";
    url = "https://github.com/nixos/nixpkgs/archive/${nixpkgsRev}.tar.gz";
    sha256 = "1cyc16zjjc2pw8jhs4fsf763xckn63zimd07z3h45k7kz9dq79k3";
  };
in
import nixpkgs
