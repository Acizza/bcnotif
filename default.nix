with import <nixpkgs> {};

pkgs.mkShell {
    buildInputs = [ stdenv.cc sqlite.dev pkgconfig dbus ];
}
