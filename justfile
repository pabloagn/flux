# ────────────────────────────────────────────────────────────────
#  Flux justfile
# ────────────────────────────────────────────────────────────────
#  Usage:
#     just <recipe>          # run recipe
#     just -l                # list all
#     just -d <recipe>       # show command without running
# ----------------------------------------------------------------

set shell := ["bash", "-euo", "pipefail", "-c"]

# Environment Variables -----------------------------------------------------------
export UV_NO_PROGRESS := "true"
export RUST_BACKTRACE := "1"

# Paths -----------------------------------------------------------
ROOT := justfile_directory()
POC_DIR     := ROOT + "/poc"
CORE_DIR    := ROOT + "/core"
GUI_DIR     := ROOT + "/gui"
SCRIPTS_DIR := POC_DIR + "/scripts"

# Help
default:
  @just --list --unsorted

# Nix shells ------------------------------------------------------
# Enter root dev shell
dev-shell:
	@nix develop .

# Enter poc dev shell
poc-shell:
	@cd "{{POC_DIR}}" && nix develop ..#poc

# Enter core dev shell
core-shell:
	@cd "{{CORE_DIR}}" && nix develop ..#core

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
	cd "{{POC_DIR}}" && uv sync >/dev/null && \
	  python "{{SCRIPTS_DIR}}/run_simulation.py" -t {{time}}

# Build & test ----------------------------------------------------

# Build core
core-build:
	@cd "{{CORE_DIR}}" && cargo build --release

# Test core
core-test:
	@cd "{{CORE_DIR}}" && cargo test

# Build GUI
gui-build:
	@cd "{{GUI_DIR}}" && cargo build --quiet --release

# Run GUI
gui-run:
	@cd "{{GUI_DIR}}" && ./target/release/flux-gui

# Pytest suite
poc-test:
	@cd "{{POC_DIR}}" && pytest -q

# Lint / format ---------------------------------------------------
# Lint code using ruff
lint:
	@ruff check "{{POC_DIR}}/flux" "{{SCRIPTS_DIR}}" --fix

# Format code using ruff
format:
	@ruff format "{{POC_DIR}}/flux" "{{SCRIPTS_DIR}}"
	@cd "{{CORE_DIR}}" && cargo fmt
	@cd "{{GUI_DIR}}"  && cargo fmt

# Clean-ups -------------------------------------------------------
# Clean docker images
docker-clean:
	@docker system prune -f

# Clean rust targets
target-clean:
	@rm -rf "{{CORE_DIR}}/target" "{{GUI_DIR}}/target"

