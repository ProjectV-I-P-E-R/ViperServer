{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            pkg-config
            openssl
            protobuf
            rustup
            just
            redis
          ];

          env = {
            PROTOBUF_LOCATION = "${pkgs.protobuf}";
            PROTOC = "${pkgs.protobuf}/bin/protoc";
            PROTOC_INCLUDE = "${pkgs.protobuf}/include";
          };
        };
      }
    );
}