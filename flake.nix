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

        # ─────────── Toolchains & common deps ───────────
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        pythonBase = pkgs.python311;

        systemPackages = with pkgs; [
          btop
          clickhouse
          cmake
          dive
          docker-compose
          gcc
          git
          httpie
          jq
          kcat
          confluent-platform # Provides kafka-console-consumer
          lazydocker
          openssl
          pkg-config
          stdenv.cc.cc.lib
          uv
          yq-go
        ];

        # ─────────────── docker-compose file ─────────────
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
              depends_on: [ zookeeper ]

            zookeeper:
              image: confluentinc/cp-zookeeper:latest
              environment:
                ZOOKEEPER_CLIENT_PORT: "2181"
                ZOOKEEPER_TICK_TIME:  "2000"
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
              depends_on: [ kafka, clickhouse, nats ]
        '';

        # ─────────────── Helper Scripts ────────────────
        setupScript = pkgs.writeShellScriptBin "flux-setup" ''
          FLUX_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
          mkdir -p $FLUX_ROOT/infra/docker \
                   $FLUX_ROOT/data/{sample,results} \
                   $FLUX_ROOT/config/schemas/{clickhouse,kafka}
          cp ${dockerComposeFile} $FLUX_ROOT/infra/docker/docker-compose.yml
          echo "Run 'flux-start' to boot services."
        '';

        startScript = pkgs.writeShellScriptBin "flux-start" ''
          FLUX_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
          cd $FLUX_ROOT/infra/docker && docker-compose up -d
          echo "Waiting for containers…"; sleep 10
          echo "↳ Kafka      : localhost:9092"
          echo "↳ ClickHouse : localhost:8123"
          echo "↳ GlassFlow  : localhost:8080"
        '';

        stopScript = pkgs.writeShellScriptBin "flux-stop" ''
          FLUX_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
          cd $FLUX_ROOT/infra/docker && docker-compose down
          echo "Stack stopped."
        '';

        statusScript = pkgs.writeShellScriptBin "flux-status" ''
          set -euo pipefail
          FLUX_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
          cyan='\033[36;1m'; green='\033[32;1m'; red='\033[31;1m'; reset='\033[0m'
          echo -e "''${cyan}── FLUX STACK STATUS ───────────────────────────''${reset}"
          cd $FLUX_ROOT/infra/docker
          docker compose ps --format '{{.Name}}\t{{.State}}' |
          while IFS=$'\t' read -r c state; do
            printf "%-25s %s\n" "''${c}" \
              "$( [[ ''${state} =~ running ]] && echo -e "''${green}ONLINE''${reset}" || echo -e "''${red}OFFLINE''${reset}" )"
          done
          echo -e "''${cyan}──────────────────────────────────────────────────''${reset}"
        '';

        libsPath = "${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.zlib}/lib";

        # ─────────────── Shells ───────────────
        shells = import ./nix {
          inherit
            pkgs
            rustToolchain
            pythonBase
            libsPath
            systemPackages
            setupScript
            startScript
            stopScript
            statusScript
            ;
        };
      in
      {
        # ─────────────── Development Shells ───────────────
        devShells.default = shells.default;
        devShells.simulator = shells.simulator;
        devShells.tui = shells.tui;
        devShells.kpi = shells.kpi;

        # ─────────────── Packages ───────────────
        packages = {
          # Simulator service package
          simulator = pkgs.stdenv.mkDerivation {
            pname = "flux-simulator";
            version = "0.1.0";
            src = ./services/simulator;
            buildInputs = [ pythonBase ];
            installPhase = ''
              mkdir -p $out/{bin,lib}
              cp -r flux $out/lib
              cat > $out/bin/flux-simulator <<EOF
              #!${pkgs.bash}/bin/bash
              export PYTHONPATH=$out/lib:\$PYTHONPATH
              ${pythonBase}/bin/python $out/lib/flux/flux.py "\$@"
              EOF
              chmod +x $out/bin/flux-simulator
            '';
          };

          # KPI Engine service package
          kpi-engine = pkgs.stdenv.mkDerivation {
            pname = "flux-kpi-engine";
            version = "0.1.0";
            src = ./services/kpi-engine;
            buildInputs = [ pythonBase ];
            installPhase = ''
              mkdir -p $out/{bin,lib}
              cp -r src $out/lib
              cat > $out/bin/flux-kpi <<EOF
              #!${pkgs.bash}/bin/bash
              export PYTHONPATH=$out/lib:\$PYTHONPATH
              ${pythonBase}/bin/python -m $out/lib/__main__.py "\$@"
              EOF
              chmod +x $out/bin/flux-kpi
            '';
          };

          # Operator TUI application
          operator-tui = pkgs.rustPlatform.buildRustPackage {
            pname = "flux-operator-tui";
            version = "0.1.0";
            src = ./apps/operator-tui;
            cargoLock.lockFile = ./apps/operator-tui/Cargo.lock;
            buildInputs = with pkgs; [
              openssl
              pkg-config
            ];
          };

          # Docker images
          docker-simulator = pkgs.dockerTools.buildImage {
            name = "flux-simulator";
            tag = "latest";
            contents = [ self.packages.${system}.simulator ];
            config.Cmd = [ "/bin/flux-simulator" ];
          };

          docker-kpi = pkgs.dockerTools.buildImage {
            name = "flux-kpi-engine";
            tag = "latest";
            contents = [ self.packages.${system}.kpi-engine ];
            config.Cmd = [ "/bin/flux-kpi" ];
          };

          docker-tui = pkgs.dockerTools.buildImage {
            name = "flux-operator-tui";
            tag = "latest";
            contents = [ self.packages.${system}.operator-tui ];
            config.Cmd = [ "/bin/flux-operator-tui" ];
          };
        };

        # ─────────────── Apps ───────────────
        apps = {
          simulator = flake-utils.lib.mkApp { drv = self.packages.${system}.simulator; };
          kpi = flake-utils.lib.mkApp { drv = self.packages.${system}.kpi-engine; };
          tui = flake-utils.lib.mkApp { drv = self.packages.${system}.operator-tui; };
        };
      }
    );
}
