let pkgs = import ../../nixpkgs.nix { };
in pkgs.mkShell {
  buildInputs = [
    (pkgs.rWrapper.override {
      packages = with pkgs.rPackages; [
        languageserver
        jsonlite
        dplyr
        ggplot2
      ];
    })
  ];
}
