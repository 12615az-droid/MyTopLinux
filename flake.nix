{
  description = "MyTop Linux: Rust + GTK/WebKitGTK development environment";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/02c087f3fe673c9e317f206a627cb6b910336dbe";
  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
    mytop = pkgs.rustPlatform.buildRustPackage {
      pname = "mytop-linux";
      version = "0.1.0";
      src = pkgs.lib.cleanSource ./.;
      cargoLock.lockFile = ./Cargo.lock;
      nativeBuildInputs = [ pkgs.pkg-config pkgs.wrapGAppsHook3 ];
      buildInputs = [ pkgs.gtk3 pkgs.webkitgtk_4_1 ];
      postInstall = ''
        install -Dm644 packaging/io.github.mytop.Linux.desktop $out/share/applications/io.github.mytop.Linux.desktop
        install -Dm644 ui/icon.png $out/share/icons/hicolor/192x192/apps/mytop-linux.png
      '';
      meta = {
        description = "Offline game collection for Linux";
        mainProgram = "mytop-linux";
        platforms = [ "x86_64-linux" ];
      };
    };
  in {
    devShells.${system}.default = import ./shell.nix { inherit pkgs; };
    packages.${system} = {
      default = mytop;
      mytop-linux = mytop;
    };
    apps.${system}.default = {
      type = "app";
      program = "${mytop}/bin/mytop-linux";
      meta.description = "Run MyTop Linux";
    };
  };
}
