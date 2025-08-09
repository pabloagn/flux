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

        # Rust Environments
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

        buildRustApp =
          { pname, src }:
          pkgs.rustPlatform.buildRustPackage {
            inherit pname;
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
              which # Add which command
              findutils
              gawk
              gnused
            ];

            # Patch vendor directory scripts
            prePatch = ''
              # Find and patch all shell and python scripts in vendor directory
              echo "Patching vendor directory for rdkafka-sys..."
              if [ -d "$NIX_BUILD_TOP/cargo-vendor-dir" ]; then
                # Patch shell scripts
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

        # Rust Image Builder Function
        buildRustImage =
          { pname, bin }:
          pkgs.dockerTools.buildImage {
            name = pname;
            tag = "latest";
            copyToRoot = [ bin ];
            config = {
              Cmd = [ "/bin/${pname}" ];
            };
          };

        # Python Image Builder Function
        buildPythonImage =
          {
            pname,
            src,
            pythonEnv,
          }:
          let
            pythonApp = pkgs.python311.pkgs.buildPythonApplication {
              inherit pname src;
              version = "0.1.0";
              pyproject = true;
              nativeBuildInputs = [ pkgs.python311.pkgs.setuptools ];
              buildInputs = [ pythonEnv ];
            };
          in
          pkgs.dockerTools.buildImage {
            name = pname;
            tag = "latest";
            copyToRoot = [
              pkgs.bash
              pythonApp
            ];
            config = {
              Cmd = [ "${pythonApp}/bin/${pname}" ];
              WorkingDir = "/";
              Env = [ "PYTHONUNBUFFERED=1" ];
            };
          };

      in
      {
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

        packages = {
          questdb-with-healthcheck = pkgs.dockerTools.buildImage {
            name = "flux-questdb";
            tag = "7.3.10-custom";
            fromImage = pkgs.dockerTools.pullImage {
              imageName = "questdb/questdb";
              sha256 = "sha256-V4G+ah+ofZGomsEG1ztWJaQju3P4XbwFemHiNIMAHa4=";
              imageDigest = "sha256:2a0408813dee86aa6e0d38f6d4411ea2918c6be3e45f3802f3a11f1e8000635";
            };
            copyToRoot = [ pkgs.curl ];
          };

          # NOTE: The `src` for all Rust apps is now implicitly `./.` via the `buildRustApp` function
          operator-tui-bin = buildRustApp {
            pname = "operator-tui";
            src = ./apps/operator-tui;
          };
          operator-tui = buildRustImage {
            pname = "operator-tui";
            bin = self.packages.${system}.operator-tui-bin;
          };

          # audit-logger-bin = buildRustApp {
          #   pname = "audit-logger";
          #   src = ./services/audit-logger;
          # };
          # audit-logger = buildRustImage {
          #   pname = "audit-logger";
          #   bin = self.packages.${system}.audit-logger-bin;
          # };

          # control-system-bin = buildRustApp {
          #   pname = "control-system";
          #   src = ./services/control-system;
          # };
          # control-system = buildRustImage {
          #   pname = "control-system";
          #   bin = self.packages.${system}.control-system-bin;
          # };

          # safety-interlock-bin = buildRustApp {
          #   pname = "safety-interlock";
          #   src = ./services/safety-interlock;
          # };
          # safety-interlock = buildRustImage {
          #   pname = "safety-interlock";
          #   bin = self.packages.${system}.safety-interlock-bin;
          # };

          # state-manager-bin = buildRustApp {
          #   pname = "state-manager";
          #   src = ./services/state-manager;
          # };
          # state-manager = buildRustImage {
          #   pname = "state-manager";
          #   bin = self.packages.${system}.state-manager-bin;
          # };

          # alarm-manager = buildPythonImage {
          #   pname = "alarm-manager";
          #   src = ./services/alarm-manager;
          #   pythonEnv = pythonEnv;
          # };

          data-pipeline = buildPythonImage {
            pname = "data-pipeline";
            src = ./services/data-pipeline;
            pythonEnv = pythonEnvForDataPipeline;
          };

          # historian = buildPythonImage {
          #   pname = "historian";
          #   src = ./services/historian;
          #   pythonEnv = pythonEnv;
          # };

          kpi-engine = buildPythonImage {
            pname = "kpi-engine";
            src = ./services/kpi-engine;
            pythonEnv = pythonEnvForKpiEngine;
          };

          # ml-platform = buildPythonImage {
          #   pname = "ml-platform";
          #   src = ./services/ml-platform;
          #   pythonEnv = pythonEnv;
          # };

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

          all-images = pkgs.buildEnv {
            name = "all-flux-images";
            paths = with self.packages.${system}; [
              questdb-with-healthcheck
              operator-tui
              # audit-logger
              # control-system
              # safety-interlock
              # state-manager
              # alarm-manager
              data-pipeline
              # historian
              kpi-engine
              # ml-platform
            ];
          };
        };

        apps = {
          simulator = flake-utils.lib.mkApp { drv = self.packages.${system}.simulator; };
          tui = flake-utils.lib.mkApp { drv = self.packages.${system}.operator-tui-bin; };
          default = self.apps.${system}.tui;
        };
      }
    );
}
