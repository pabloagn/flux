# Flux: Chemical Process Digital Twin Requirements Specification

## Project Overview

### Vision

Develop a state-of-the-art digital twin for a chlor-alkali electrolysis plant that addresses real industrial challenges in electrochemical processing.

### Scope

A multi-electrolyzer chlor-alkali plant producing chlorine gas (Cl₂), hydrogen gas (H₂), and sodium hydroxide (NaOH) through membrane cell electrolysis technology.

### Industry Context

The global chlor-alkali market represents an $80+ billion industry, with membrane cell technology dominating 65% of worldwide production capacity. Energy costs constitute 40-50% of total production costs, while membrane replacement represents 30-40% of operational expenses.

## Industry Problems Being Addressed

Based on industrial research and operational data:

### 1. Membrane Degradation Management

**Problem**: Membrane lifetime varies from 2-7 years depending on operational conditions, with replacement costs of $500-1000/m² [^1].

- Current efficiency drops from 96% to 85% over membrane lifetime
- Voltage increases by 0.2-0.5V over operational life
- Unplanned membrane failures cause 5-10 days of downtime

**Solution Approach**: Implement predictive models correlating operational parameters with membrane health indicators.

### 2. Brine Quality Impact on Performance

**Problem**: Impurities (Ca²⁺, Mg²⁺, SO₄²⁻) cause membrane fouling and reduced efficiency [^2].

- Calcium/Magnesium must be <20 ppb for optimal operation
- Residence time between brine treatment and electrolysis affects impurity precipitation
- Poor brine quality reduces current efficiency by 2-5%

**Solution Approach**: Use GlassFlow's temporal joins to correlate upstream brine quality with downstream cell performance, accounting for 2-4 hour residence times.

### 3. Current Distribution Optimization

**Problem**: Individual cell variations in multi-cell electrolyzers cause efficiency losses [^3].

- 5-10% variation in individual cell voltages within same electrolyzer
- Hotspots reduce membrane lifetime by up to 50%
- Manual current redistribution done quarterly in most plants

**Solution Approach**: Real-time monitoring and optimization of current distribution across cells.

### 4. Energy Cost Management

**Problem**: Electricity represents 40-50% of production costs with volatile pricing [^4].

- Load flexibility limited by product demand
- Startup/shutdown cycles damage membranes
- Efficiency varies with load (85-95% turndown capability)

**Solution Approach**: Dynamic load optimization based on electricity spot prices while maintaining product quality.

## Technical Architecture

### Process Configuration

#### Plant Layout

```txt
Plant
├── Brine Treatment System
│   ├── Primary Treatment (settling)
│   ├── Secondary Treatment (ion exchange)
│   └── Ultra-purification (<20 ppb hardness)
├── Electrolyzer Unit 1 (100 MW)
│   ├── Cell Stack A (20 cells × 2.5 MW)
│   ├── Cell Stack B (20 cells × 2.5 MW)
│   └── Cell Stack C (20 cells × 2.5 MW)
├── Electrolyzer Unit 2 (100 MW)
│   └── [identical structure]
├── Electrolyzer Unit 3 (100 MW)
│   └── [identical structure]
└── Product Handling
├── Cl₂ compression and drying
├── H₂ purification
└── NaOH concentration (32% → 50%)
```

#### Operating Conditions

- **Current Density**: 4-6 kA/m² (typical industrial range)
- **Cell Temperature**: 85-90°C
- **Operating Pressure**: 100-300 mbar differential
- **Brine Concentration**: 300 g/L NaCl inlet, 200 g/L outlet
- **NaOH Product**: 32% concentration ex-cell

### Data Architecture

#### Sensor Network Design

Per electrolyzer cell (180 cells total):

- Cell voltage (redundant pair) - 1 Hz sampling
- Cell current - 1 Hz sampling
- Anolyte temperature - 0.1 Hz sampling
- Catholyte temperature - 0.1 Hz sampling
- Differential pressure - 1 Hz sampling

Per electrolyzer unit (3 units):

- Brine flow rate - 1 Hz
- Brine conductivity - 0.1 Hz
- NaOH concentration - 0.1 Hz
- Cl₂ purity - 0.017 Hz (every 60s)
- H₂ purity - 0.017 Hz
- Power quality metrics - 10 Hz

Plant-wide:

- Brine quality (Ca, Mg, SO₄) - 0.0017 Hz (every 10 min)
- Ambient conditions - 0.017 Hz
- Electricity spot price - 0.00028 Hz (hourly)

**Total Data Points**: ~2,500 primary tags → ~10,000 events/second with derived calculations

#### Kafka Topic Structure

By measurement type (optimized for temporal joins):

- `flux.electrical.realtime` - voltage, current, power (high frequency)
- `flux.process.temperatures` - all temperature measurements
- `flux.process.pressures` - pressure differentials
- `flux.process.flows` - flow rates
- `flux.quality.brine` - upstream brine analysis
- `flux.quality.products` - product specifications
- `flux.external.market` - electricity prices, demand signals

#### GlassFlow Pipeline Features

1. **Deduplication**:
   - Redundant voltage sensors (safety-critical)
   - Multiple brine quality analyzers
   - Time window: 1 minute for electrical, 1 hour for quality

2. **Temporal Joins**:
   - Brine quality → Cell performance (2-4 hour lag)
   - Power changes → Temperature response (15-30 min lag)
   - Current distribution → Individual cell voltage (immediate)

#### ClickHouse Schema

Tables:

- `cell_metrics` - High-frequency cell-level data (partitioned daily)
- `electrolyzer_metrics` - Aggregated unit performance
- `quality_metrics` - Product and brine quality
- `calculated_kpis` - Real-time KPI calculations
- `anomaly_events` - Detected anomalies and alarms

Materialized Views:

- 5-minute aggregations for trending
- Hourly KPI calculations
- Daily efficiency reports

### Process Simulation

#### Electrochemical Model (Phase 0 - Simplified)

Based on industrial correlations [^5]:

Cell Voltage: `V_cell = E₀ + η_act + η_ohm + η_conc`

- E₀ = 2.19 V (theoretical decomposition voltage at 85°C)
- η_act = activation overpotential (Butler-Volmer simplified)
- η_ohm = ohmic losses (membrane + electrolyte)
- η_conc = concentration overpotential

Current Efficiency: `CE = 96 - 0.3×(J/1000) - 0.002×(T-85) - k×t_operation`

- J = current density (A/m²)
- T = temperature (°C)
- t_operation = hours of operation
- k = degradation factor

#### Control Strategy

Hierarchical control structure:

1. **Regulatory Control** (PID loops):
   - Cell temperature control via cooling water
   - Pressure differential control
   - Brine flow control
   - Level controls

2. **Supervisory Control**:
   - Current distribution optimization
   - Load balancing between electrolyzers
   - Product quality coordination

3. **Advanced Control** (future phases):
   - Model predictive control (MPC)
   - Real-time optimization (RTO)

### Key Performance Indicators

#### Operational KPIs

- **Current Efficiency (CE)**: Target >94%, alarm <90%
- **Specific Energy Consumption**: Target <2,450 kWh/MT Cl₂
- **Cell Voltage**: Target <3.1V, alarm >3.3V
- **Membrane Voltage Drop**: Track trend, alarm on 20% increase
- **Production Rate**: MT/day of Cl₂, NaOH, H₂

#### Economic KPIs

- **Energy Cost per MT**: $/MT including spot price variations
- **Membrane Life Remaining**: Predicted days to replacement
- **Overall Equipment Effectiveness (OEE)**: Target >85%

#### Safety & Environmental KPIs

- **Cl₂/H₂ Ratio**: Must remain 1:1 ±2%
- **Differential Pressure Events**: Count per month
- **Carbon Intensity**: kg CO₂/MT product (with renewable integration)

## Development Phases

### Phase 0: Base Infrastructure (Weeks 1-4)

- [ ] Process simulation engine (Python)
  - Electrochemical model implementation
  - Mass/energy balance
  - Basic PID control loops
- [ ] Sensor simulation layer
  - Realistic noise models (±0.5% for voltage, ±2% for flow)
  - Drift simulation (0.1%/month)
  - Failure modes
- [ ] Data pipeline setup
  - Kafka topics creation
  - GlassFlow configuration
  - ClickHouse schema
- [ ] Basic data generation
  - Normal operation scenarios
  - 1000-10000 events/second

### Phase 1: Monitoring Twin (Weeks 5-8)

- [ ] Real-time data ingestion
- [ ] KPI calculation engine
- [ ] Basic visualization dashboard
- [ ] Alarm management system
- [ ] Historical data storage (30 days)

### Phase 2: Predictive Twin (Weeks 9-12)

- [ ] Membrane degradation model
- [ ] Efficiency forecasting
- [ ] Anomaly detection algorithms
- [ ] Maintenance scheduling optimization

### Phase 3: Prescriptive Twin (Weeks 13-16)

- [ ] Energy optimization algorithms
- [ ] Current distribution optimization
- [ ] What-if scenario analysis
- [ ] Recommendation engine

### Phase 4: Autonomous Twin (Future)

- [ ] Closed-loop optimization
- [ ] Self-tuning control
- [ ] Automated fault recovery

## Technology Stack

### Core Components

- **Process Simulation**: Python (NumPy, SciPy, pandas)
- **Message Streaming**: Apache Kafka
- **ETL Pipeline**: GlassFlow
- **Time-Series Database**: ClickHouse
- **Container Runtime**: Docker
- **Package Management**: Nix

### Python Libraries

- `numpy`, `scipy`: Numerical computation
- `pandas`: Data manipulation
- `pydantic`: Data validation
- `asyncio`: Asynchronous operations
- `kafka-python`: Kafka client
- `clickhouse-driver`: ClickHouse client

### Development Tools

- Nix flakes for reproducible environments
- Docker Compose for service orchestration
- pytest for testing
- Black/ruff for code formatting

## Success Criteria

### Technical Metrics

- Pipeline handles 10,000+ events/second
- <100ms end-to-end latency (sensor to dashboard)
- 99.9% data delivery guarantee
- <1% CPU overhead for deduplication

### Business Metrics

- Demonstrate 2-3% efficiency improvement potential
- Show 15-20% reduction in membrane replacement costs
- Identify 5-10% energy cost savings opportunities

### Validation Methods

- Compare KPIs with published industrial benchmarks [^6]
- Validate model predictions against literature data
- Ensure control strategies align with industry best practices

## Anomaly Scenarios

Based on industrial incident reports [^7]:

### Priority 1 (Safety Critical)

1. **Differential Pressure Excursion**: Membrane rupture risk
2. **H₂ in Cl₂ Stream**: Explosion risk >4% H₂
3. **Power Supply Failure**: Emergency shutdown procedures

### Priority 2 (Production Impact)

1. **Brine Quality Upset**: Ca/Mg spike to >100 ppb
2. **Current Maldistribution**: >15% deviation between cells
3. **Cooling System Failure**: Temperature rise >95°C

### Priority 3 (Efficiency Impact)

1. **Membrane Degradation**: Voltage increase >0.1V/month
2. **Sensor Drift**: Calibration deviation >2%
3. **Grid Frequency Deviation**: Impact on rectifier efficiency

## Constraints and Assumptions

### Technical Constraints

- Single geographic location (no multi-site coordination)
- Greenfield plant (no legacy system integration)
- Steady-state design capacity of 1000 MT/day Cl₂

### Assumptions

- Stable brine supply (salt mines or solution mining)
- Grid connection with spot market access
- Modern membrane technology (DuPont N2030 or equivalent)
- Environmental permits in place

## References

[^1]: Karlsson, H., & Cornell, A. (2016). "Membrane degradation in chlor-alkali cells: A review of mechanisms and mitigation strategies." *Journal of Applied Electrochemistry*, 46(6), 599-614.

[^2]: Moussallem, I., et al. (2008). "Chlor-alkali electrolysis with oxygen depolarized cathodes: history, present status and future prospects." *Journal of Applied Electrochemistry*, 38(9), 1177-1194.

[^3]: Bergner, D., et al. (2015). "Current distribution in industrial chlor-alkali cells: Modeling and optimization." *Chemical Engineering Science*, 135, 196-204.

[^4]: Brinkmann, T., et al. (2014). "Best Available Techniques Reference Document for the Production of Chlor-alkali." European Commission Joint Research Centre, EUR 26844 EN.

[^5]: Schmittinger, P. (Ed.). (2000). *Chlorine: Principles and Industrial Practice*. Wiley-VCH.

[^6]: Euro Chlor. (2023). "Chlor-alkali Industry Review 2022-2023: Safety, Health, Environment and Energy Report."

[^7]: U.S. Chemical Safety Board. (2020). "Incident Database: Chlor-alkali facility incidents 2010-2020."

## Appendices

### A. Electrochemical Reactions

**Anode (Chlorine Evolution)**:
$$2Cl^- \rightarrow Cl_2 + 2e^- \quad (E^0 = 1.36V)$$

**Cathode (Hydrogen Evolution)**:
$$2H_2O + 2e^- \rightarrow H_2 + 2OH^- \quad (E^0 = -0.83V)$$

**Overall**:
$$2NaCl + 2H_2O \rightarrow Cl_2 + H_2 + 2NaOH \quad (E^0_{cell} = 2.19V)$$

### B. Typical Operating Ranges

| Parameter | Min | Normal | Max | Unit |
|-----------|-----|---------|-----|------|
| Current Density | 3.0 | 4.5 | 6.0 | kA/m² |
| Temperature | 80 | 85 | 90 | °C |
| NaCl (inlet) | 280 | 300 | 320 | g/L |
| NaCl (outlet) | 180 | 200 | 220 | g/L |
| NaOH Product | 30 | 32 | 33 | wt% |
| Pressure Diff | 50 | 150 | 300 | mbar |

### C. Data Volume Estimates

- Raw sensor data: ~850 GB/month
- Deduplicated data: ~750 GB/month
- Aggregated KPIs: ~50 GB/month
- Anomaly events: ~1 GB/month

### D. Noise and Drift Specifications

| Measurement Type | Noise (σ) | Drift Rate | Failure Rate |
|-----------------|-----------|------------|--------------|
| Voltage | ±0.5% | 0.1%/month | 0.01%/year |
| Current | ±0.3% | 0.05%/month | 0.01%/year |
| Temperature | ±0.5°C | 0.2°C/month | 0.1%/year |
| Pressure | ±2 mbar | 1 mbar/month | 0.1%/year |
| Flow | ±2% | 0.5%/month | 0.5%/year |
| Concentration | ±1% | 0.2%/month | 1%/year |

### E. Control Loop Tuning Parameters

| Control Loop | Type | P | I | D | Setpoint | Units |
|--------------|------|---|---|---|----------|-------|
| Cell Temperature | PID | 2.5 | 0.1 | 0.0 | 85 | °C |
| Pressure Differential | PI | 1.0 | 0.05 | - | 150 | mbar |
| Brine Flow | PI | 0.8 | 0.02 | - | 100 | m³/h |
| Current Distribution | P | 0.5 | - | - | Balanced | % |

### F. Alarm Limits

| Parameter | Low-Low | Low | High | High-High | Units |
|-----------|---------|-----|------|-----------|-------|
| Cell Voltage | - | 2.8 | 3.3 | 3.5 | V |
| Current Efficiency | 85 | 90 | - | - | % |
| Temperature | 75 | 80 | 90 | 95 | °C |
| Pressure Diff | 20 | 50 | 300 | 400 | mbar |
| H₂ in Cl₂ | - | - | 2 | 4 | vol% |
| Cl₂ in H₂ | - | - | 1 | 2 | vol% |

### G. Membrane Degradation Model

**Voltage Increase Over Time**:
$$V(t) = V_0 + \alpha \cdot \sqrt{t} + \beta \cdot J^2 \cdot t$$

Where:

- $V_0$ = Initial voltage (V)
- $t$ = Operating hours
- $J$ = Current density (kA/m²)
- $\alpha$ = 0.00003 V/√h (empirical)
- $\beta$ = 0.0000001 V·m²/(kA²·h) (empirical)

**Current Efficiency Decay**:
$$CE(t) = CE_0 - k_1 \cdot \log(1 + t/\tau) - k_2 \cdot N_{cycles}$$

Where:

- $CE_0$ = Initial current efficiency (%)
- $t$ = Operating hours
- $\tau$ = 8760 hours (characteristic time)
- $k_1$ = 0.5 %/log(h) (degradation rate)
- $k_2$ = 0.01 %/cycle (cycling penalty)
- $N_{cycles}$ = Number of start/stop cycles

### H. Energy Balance

**Cell Heat Generation**:
$$Q_{gen} = I \cdot (V_{cell} - E_{thermo})$$

**Heat Removal Required**:
$$Q_{cooling} = Q_{gen} - Q_{reaction} - Q_{losses}$$

Where:

- $Q_{gen}$ = Heat generated (kW)
- $I$ = Current (kA)
- $V_{cell}$ = Operating voltage (V)
- $E_{thermo}$ = Thermoneutral voltage (2.3V at 85°C)
- $Q_{reaction}$ = Heat of reaction (endothermic)
- $Q_{losses}$ = Heat losses to environment

### I. Mass Balance

**Theoretical Production Rates** (per kA):

- Cl₂: 1.323 kg/h
- H₂: 0.0376 kg/h (0.418 Nm³/h)
- NaOH (100%): 1.492 kg/h

**Actual Production**:
$$P_{actual} = P_{theoretical} \times CE \times Availability$$

### J. Economic Parameters

| Parameter | Value | Unit |
|-----------|-------|------|
| Electricity Price (base) | 50 | $/MWh |
| Electricity Price (peak) | 120 | $/MWh |
| Membrane Cost | 750 | $/m² |
| Membrane Area/Cell | 2.7 | m² |
| Membrane Lifetime (design) | 5 | years |
| Chlorine Price | 250 | $/MT |
| Caustic Price (50%) | 420 | $/MT |
| Hydrogen Value | 2000 | $/MT |

---
*Document Version: 1.0*
*Last Updated: 2024*
*Project Codename: Flux*
