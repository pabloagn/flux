{
  description = "Flux: A High-Performance Electrochemical-Processes Digital Twin";

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
        pkgs = import nixpkgs { inherit system overlays; };

        # --- Base Toolchains & Environments ---
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        # --- Python Environments ---
        pythonEnv = pkgs.python311.withPackages (
          ps: with ps; [
            hatchling
            numpy
            uv
          ]
        );
        pythonEnvForKpiEngine = pkgs.python311.withPackages (
          ps: with ps; [
            aiokafka
            click
            clickhouse-driver
            hatchling
            numpy
            pydantic
            uv
          ]
        );
        pythonEnvForDataPipeline = pkgs.python311.withPackages (
          ps: with ps; [
            hatchling
            orjson
            prometheus-client
            psycopg
            structlog
            tenacity
            uvloop
          ]
        );

      in
      {
        # --- Development Shells ---
        devShells = import ./nix {
          inherit pkgs rustToolchain;
          pythonBase = pythonEnv;
          libsPath = "${pkgs.stdenv.cc.cc.lib}/lib";
          systemPackages = with pkgs; [
            btop
            cmake
            confluent-platform
            curl
            dive
            docker-compose
            gcc
            git
            httpie
            jq
            just
            kcat
            lazydocker
            openssl
            pkg-config
            skopeo
            yq-go
          ];
        };

        # --- Packages and Docker Images ---
        packages = {

          # --- Custom Questdb Image ---
          questdb-with-healthcheck =
            let
              entrypointScript = pkgs.writeShellScriptBin "questdb-entrypoint" ''
                #!${pkgs.bash}/bin/bash
                set -euo pipefail
                /app/bin/java -m io.questdb/io.questdb.ServerMain -d /var/lib/questdb &
                QDB_PID=$!
                echo "--> Waiting for QuestDB to become available..."
                while ! ${pkgs.postgresql}/bin/pg_isready -h localhost -p 8812 -q -U admin; do
                  sleep 1
                done
                echo "--> QuestDB is ready."
                INIT_SCRIPT="/docker-entrypoint-initdb.d/init.sql"
                if [ -f "$INIT_SCRIPT" ]; then
                  echo "--> Found initialization script, executing: $INIT_SCRIPT"
                  PGPASSWORD=quest ${pkgs.postgresql}/bin/psql \
                    -h localhost \
                    -p 8812 \
                    -U admin \
                    -d qdb \
                    -v ON_ERROR_STOP=1 \
                    -a \
                    -f "$INIT_SCRIPT"
                  echo "--> Initialization script finished."
                else
                  echo "--> No initialization script found."
                fi
                wait $QDB_PID
              '';
            in
            pkgs.dockerTools.buildImage {
              name = "flux-questdb";
              tag = "7.3.10-custom";
              fromImage = pkgs.dockerTools.pullImage {
                imageName = "questdb/questdb";
                imageDigest = "sha256:2a0408813dee86aa6e0d38f6d4411ea2918c6be3e45f3802f3a11f1e8000635b";
                sha256 = "sha256-V4G+ah+ofZGomsEG1ztWJaQju3P4XbwFemHiNIMAHa4=";
              };

              copyToRoot = with pkgs; [
                curl
                postgresql
              ];

              config = {
                Entrypoint = [ "${entrypointScript}/bin/questdb-entrypoint" ];
                Cmd = [ ];
                WorkingDir = "/var/lib/questdb";
                ExposedPorts = {
                  "9000/tcp" = { };
                  "8812/tcp" = { };
                  "9009/tcp" = { };
                };
              };
            };

          # --- Operator TUI ---
          operator-tui-bin = pkgs.rustPlatform.buildRustPackage {
            pname = "flux-operator-tui";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            buildInputs = with pkgs; [
              openssl
              openssl.dev
              zlib
              cyrus_sasl
              lz4
              zstd
              curl
            ];

            nativeBuildInputs = with pkgs; [
              pkg-config
              cmake
              bash
              gnumake
              gcc
              perl
              python3
              autoconf
              automake
              libtool
              coreutils
              which
              findutils
              gawk
              gnused
            ];

            # Patch vendor directory scripts
            prePatch = ''
              echo "Patching vendor directory for rdkafka-sys..."
              if [ -d "$NIX_BUILD_TOP/cargo-vendor-dir" ]; then
                find "$NIX_BUILD_TOP/cargo-vendor-dir" -type f \( -name "configure" -o -name "*.sh" \) | while read script; do
                  if [ -f "$script" ]; then
                    echo "Patching shell script: $script"
                    chmod +x "$script"
                    sed -i "1s|^#!.*|#!${pkgs.bash}/bin/bash|" "$script"
                  fi
                done
                
                # Patch Python scripts
                find "$NIX_BUILD_TOP/cargo-vendor-dir" -type f -name "*.py" | while read script; do
                  if [ -f "$script" ]; then
                    echo "Patching Python script: $script"
                    chmod +x "$script"
                    sed -i "1s|^#!/usr/bin/env python.*|#!${pkgs.python3}/bin/python3|" "$script"
                  fi
                done
              fi
            '';

            preBuild = ''
              # Setup paths for the build
              export LIBRARY_PATH="${pkgs.openssl}/lib:${pkgs.zlib}/lib:${pkgs.lz4}/lib:${pkgs.zstd}/lib:$LIBRARY_PATH"
              export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.zlib}/lib/pkgconfig:$PKG_CONFIG_PATH"

              # Ensure all tools are available
              export PATH="${pkgs.bash}/bin:${pkgs.coreutils}/bin:${pkgs.gnumake}/bin:${pkgs.gcc}/bin:${pkgs.which}/bin:${pkgs.python3}/bin:$PATH"
              export CONFIG_SHELL="${pkgs.bash}/bin/bash"
              export SHELL="${pkgs.bash}/bin/bash"

              # Compiler settings
              export CC="${pkgs.gcc}/bin/gcc"
              export CXX="${pkgs.gcc}/bin/g++"
              export AR="${pkgs.gcc}/bin/ar"

              # Enable rdkafka features
              export CARGO_FEATURE_SSL=1
              export CARGO_FEATURE_ZSTD=1
            '';

            # Also patch after unpacking source
            postPatch = ''
              patchShebangs .
            '';
          };

          operator-tui = pkgs.dockerTools.buildImage {
            name = "flux-operator-tui";
            tag = "latest";
            copyToRoot = [ self.packages.${system}.operator-tui-bin ];
            config.Cmd = [ "/bin/flux-operator-tui" ];
          };

          # --- Data Pipeline ---
          data-pipeline =
            let
              pname = "data-pipeline";
              pythonApp = pkgs.python311.pkgs.buildPythonApplication {
                inherit pname;
                version = "0.1.0";
                src = ./services/data-pipeline;
                pyproject = true;
                nativeBuildInputs = with pkgs.python311.pkgs; [
                  setuptools
                  wheel
                ];
                propagatedBuildInputs = with pkgs.python311.pkgs; [
                  aiokafka
                  orjson
                  prometheus-client
                  psycopg
                  psycopg-pool
                  pydantic
                  structlog
                  tenacity
                  uvloop
                ];
                pythonImportsCheck = [ "data_pipeline" ];
              };
            in
            pkgs.dockerTools.buildImage {
              name = "flux-data-pipeline";
              tag = "latest";
              copyToRoot = [ pythonApp ];
              config.Cmd = [ "/bin/data-pipeline" ];
            };

          # KPI Engine
          kpi-engine =
            let
              pname = "kpi-engine";
              pythonApp = pkgs.python311.pkgs.buildPythonApplication {
                inherit pname;
                version = "0.1.0";
                src = ./services/kpi-engine;
                pyproject = true;
                nativeBuildInputs = with pkgs.python311.pkgs; [ hatchling ];
                propagatedBuildInputs = with pkgs.python311.pkgs; [
                  aiokafka
                  click
                  clickhouse-driver
                  numpy
                  pydantic
                ];
              };
            in
            pkgs.dockerTools.buildImage {
              name = "flux-kpi-engine";
              tag = "latest";
              copyToRoot = [ pythonApp ];
              config.Cmd = [ "${pythonApp}/bin/${pname}" ];
            };

          # 5. Simulator Package
          simulator = pkgs.stdenv.mkDerivation {
            pname = "flux-simulator";
            version = "0.1.0";
            src = ./services/simulator;
            nativeBuildInputs = [ pythonEnv ];
            installPhase = ''
              mkdir -p $out/bin
              cp -r flux $out/
              cat > $out/bin/flux-simulator <<EOF
              #!${pkgs.bash}/bin/bash
              export PYTHONPATH=$out:\$PYTHONPATH
              ${pythonEnv}/bin/python $out/flux/simulation/plant.py "\$@"
              EOF
              chmod +x $out/bin/flux-simulator
            '';
          };

          # 6. Meta-package to build all Docker images
          all-images = pkgs.stdenv.mkDerivation {
            name = "all-flux-images";

            # Don't use buildInputs for docker images!
            # buildInputs is for build dependencies like compilers, libraries, etc.

            phases = [ "installPhase" ];

            installPhase = ''
              mkdir -p $out

              # Just reference the images in the derivation to ensure they're built
              echo "All Docker images built:" > $out/images.txt
              echo "questdb: ${self.packages.${system}.questdb-with-healthcheck}" >> $out/images.txt
              echo "operator-tui: ${self.packages.${system}.operator-tui}" >> $out/images.txt
              echo "data-pipeline: ${self.packages.${system}.data-pipeline}" >> $out/images.txt
              echo "kpi-engine: ${self.packages.${system}.kpi-engine}" >> $out/images.txt
            '';
          };
        };

        # --- Runnable Apps ---
        apps = {
          simulator = flake-utils.lib.mkApp { drv = self.packages.${system}.simulator; };
          tui = flake-utils.lib.mkApp { drv = self.packages.${system}.operator-tui-bin; };
          default = self.apps.${system}.tui;
        };
      }
    );
}
