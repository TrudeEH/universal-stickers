{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = with pkgs; [
    cargo
    pkg-config
    rustc
    gtk4
    libadwaita
    xdg-desktop-portal-gnome
  ];

  shellHook = ''
    if [ -z "''${UNIVERSAL_STICKERS_SKIP_RUN:-}" ]; then
      ./run.sh
    fi
  '';
}
