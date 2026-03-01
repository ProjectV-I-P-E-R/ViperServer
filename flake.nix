{
    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    };

    outputs = { self, nixpkgs, ... }:
    let
        system = "x86_64-linux";
        pkgs = nixpkgs.legacyPackages.${system};
    in {
        devShells.${system}.default = pkgs.mkShell {
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
    };
}