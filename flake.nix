{
  description = "Flux - High-performance chemical process simulator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        buildInputs = with pkgs; [
          # Core dependencies
          rustToolchain
          pkg-config

          # Scientific computing libs
          openblas
          lapack

          # Potential thermodynamic property libs
          gsl

          # Build tools
          cmake
          clang

          # Development tools
          cargo-watch
          cargo-edit
          cargo-outdated
          cargo-audit
          cargo-criterion
          bacon
          hyperfine
          valgrind
          heaptrack

          # Documentation
          mdbook
          graphviz
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          shellHook = ''
            echo "Flux Development Environment"
            echo "Rust: $(rustc --version)"
            echo "Cargo: $(cargo --version)"
          '';

          RUST_BACKTRACE = 1;
          RUST_LOG = "flux=debug";
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "flux";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [
            openblas
            lapack
          ];
        };
      }
    );
}
