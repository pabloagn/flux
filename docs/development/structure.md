# FLUX Project Structure

## Root Files

- [`flake.nix`](../../flake.nix) - Nix flake for reproducible development environments
- [`flake.lock`](../../flake.lock) - Locked dependencies for Nix flake
- [`justfile`](../../justfile) - Task runner commands (build, test, deploy)
- [`Makefile`](../../Makefile) - Alternative build automation
- [`docker-compose.yml`](../../docker-compose.yml) - Local development stack (Kafka, QuestDB, ClickHouse, ScyllaDB)
- [`Cargo.workspace.toml`](../../Cargo.workspace.toml) - Rust workspace configuration for monorepo
- [`pyproject.toml`](../../pyproject.toml) - Python project configuration and dependencies
- [`LICENSE`](../../LICENSE) - Apache 2.0 open source license
- [`LICENSE-COMMERCIAL.md`](../../LICENSE-COMMERCIAL.md) - Commercial licensing terms
- [`CLA.md`](../../CLA.md) - Contributor License Agreement
- [`README.md`](../../README.md) - Project overview and quickstart
- [`CHANGELOG.md`](../../CHANGELOG.md) - Version history and changes
- [`ROADMAP.md`](../../ROADMAP.md) - Development timeline and milestones
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) - Contribution guidelines
- [`SECURITY.md`](../../SECURITY.md) - Security policy and reporting
- [`.gitignore`](../../.gitignore) - Git ignore patterns
- [`.dockerignore`](../../.dockerignore) - Docker build ignore patterns
- [`.rustfmt.toml`](../../.rustfmt.toml) - Rust code formatting rules
- [`.pre-commit-config.yaml`](../../.pre-commit-config.yaml) - Pre-commit hooks configuration
- [`.env.example`](../../.env.example) - Environment variables template

## [`/apps`](../../apps) - User Interfaces

### [`/apps/operator-tui`](../../apps/operator-tui)
Rust-based terminal UI for plant operators
- [`src/app.rs`](../../apps/operator-tui/src/app.rs) - Application state management
- [`src/calculations.rs`](../../apps/operator-tui/src/calculations.rs) - Electrochemical calculations
- [`src/config.rs`](../../apps/operator-tui/src/config.rs) - Configuration loader
- [`src/control/`](../../apps/operator-tui/src/control) - Plant control commands (write-back)
- [`src/crypto/`](../../apps/operator-tui/src/crypto) - Command signing/verification (Ed25519)
- [`src/data/`](../../apps/operator-tui/src/data) - Database connectors (Kafka, QuestDB, ClickHouse)
- [`src/safety/`](../../apps/operator-tui/src/safety) - Emergency shutdown, interlocks
- [`src/state/`](../../apps/operator-tui/src/state) - Distributed state management
- [`src/ui/`](../../apps/operator-tui/src/ui) - Terminal UI components (Ratatui)

### [`/apps/web-dashboard`](../../apps/web-dashboard)
React-based web dashboard for analytics
- [`package.json`](../../apps/web-dashboard/package.json) - Node.js dependencies
- [`public/`](../../apps/web-dashboard/public) - Static assets
- [`src/`](../../apps/web-dashboard/src) - React components and logic

### [`/apps/mobile-app`](../../apps/mobile-app)
Mobile operator interface (planned)

## [`/services`](../../services) - Microservices

### [`/services/simulator`](../../services/simulator)
Python electrochemical plant simulator
- [`flux/simulation/`](../../services/simulator/flux/simulation) - Cell, electrolyzer, plant models
- [`flux/pipeline/`](../../services/simulator/flux/pipeline) - Kafka producer, GlassFlow, ClickHouse clients
- [`flux/analytics/`](../../services/simulator/flux/analytics) - KPI calculations, anomaly detection
- [`scripts/`](../../services/simulator/scripts) - Setup and validation utilities

### [`/services/control-system`](../../services/control-system)
Rust service for plant control loops
- [`src/opc_ua.rs`](../../services/control-system/src/opc_ua.rs) - OPC UA server implementation
- [`src/safety.rs`](../../services/control-system/src/safety.rs) - Safety logic integration

### [`/services/safety-interlock`](../../services/safety-interlock)
SIL-3 certified safety system
- [`src/sis.rs`](../../services/safety-interlock/src/sis.rs) - Safety Instrumented System
- [`src/watchdog.rs`](../../services/safety-interlock/src/watchdog.rs) - Hardware watchdog timers

### [`/services/state-manager`](../../services/state-manager)
Distributed state synchronization
- [`src/scylla.rs`](../../services/state-manager/src/scylla.rs) - ScyllaDB state persistence
- [`src/redis.rs`](../../services/state-manager/src/redis.rs) - Redis hot cache

### [`/services/audit-logger`](../../services/audit-logger)
Immutable audit trail with cryptographic signing
- [`src/pulsar.rs`](../../services/audit-logger/src/pulsar.rs) - Apache Pulsar client
- [`src/crypto.rs`](../../services/audit-logger/src/crypto.rs) - Event signing

### [`/services/ml-platform`](../../services/ml-platform)
Machine learning model serving
- [`src/models.py`](../../services/ml-platform/src/models.py) - PyTorch/TensorFlow models
- [`src/features.py`](../../services/ml-platform/src/features.py) - Feature engineering
- [`src/server.py`](../../services/ml-platform/src/server.py) - Model serving API

### [`/services/kpi-engine`](../../services/kpi-engine)
Real-time KPI calculation service

### [`/services/alarm-manager`](../../services/alarm-manager)
Alarm processing and escalation

### [`/services/historian`](../../services/historian)
Time-series data archival service

### [`/services/data-pipeline`](../../services/data-pipeline)
Stream processing orchestration

## [`/lib`](../../lib) - Shared Libraries

### [`/lib/flux-control`](../../lib/flux-control)
Control algorithms (PID, MPC)
- [`src/pid.rs`](../../lib/flux-control/src/pid.rs) - PID controller implementation
- [`src/mpc.rs`](../../lib/flux-control/src/mpc.rs) - Model Predictive Control

### [`/lib/flux-safety`](../../lib/flux-safety)
Safety system primitives
- [`src/sil.rs`](../../lib/flux-safety/src/sil.rs) - SIL rating functions
- [`src/hazop.rs`](../../lib/flux-safety/src/hazop.rs) - HAZOP analysis tools

### [`/lib/flux-state`](../../lib/flux-state)
Distributed state management
- [`src/consensus.rs`](../../lib/flux-state/src/consensus.rs) - Raft consensus
- [`src/distributed.rs`](../../lib/flux-state/src/distributed.rs) - State distribution

### [`/lib/flux-protocol`](../../lib/flux-protocol)
Industrial protocol implementations

### [`/lib/flux-common`](../../lib/flux-common)
Shared types and utilities

### [`/lib/flux-py-common`](../../lib/flux-py-common)
Python shared libraries
- [`src/models/`](../../lib/flux-py-common/src/models) - Data models
- [`src/utils/`](../../lib/flux-py-common/src/utils) - Utility functions

## [`/config`](../../config) - Configuration Files

### [`/config/plant`](../../config/plant)
Plant topology and parameters
- [`plant.json`](../../config/plant/plant.json) - Plant structure (units, stacks, cells)
- [`defaults.json`](../../config/plant/defaults.json) - Default operational values
- [`financials.json`](../../config/plant/financials.json) - Economic parameters
- [`materials.json`](../../config/plant/materials.json) - Material properties
- [`ops_limits.json`](../../config/plant/ops_limits.json) - Operational limits

### [`/config/questdb`](../../config/questdb)
QuestDB configuration
- [`questdb.conf`](../../config/questdb/questdb.conf) - Server configuration
- [`tables.sql`](../../config/questdb/tables.sql) - Table definitions

### [`/config/scylladb`](../../config/scylladb)
ScyllaDB configuration
- [`scylla.yaml`](../../config/scylladb/scylla.yaml) - Cluster configuration
- [`schema.cql`](../../config/scylladb/schema.cql) - CQL schema

### [`/config/schemas`](../../config/schemas)
Database schemas
- [`clickhouse/`](../../config/schemas/clickhouse) - ClickHouse DDL
- [`questdb/init.sql`](../../config/schemas/questdb/init.sql) - QuestDB initialization
- [`kafka/`](../../config/schemas/kafka) - Kafka topic schemas
- [`pulsar/audit-schema.json`](../../config/schemas/pulsar/audit-schema.json) - Audit event schema

### [`/config/security`](../../config/security)
Security configuration
- [`rbac.yaml`](../../config/security/rbac.yaml) - Role-based access control
- [`crypto.yaml`](../../config/security/crypto.yaml) - Cryptographic settings
- [`keys.yaml`](../../config/security/keys.yaml) - Key management

### [`/config/control`](../../config/control)
Control system tuning
- [`pid_tuning.yaml`](../../config/control/pid_tuning.yaml) - PID parameters

### [`/config/safety`](../../config/safety)
Safety system configuration
- [`interlocks.yaml`](../../config/safety/interlocks.yaml) - Interlock definitions

### [`/config/ml`](../../config/ml)
Machine learning configuration
- [`models.yaml`](../../config/ml/models.yaml) - Model definitions
- [`features.yaml`](../../config/ml/features.yaml) - Feature engineering
- [`model_registry.yaml`](../../config/ml/model_registry.yaml) - Model versions

### [`/config/opc-ua`](../../config/opc-ua)
OPC UA server configuration
- [`server.xml`](../../config/opc-ua/server.xml) - Server settings
- [`nodes.xml`](../../config/opc-ua/nodes.xml) - Node definitions

### [`/config/ptp`](../../config/ptp)
Precision Time Protocol
- [`ptp4l.conf`](../../config/ptp/ptp4l.conf) - PTP daemon configuration

## [`/infra`](../../infra) - Infrastructure as Code

### [`/infra/docker`](../../infra/docker)
Docker Compose files
- [`docker-compose.control.yml`](../../infra/docker/docker-compose.control.yml) - Control systems
- [`docker-compose.ml.yml`](../../infra/docker/docker-compose.ml.yml) - ML platform
- [`docker-compose.questdb.yml`](../../infra/docker/docker-compose.questdb.yml) - QuestDB stack

### [`/infra/kubernetes`](../../infra/kubernetes)
Kubernetes manifests
- [`base/`](../../infra/kubernetes/base) - Base configurations
- [`overlays/prod/`](../../infra/kubernetes/overlays/prod) - Production overlays

### [`/infra/terraform`](../../infra/terraform)
Infrastructure provisioning
- [`modules/networking/tsn.tf`](../../infra/terraform/modules/networking/tsn.tf) - TSN network setup
- [`modules/compute/`](../../infra/terraform/modules/compute) - Compute resources
- [`modules/storage/`](../../infra/terraform/modules/storage) - Storage resources

### [`/infra/ansible`](../../infra/ansible)
Configuration management
- [`playbooks/deploy.yml`](../../infra/ansible/playbooks/deploy.yml) - Deployment playbook
- [`inventory/production.ini`](../../infra/ansible/inventory/production.ini) - Production inventory

## [`/scripts`](../../scripts) - Automation Scripts

### [`/scripts/deployment`](../../scripts/deployment)
- [`deploy_prod.sh`](../../scripts/deployment/deploy_prod.sh) - Production deployment
- [`rollback.sh`](../../scripts/deployment/rollback.sh) - Rollback procedure
- [`canary.sh`](../../scripts/deployment/canary.sh) - Canary deployment

### [`/scripts/safety`](../../scripts/safety)
- [`emergency_shutdown.sh`](../../scripts/safety/emergency_shutdown.sh) - Emergency stop
- [`verify_interlocks.sh`](../../scripts/safety/verify_interlocks.sh) - Interlock verification

### [`/scripts/ptp`](../../scripts/ptp)
- [`sync_time.sh`](../../scripts/ptp/sync_time.sh) - Time synchronization
- [`check_drift.sh`](../../scripts/ptp/check_drift.sh) - Clock drift monitoring

### [`/scripts/backup`](../../scripts/backup)
- [`state_backup.sh`](../../scripts/backup/state_backup.sh) - State backup
- [`restore.sh`](../../scripts/backup/restore.sh) - State restoration

### [`/scripts/monitoring`](../../scripts/monitoring)
- [`healthcheck.sh`](../../scripts/monitoring/healthcheck.sh) - Health checks
- [`latency_monitor.sh`](../../scripts/monitoring/latency_monitor.sh) - Latency monitoring

## [`/tests`](../../tests) - Test Suites

### [`/tests/integration`](../../tests/integration)
- [`test_control_loop.py`](../../tests/integration/test_control_loop.py) - Control loop tests
- [`test_safety.py`](../../tests/integration/test_safety.py) - Safety system tests

### [`/tests/performance`](../../tests/performance)
- [`load_test.js`](../../tests/performance/load_test.js) - Load testing (1M events/sec)
- [`latency_test.py`](../../tests/performance/latency_test.py) - Latency verification

### [`/tests/safety`](../../tests/safety)
- [`test_emergency_stop.py`](../../tests/safety/test_emergency_stop.py) - E-stop tests
- [`test_interlocks.py`](../../tests/safety/test_interlocks.py) - Interlock tests

### [`/tests/chaos`](../../tests/chaos)
- [`network_partition.sh`](../../tests/chaos/network_partition.sh) - Network failure simulation
- [`node_failure.sh`](../../tests/chaos/node_failure.sh) - Node failure simulation

## [`/docs`](../../docs) - Documentation

### [`/docs/architecture`](../architecture)
- [`system-design.md`](../architecture/system-design.md) - System architecture
- [`data-flow.md`](../architecture/data-flow.md) - Data pipeline design
- [`deployment.md`](../architecture/deployment.md) - Deployment architecture

### [`/docs/safety`](../safety)
- [`sil_certification.md`](../safety/sil_certification.md) - SIL-3 certification
- [`hazop_analysis.md`](../safety/hazop_analysis.md) - HAZOP analysis
- [`emergency_procedures.md`](../safety/emergency_procedures.md) - Emergency procedures

### [`/docs/compliance`](../compliance)
- [`iec_61511.md`](../compliance/iec_61511.md) - IEC 61511 compliance
- [`isa_84.md`](../compliance/isa_84.md) - ISA-84 compliance
- [`audit_trail.md`](../compliance/audit_trail.md) - Audit requirements

### [`/docs/sop`](../sop)
- [`startup.md`](../sop/startup.md) - Startup procedures
- [`shutdown.md`](../sop/shutdown.md) - Shutdown procedures
- [`emergency.md`](../sop/emergency.md) - Emergency procedures

## [`/nix`](../../nix) - Nix Development Shells
- [`shell-default.nix`](../../nix/shell-default.nix) - Default environment
- [`shell-app-tui.nix`](../../nix/shell-app-tui.nix) - TUI development
- [`shell-srv-sim.nix`](../../nix/shell-srv-sim.nix) - Simulator environment
- [`shell-srv-kpi.nix`](../../nix/shell-srv-kpi.nix) - KPI engine environment

## [`/certs`](../../certs) - PKI Certificates
- [`ca/`](../../certs/ca) - Certificate Authority
- [`server/`](../../certs/server) - Server certificates
- [`client/`](../../certs/client) - Client certificates

## [`/data`](../../data) - Data Storage
- [`questdb/`](../../data/questdb) - Real-time data (24hr retention)
- [`scylladb/`](../../data/scylladb) - Plant state storage
- [`timescale/`](../../data/timescale) - Audit trail
- [`sample/`](../../data/sample) - Sample datasets

## [`/assets`](../../assets) - Brand Assets
- [`FLUX.svg`](../../assets/FLUX.svg) - Logo vector
- [`FLUX_Dark.png`](../../assets/FLUX_Dark.png) - Dark theme logo
- [`FLUX_Light.png`](../../assets/FLUX_Light.png) - Light theme logo

## [`/.github`](../../.github) - GitHub Configuration
- [`workflows/`](../../.github/workflows) - CI/CD pipelines
- [`CODEOWNERS`](../../.github/CODEOWNERS) - Code ownership rules
- [`dependabot.yml`](../../.github/dependabot.yml) - Dependency updates
