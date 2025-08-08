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

# --- Build Configuration ---
RUST_BACKTRACE := "1"
RUST_LOG := "info"
UV_NO_PROGRESS := "true"
DOCKER_BUILDKIT := "1"

# --- Recipes ---
# Help
default:
  @just --list --unsorted

# Nix shells ------------------------------------------------------
# Enter root dev shell
shell-default:
	@nix develop .

# Enter simulator dev shell
shell-simulator:
  @cd "{{SIMULATOR_DIR}}" && nix develop ..#simulator

# Enter simulator dev shell
shell-kpi-engine:
  @cd "{{KPI_ENGINE_DIR}}" && nix develop ..#kpi-engine

# Enter simulator dev shell
shell-operator-tui:
  @cd "{{OPERATOR_TUI_DIR}}" && nix develop ..#operator-tui

# Stack lifecycle -------------------------------------------------
# Initialise workspace & copy compose file
setup:
  @rm -f poc/docker-compose.yml
  @flux-setup

# Boot Kafka, ClickHouse, NATS, GlassFlow
start:
  @flux-start

# Stop containers
stop:           
	@flux-stop

# Pretty status table
status:         
	@flux-status

stack-restart: stop start

# Kafka -----------------------------------------------------------

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

# ClickHouse ------------------------------------------------------

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
