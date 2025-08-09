# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║                            FLUX PLATFORM JUSTFILE                            ║
# ╠══════════════════════════════════════════════════════════════════════════════╣
# ║  A high-performance digital twin for electrochemical processes               ║
# ║                                                                              ║
# ║  Usage:                                                                      ║
# ║    just              → list all recipes                                      ║
# ║    just <recipe>     → execute recipe                                        ║
# ║    just --evaluate   → show all variables                                    ║
# ╚══════════════════════════════════════════════════════════════════════════════╝

set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-load := true
set export := true

# ═══════════════════════════════════════════════════════════════════════════════
# Configuration
# ═══════════════════════════════════════════════════════════════════════════════

# --- Main ---
VERSION := `git describe --tags --always --dirty 2>/dev/null || echo "dev"`
TIMESTAMP := `date -u +"%Y-%m-%dT%H:%M:%SZ"`
ROOT := justfile_directory()

# --- Root Directories ---
APPS_DIR := ROOT / "apps"
CONFIG_DIR := ROOT / "config"
DATA_DIR := ROOT / "data"
INFRA_DIR := ROOT / "infra"
LIB_DIR := ROOT / "lib"
SCRIPTS_DIR := ROOT / "scripts"
SERVICES_DIR := ROOT / "services"
TESTS_DIR := ROOT / "tests"

# --- Subdirectories ---
# Apps
MOBILE_APP_DIR := APPS_DIR / "mobile-app"
OPERATOR_TUI_DIR := APPS_DIR / "operator-tui"
WEB_DASHBOARD_DIR := APPS_DIR / "web-dashboard"

# Services
ALARM_MANAGER_DIR := SERVICES_DIR / "alarm-manager"
DATA_PIPELINE_DIR := SERVICES_DIR / "data-pipeline"
HISTORIAN_DIR := SERVICES_DIR / "historian"
KPI_ENGINE_DIR := SERVICES_DIR / "kpi-engine"
SIMULATOR_DIR := SERVICES_DIR / "simulator"

# --- Docker ---
COMPOSE_FILES := "-f infra/docker/compose.yml"
COMPOSE_CMD   := "docker compose {{COMPOSE_FILES}}"

# --- Build Configuration ---
RUST_BACKTRACE := "1"
RUST_LOG := "info"
UV_NO_PROGRESS := "true"
DOCKER_BUILDKIT := "1"

# ═══════════════════════════════════════════════════════════════════════════════
# Recipes
# ═══════════════════════════════════════════════════════════════════════════════

# Help
default:
  @just --list --unsorted

# ─= NIX =───────────────────────────────────────────────────────────────────────

# Enter root dev shell
shell-default:
	@nix develop .

# TODO: Add the rest of the devshells here

# ─= BUILD =─────────────────────────────────────────────────────────────────────
# Build all custom docker images defined in flake.nix and load them into docker
build:
	@echo "Building and loading all FLUX container images via Nix..."
	@nix build .#all-images --out-link result-images
	@for image in result-images/*; do \
	  echo "--> Loading $$image"; \
	  docker load < $$image; \
	done
	@rm result-images


# ─= STACK LIFECYCLE =───────────────────────────────────────────────────────────
# Start the full development stack (builds images first)
up: build
	@echo "Starting FLUX stack..."
	@{{COMPOSE_CMD}} up -d

# Stop the stack
down:
	@echo "Stopping FLUX stack..."
	@{{COMPOSE_CMD}} down --remove-orphans

# Show status of all containers
status:
	@{{COMPOSE_CMD}} ps

# Restart the entire stack
restart: down up

# Tail logs for all services, or a specific one
# Usage: just logs [service-name]
logs service='':
	@{{COMPOSE_CMD}} logs -f {{service}}

# ─= KAFKA =───────────────────────────────────────────────────────────────────

# Idempotent creation of required Kafka topics
topics-create:
	@"{{SCRIPTS_DIR}}/create_topics.sh"

# Display Kafka topics list
topics-list:
  @kcat -L -b localhost:9092 | grep -E '^  topic' || true

# Display tail for any Kafka topic
topic-tail topic="flux_electrical_realtime" count="10":
  @kcat -C -b localhost:9092 -t {{topic}} -o -{{count}} -e -J

topics-listen topic="flux_electrical_realtime":
  @kafka-console-consumer --bootstrap-server localhost:9092 --topic {{topic}} --from-beginning --max-messages 5 | jq '.'

# ─= CLICKHOUSE =────────────────────────────────────────────────────────────────

# Run the init SQL file into ClickHouse
clickhouse-init: 
	@"{{SCRIPTS_DIR}}/init_clickhouse.sh"

# Quick row count
clickhouse-count table='flux.cell_metrics':
	@docker compose exec clickhouse \
	  clickhouse-client --query "SELECT count() FROM {{table}}"

# GlassFlow -------------------------------------------------------

# (Re)create ETL pipeline
glassflow-setup:
	@"{{SCRIPTS_DIR}}/setup_glassflow.py"

# Delete glassflow pipeline
glassflow-delete:
	@"{{SCRIPTS_DIR}}/delete_glassflow_pipeline.py"

# Simulator -------------------------------------------------------

# Run Python plant simulation for N seconds
sim time="60":
	cd "{{SIMULATOR_DIR}}" && uv sync >/dev/null && \
	  python "{{SCRIPTS_DIR}}/run_simulation.py" -t {{time}}

# --- Build & Test ---
# Build Operator TUI
operator-tui-build:
	@cd "{{OPERATOR_TUI_DIR}}" && cargo build --quiet --release

# Run Operator TUI
operator-tui-run:
	@cd "{{OPERATOR_TUI_DIR}}" && ./target/release/flux-gui
