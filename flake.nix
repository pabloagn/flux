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

        # ─────────────── Toolchains & common deps ────────────────
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };
        pythonBase = pkgs.python311;

        systemPackages = with pkgs; [
          pkg-config
          openssl
          cmake
          gcc
          stdenv.cc.cc.lib
          uv
          clickhouse
          kcat
          git
          jq
          yq-go
          httpie
          dive
          lazydocker
          docker-compose
          btop
        ];

        # ───────────────────── docker-compose ─────────────────────
        dockerComposeFile = pkgs.writeText "docker-compose.yml" ''
          services:
            kafka:
              image: confluentinc/cp-kafka:7.5.5
              environment:
                KAFKA_BROKER_ID: "1"
                KAFKA_ZOOKEEPER_CONNECT: "zookeeper:2181"
                KAFKA_LISTENERS: "PLAINTEXT://0.0.0.0:9092,PLAINTEXT_INTERNAL://0.0.0.0:9093"
                KAFKA_ADVERTISED_LISTENERS: "PLAINTEXT://localhost:9092,PLAINTEXT_INTERNAL://kafka:9093"
                KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: "PLAINTEXT:PLAINTEXT,PLAINTEXT_INTERNAL:PLAINTEXT"
                KAFKA_INTER_BROKER_LISTENER_NAME: "PLAINTEXT_INTERNAL"
                KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: "1"
              ports: [ "9092:9092" ]
              depends_on:
                - zookeeper

            zookeeper:
              image: confluentinc/cp-zookeeper:latest
              environment:
                ZOOKEEPER_CLIENT_PORT: "2181"
                ZOOKEEPER_TICK_TIME: "2000"
              ports: [ "2181:2181" ]

            clickhouse:
              image: clickhouse/clickhouse-server:latest
              ports: [ "8123:8123", "9000:9000" ]
              environment:
                CLICKHOUSE_DB: flux
                CLICKHOUSE_USER: default
                CLICKHOUSE_PASSWORD: ""
                CLICKHOUSE_DEFAULT_ACCESS_MANAGEMENT: "1"
              volumes:
                - ./config/clickhouse:/docker-entrypoint-initdb.d:ro

            nats:
              image: nats:alpine
              command: [ "-js" ]
              ports: [ "4222:4222", "8222:8222" ]

            glassflow:
              image: glassflow/clickhouse-etl-be:stable
              ports: [ "8080:8080" ]
              environment:
                GLASSFLOW_LOG_FILE_PATH: /tmp/logs/glassflow
                GLASSFLOW_NATS_SERVER: nats:4222
              depends_on:
                - kafka
                - clickhouse
                - nats
        '';

        # ───────────────────── helper scripts ────────────────────
        setupScript = pkgs.writeShellScriptBin "flux-setup" ''
          echo "Initialising Flux workspace…"
          FLUX_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
          mkdir -p $FLUX_ROOT/poc/data/{sample,results} $FLUX_ROOT/poc/config/development
          cp ${dockerComposeFile} $FLUX_ROOT/poc/docker-compose.yml
          echo "Run 'flux-start' to boot services."
        '';

        startScript = pkgs.writeShellScriptBin "flux-start" ''
          echo "Starting Flux stack…"
          FLUX_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
          cd $FLUX_ROOT/poc && docker-compose up -d
          echo "Waiting for containers…"; sleep 10
          echo "↳ Kafka      : localhost:9092"
          echo "↳ ClickHouse : localhost:8123"
          echo "↳ GlassFlow  : localhost:8080"
        '';

        stopScript = pkgs.writeShellScriptBin "flux-stop" ''
          echo "Stopping Flux stack…"
          FLUX_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
          cd $FLUX_ROOT/poc && docker-compose down
          echo "Stack stopped."
        '';

        statusScript = pkgs.writeShellScriptBin "flux-status" ''
          #!/usr/bin/env bash
          set -euo pipefail
          FLUX_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
          cyan='\033[36;1m'; green='\033[32;1m'; red='\033[31;1m'; reset='\033[0m'
          echo -e "''${cyan}── FLUX STACK STATUS ───────────────────────────''${reset}"
          cd $FLUX_ROOT/poc && docker compose ps --format '{{.Name}}\t{{.State}}' | \
          while IFS=$'\t' read -r c state; do
            printf "%-25s %s\n" "''${c}" \
              "$( [[ ''${state} =~ running ]] && echo -e "''${green}ONLINE''${reset}" || echo -e "''${red}OFFLINE''${reset}" )"
          done
          echo -e "''${cyan}──────────────────────────────────────────────────''${reset}"
        '';

        # native-library path needed by NumPy / SciPy wheels
        libsPath = "''${pkgs.stdenv.cc.cc.lib}/lib:''${pkgs.zlib}/lib";
      in
      {
        # ───────────────────── dev-shells ─────────────────────────
        devShells.default = pkgs.mkShell {
          buildInputs = systemPackages ++ [
            pythonBase
            rustToolchain
            setupScript
            startScript
            stopScript
            statusScript
          ];
          shellHook = ''
            export LD_LIBRARY_PATH='${libsPath}':$LD_LIBRARY_PATH
            export FLUX_ROOT=$PWD
            export PYTHONPATH="''${FLUX_ROOT}/poc:$PYTHONPATH"
            export RUST_BACKTRACE=1
            echo "Flux dev-shell ready → run 'flux-start'."
          '';
        };

        devShells.poc = pkgs.mkShell {
          buildInputs =
            [
              pythonBase
              pkgs.uv
            ]
            ++ systemPackages
            ++ [
              startScript
              stopScript
              statusScript
              setupScript
            ];
          shellHook = ''
            export LD_LIBRARY_PATH='${libsPath}':$LD_LIBRARY_PATH
            [ -d .venv ] || uv venv
            source .venv/bin/activate
            export PYTHONPATH=$PWD:$PYTHONPATH
            echo "POC shell (Python) ready."
          '';
        };

        devShells.core = pkgs.mkShell {
          buildInputs =
            [ rustToolchain ]
            ++ systemPackages
            ++ [
              startScript
              stopScript
            ];
          shellHook = ''
            export LD_LIBRARY_PATH='${libsPath}':$LD_LIBRARY_PATH
            cd core
            echo "Core shell (Rust) ready."
          '';
        };

        # ───────────────────── packages (unchanged) ──────────────
        packages = {
          poc = pkgs.stdenv.mkDerivation {
            pname = "flux-poc";
            version = "0.1.0";
            src = ./poc;
            buildInputs = [ pythonBase ];
            installPhase = ''
              mkdir -p $out/{bin,lib}
              cp -r flux $out/lib
              cat > $out/bin/flux-poc <<EOF
              #!${pkgs.bash}/bin/bash
              export PYTHONPATH=$out/lib:\$PYTHONPATH
              ${pythonBase}/bin/python $out/lib/flux/main.py "\$@"
              EOF
              chmod +x $out/bin/flux-poc
            '';
          };

          core = pkgs.rustPlatform.buildRustPackage {
            pname = "flux-core";
            version = "0.1.0";
            src = ./core;
            cargoLock.lockFile = ./core/Cargo.lock;
            buildInputs = with pkgs; [
              openssl
              pkg-config
            ];
          };

          docker-poc = pkgs.dockerTools.buildImage {
            name = "flux-poc";
            tag = "latest";
            contents = [ self.packages.${system}.poc ];
            config.Cmd = [ "/bin/flux-poc" ];
          };

          docker-core = pkgs.dockerTools.buildImage {
            name = "flux-core";
            tag = "latest";
            contents = [ self.packages.${system}.core ];
            config.Cmd = [ "/bin/flux-core" ];
          };
        };

        apps = {
          poc = flake-utils.lib.mkApp { drv = self.packages.${system}.poc; };
          core = flake-utils.lib.mkApp { drv = self.packages.${system}.core; };
        };
      }
    );
}
