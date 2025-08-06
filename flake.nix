{
  description = "Flux: A High Performance Electrochemical Processes Digital Twin";

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

        # Rust toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        # Python base
        pythonBase = pkgs.python311;

        # System dependencies
        systemPackages = with pkgs; [
          # --- Build Tools ---
          pkg-config
          openssl
          cmake
          gcc
          stdenv.cc.cc.lib

          # --- Python Package Manager ---
          uv # This IS in nixpkgs

          # --- Database Clients ---
          clickhouse
          kcat # formerly kafkacat

          # --- Development Tools ---
          git
          jq
          yq-go
          httpie
          dive
          lazydocker
          docker-compose

          # --- Monitoring ---
          btop
        ];

        # Docker compose for services
        dockerComposeFile = pkgs.writeText "docker-compose.yml" ''
          services:
            kafka:
              image: confluentinc/cp-kafka:7.5.5
              environment:
                KAFKA_BROKER_ID: 1
                KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
                KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://localhost:9092
                KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
              ports:
                - "9092:9092"
              depends_on:
                - zookeeper

            zookeeper:
              image: confluentinc/cp-zookeeper:latest
              environment:
                ZOOKEEPER_CLIENT_PORT: 2181
                ZOOKEEPER_TICK_TIME: 2000
              ports:
                - "2181:2181"

            clickhouse:
              image: clickhouse/clickhouse-server:latest
              ports:
                - "8123:8123"
                - "9000:9000"
              volumes:
                - ./config/clickhouse:/etc/clickhouse-server/config.d

            glassflow:
              image: glassflow/clickhouse-etl-be:stable
              environment:
                GLASSFLOW_LOG_FILE_PATH: /tmp/logs/glassflow
                GLASSFLOW_NATS_SERVER: nats:4222
              ports:
                - "8080:8080"
              depends_on:
                - nats

            nats:
              image: nats:alpine
              ports:
                - "4222:4222"
                - "8222:8222"
              command: ["-js"]
        '';

        # Development shell script
        setupScript = pkgs.writeShellScriptBin "flux-setup" ''
          echo "Setting up Flux development environment..."

          # Create necessary directories
          mkdir -p data/sample data/results config/development

          # Copy docker-compose file
          cp ${dockerComposeFile} docker-compose.yml

          echo "Setup complete! Run 'flux-start' to start services."
        '';

        startScript = pkgs.writeShellScriptBin "flux-start" ''
          echo "Starting Flux services..."
          docker-compose up -d
          echo "Waiting for services to be ready..."
          sleep 10
          echo "Services started!"
          echo "   Kafka:      localhost:9092"
          echo "   ClickHouse: localhost:8123"
          echo "   GlassFlow:  localhost:8080"
        '';

        statusScript = pkgs.writeShellScriptBin "flux-status" ''
          #!/usr/bin/env bash

          # Colors
          RED='\033[0;31m'
          GREEN='\033[0;32m'
          YELLOW='\033[1;33m'
          BLUE='\033[0;34m'
          PURPLE='\033[0;35m'
          CYAN='\033[0;36m'
          BOLD='\033[1m'
          NC='\033[0m' # No Color

          # Unicode symbols
          CHECK="✓"
          CROSS="✗"
          ARROW="→"
          DOT="•"

          echo -e "''${BOLD}''${CYAN}════════════════════════════════════════════════════════════''${NC}"
          echo -e "''${BOLD}''${CYAN}                    FLUX SYSTEM STATUS                       ''${NC}"
          echo -e "''${BOLD}''${CYAN}════════════════════════════════════════════════════════════''${NC}"
          echo

          # Function to check service
          check_service() {
            local name=$1
            local check_cmd=$2
            local get_info=$3
            
            printf "''${BOLD}%-15s''${NC}" "$name"
            
            if eval "$check_cmd" &>/dev/null; then
              local info=$(eval "$get_info" 2>/dev/null || echo "")
              echo -e "''${GREEN}''${CHECK} ONLINE''${NC}  ''${BLUE}$info''${NC}"
              return 0
            else
              echo -e "''${RED}''${CROSS} OFFLINE''${NC}"
              return 1
            fi
          }

          echo -e "''${BOLD}''${PURPLE}Services:''${NC}"
          echo -e "''${CYAN}─────────────────────────────────────────────────────────────''${NC}"

          # Check Kafka - get actual port from docker
          KAFKA_PORT=$(docker port clickhouse-etl-kafka-1 2>/dev/null | grep "9092/tcp" | cut -d':' -f2 || echo "9092")
          check_service "Kafka" \
            "kcat -b localhost:$KAFKA_PORT -L -t 1 2>/dev/null | grep -q 'Metadata for all topics'" \
            "echo -n 'localhost:$KAFKA_PORT'"

          # Check Zookeeper
          ZK_PORT=$(docker port clickhouse-etl-zookeeper-1 2>/dev/null | grep "2181/tcp" | cut -d':' -f2 || echo "2181")
          check_service "Zookeeper" \
            "echo ruok | nc -w 2 localhost $ZK_PORT 2>/dev/null | grep -q imok" \
            "echo -n 'localhost:$ZK_PORT'"

          # Check ClickHouse - get actual port
          CH_PORT=$(docker port clickhouse-etl-clickhouse-1 2>/dev/null | grep "8123/tcp" | cut -d':' -f2 || echo "8123")
          check_service "ClickHouse" \
            "curl -s http://localhost:$CH_PORT/ping | grep -q Ok" \
            "echo -n 'localhost:$CH_PORT (HTTP)'"

          # Check GlassFlow
          GF_PORT=$(docker port clickhouse-etl-glassflow-1 2>/dev/null | grep "8080/tcp" | cut -d':' -f2 || echo "8080")
          check_service "GlassFlow" \
            "curl -s http://localhost:$GF_PORT/ -o /dev/null -w '%{http_code}' | grep -qE '200|404'" \
            "echo -n 'localhost:$GF_PORT'"

          # Check NATS
          NATS_PORT=$(docker port clickhouse-etl-nats-1 2>/dev/null | grep "4222/tcp" | cut -d':' -f2 || echo "4222")
          check_service "NATS" \
            "nc -zv localhost $NATS_PORT 2>&1 | grep -q succeeded" \
            "echo -n 'localhost:$NATS_PORT'"

          echo
          echo -e "''${BOLD}''${PURPLE}Kafka Topics:''${NC}"
          echo -e "''${CYAN}─────────────────────────────────────────────────────────────''${NC}"

          # List Kafka topics if Kafka is running
          if kcat -b localhost:$KAFKA_PORT -L 2>/dev/null | grep -q "topic"; then
            kcat -b localhost:$KAFKA_PORT -L 2>/dev/null | grep "topic" | awk '{print $2}' | sed 's/"//g' | while read topic; do
              # Get message count for each topic
              count=$(kcat -b localhost:$KAFKA_PORT -t $topic -C -e -q -o beginning 2>/dev/null | wc -l)
              printf "  ''${DOT} %-30s ''${YELLOW}%s messages''${NC}\n" "$topic" "$count"
            done
          else
            echo -e "  ''${RED}No topics available (Kafka offline or no topics created)''${NC}"
          fi

          echo
          echo -e "''${BOLD}''${PURPLE}Docker Containers:''${NC}"
          echo -e "''${CYAN}─────────────────────────────────────────────────────────────''${NC}"

          # Show container status with health
          docker ps --format "table {{.Names}}\t{{.Status}}" 2>/dev/null | tail -n +2 | while read line; do
            name=$(echo $line | awk '{print $1}')
            status=$(echo $line | cut -d' ' -f2-)
            
            if echo "$status" | grep -q "Up"; then
              echo -e "  ''${DOT} ''${GREEN}$name''${NC} - $status"
            else
              echo -e "  ''${DOT} ''${RED}$name''${NC} - $status"
            fi
          done

          echo
          echo -e "''${BOLD}''${CYAN}════════════════════════════════════════════════════════════''${NC}"
        '';

        stopScript = pkgs.writeShellScriptBin "flux-stop" ''
          echo "Stopping Flux services..."
          docker-compose down
          echo "Services stopped!"
        '';

      in
      {
        # Development shells
        devShells = {
          # Default shell with everything
          default = pkgs.mkShell {
            buildInputs = systemPackages ++ [
              pythonBase
              rustToolchain
              setupScript
              startScript
              stopScript
              statusScript
            ];

            shellHook = ''
              echo "Flux Development Environment"
              echo "================================"
              echo "Available commands:"
              echo "  flux-setup  - Initial setup"
              echo "  flux-start  - Start services"
              echo "  flux-stop   - Stop services"
              echo ""
              echo "Environments:"
              echo "  Python ${pythonBase.version} with uv (POC development)"
              echo "  Rust ${rustToolchain.version} (Core development)"
              echo ""
              echo "To work on POC:     cd poc/"
              echo "To work on Core:    cd core/"
              echo "================================"

              # Set environment variables
              export FLUX_ROOT=$(pwd)
              export PYTHONPATH="$FLUX_ROOT/poc:$PYTHONPATH"
              export RUST_BACKTRACE=1
              export RUST_LOG="warn"

              # Create Python virtual environment for POC if needed
              if [ ! -d "$FLUX_ROOT/poc/.venv" ]; then
                echo "Creating Python virtual environment with uv..."
                mkdir -p "$FLUX_ROOT/poc"
                (cd "$FLUX_ROOT/poc" && uv venv)
              fi
            '';
          };

          # Python-only shell for POC work
          poc = pkgs.mkShell {
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
              ];

            shellHook = ''
              echo "Flux POC Development (Python)"
              echo "================================"

              # Set LD_LIBRARY_PATH for numpy and other compiled packages
              export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.zlib}/lib:$LD_LIBRARY_PATH"

              if [ ! -d ".venv" ]; then
                echo "Creating Python virtual environment with uv..."
                uv venv
              fi

              source .venv/bin/activate 2>/dev/null || true
              export PYTHONPATH=$(pwd):$PYTHONPATH
            '';
          };

          # Rust-only shell for core work
          core = pkgs.mkShell {
            buildInputs =
              [ rustToolchain ]
              ++ systemPackages
              ++ [
                startScript
                stopScript
              ];

            shellHook = ''
              echo "Flux Core Development (Rust)"
              echo "================================"
              cd core
              export RUST_BACKTRACE=1
              export RUST_LOG="warn"
              cargo --version
              rustc --version
            '';
          };
        };

        # Build packages
        packages = {
          # Python POC package
          poc = pkgs.stdenv.mkDerivation {
            pname = "flux-poc";
            version = "0.1.0";
            src = ./poc;

            buildInputs = [ pythonBase ];

            installPhase = ''
              mkdir -p $out/bin $out/lib
              cp -r flux $out/lib/

              cat > $out/bin/flux-poc << EOF
              #!${pkgs.bash}/bin/bash
              export PYTHONPATH=$out/lib:\$PYTHONPATH
              ${pythonBase}/bin/python $out/lib/flux/main.py "\$@"
              EOF

              chmod +x $out/bin/flux-poc
            '';
          };

          # Rust core package
          core = pkgs.rustPlatform.buildRustPackage {
            pname = "flux-core";
            version = "0.1.0";
            src = ./core;

            cargoLock = {
              lockFile = ./core/Cargo.lock;
            };

            buildInputs = with pkgs; [
              openssl
              pkg-config
            ];
          };

          # Docker images
          docker-poc = pkgs.dockerTools.buildImage {
            name = "flux-poc";
            tag = "latest";
            contents = [ self.packages.${system}.poc ];
            config = {
              Cmd = [ "/bin/flux-poc" ];
            };
          };

          docker-core = pkgs.dockerTools.buildImage {
            name = "flux-core";
            tag = "latest";
            contents = [ self.packages.${system}.core ];
            config = {
              Cmd = [ "/bin/flux-core" ];
            };
          };
        };

        # Apps (executables)
        apps = {
          poc = flake-utils.lib.mkApp {
            drv = self.packages.${system}.poc;
          };

          core = flake-utils.lib.mkApp {
            drv = self.packages.${system}.core;
          };
        };
      }
    );
}
