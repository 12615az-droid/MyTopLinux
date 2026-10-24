{ pkgs ? import (builtins.fetchTarball "https://github.com/NixOS/nixpkgs/archive/02c087f3fe673c9e317f206a627cb6b910336dbe.tar.gz") {} }:
pkgs.mkShell {
  packages = with pkgs; [ cargo rustc rustfmt clippy rust-analyzer pkg-config nodejs (python3.withPackages (p: [ p.pillow ])) xvfb-run xdotool ];
  buildInputs = with pkgs; [ gtk3 webkitgtk_4_1 ];
  # WebKit's helper processes also need the GSettings schemas in an unwrapped dev build.
  XDG_DATA_DIRS = "${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}";
  shellHook = ''
    echo "MyTop Linux · cargo run | cargo test | cargo clippy"
  '';
}
