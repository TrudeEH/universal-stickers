{ pkgs ? import <nixpkgs> {} }:

let
  optionalKf6GlobalAccel =
    if pkgs ? kdePackages && pkgs.kdePackages ? kglobalaccel
    then [ pkgs.kdePackages.kglobalaccel ]
    else [];
in
pkgs.mkShell {
  packages = with pkgs; [
    cargo
    cmake
    ninja
    pkg-config
    rustc
    qt6.qtbase
    qt6.qtsvg
    qt6.wrapQtAppsHook
  ] ++ optionalKf6GlobalAccel;

  shellHook = ''
    export CMAKE_PREFIX_PATH="${pkgs.qt6.qtbase}:${pkgs.qt6.qtsvg}:$CMAKE_PREFIX_PATH"

    if [ -z "''${UNIVERSAL_STICKERS_SKIP_RUN:-}" ]; then
      ./run.sh
    fi
  '';
}
