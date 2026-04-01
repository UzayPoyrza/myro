{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default;
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.gcc
            pkgs.pkg-config
            pkgs.openssl
            pkgs.sqlite
            pkgs.nodejs
            pkgs.python3
            pkgs.prisma-engines
          ];

          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

          PRISMA_SCHEMA_ENGINE_BINARY =
            "${pkgs.prisma-engines_6}/bin/schema-engine";
          PRISMA_QUERY_ENGINE_BINARY =
            "${pkgs.prisma-engines_6}/bin/query-engine";
          PRISMA_QUERY_ENGINE_LIBRARY =
            "${pkgs.prisma-engines_6}/lib/libquery_engine.node";
          PRISMA_FMT_BINARY = "${pkgs.prisma-engines_6}/bin/prisma-fmt";

          LD_LIBRARY_PATH =
            pkgs.lib.makeLibraryPath [ pkgs.openssl pkgs.stdenv.cc.cc.lib ];
        };
      });
}
