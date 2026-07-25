# evento: System Architecture (C1 Level)

## Date: July 18, 2026

---

## Overview

evento is an AI-first distributed testing and validation platform designed for agentic software development. This document provides the C1 (Context Level 1) architecture showing the major system components and their interactions.

---

## System Context Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         evento Platform                              │
│                                                                       │
│  ┌─────────────────┐         ┌──────────────────┐                  │
│  │   DSL Parser    │────────▶│  Test Executor   │                  │
│  │   & Validator   │         │    (Manager)     │                  │
│  └─────────────────┘         └──────────────────┘                  │
│           │                           │                              │
│           │                           │                              │
│           ▼                           ▼                              │
│  ┌─────────────────┐         ┌──────────────────┐                  │
│  │  Embedded DB    │         │   Worker Pool    │                  │
│  │   (DuckDB +     │◀────────│  (Distributed)   │                  │
│  │     Sled)       │         └──────────────────┘                  │
│  └─────────────────┘                  │                             │
│           │                            │                             │
│           │                            ▼                             │
│           │                   ┌──────────────────┐                  │
│           │                   │ Protocol Engines │                  │
│           │                   │ HTTP│Kafka│gRPC  │                  │
│           │                   │  DB │ JMS │SOAP  │                  │
│           │                   └──────────────────┘                  │
│           │                            │                             │
│           ▼                            │                             │
│  ┌─────────────────┐                  │                             │
│  │  Insight Engine │◀─────────────────┘                             │
│  │  (AI Analysis)  │                                                │
│  └─────────────────┘                                                │
│           │                                                          │
│           ▼                                                          │
│  ┌─────────────────┐                                                │
│  │ Results & APIs  │                                                │
│  └─────────────────┘                                                │
└───────────┬─────────────────────────────────────────────────────────┘
            │
            │  Interfaces
            │
            ▼
   ┌──────────────────────────────────────────────┐
   │         External Interfaces                   │
   │                                               │
   │  ┌──────────────┐      ┌─────────────────┐  │
   │  │  AI Agents   │      │ Human Users     │  │
   │  │  (API/SDK)   │      │ (CLI/API)       │  │
   │  └──────────────┘      └─────────────────┘  │
   │                                               │
   │  ┌──────────────┐      ┌─────────────────┐  │
   │  │ Observability│      │ Systems Under   │  │
   │  │ Platforms    │      │ Test            │  │
   │  │(Prometheus,  │      │(HTTP, Kafka,    │  │
   │  │ Grafana)     │      │ gRPC, DBs, etc.)│  │
   │  └──────────────┘      └─────────────────┘  │
   └──────────────────────────────────────────────┘
```

---

## Core Components

### 1. DSL Parser & Validator

**Responsibility:** Parse YAML test definitions, validate syntax and semantics.

**Key Functions:**
- Parse YAML test definitions
- Validate schema and structure
- Resolve imports and modules
- Compile test graph (steps, dependencies, control flow)
- Handle external script references

**Inputs:**
- YAML test files
- Module imports
- External script references

**Outputs:**
- Validated test execution plan (DAG - Directed Acyclic Graph with cycles for loops)
- Compilation errors/warnings

**Technology:**
- Rust: `serde_yaml` for parsing
- Custom validation engine
- Graph representation: `petgraph` crate

---

### 2. Test Executor (Manager)

**Responsibility:** Orchestrate test execution, manage job queue, coordinate workers.

**Key Functions:**
- Job queue management (FIFO, priority-based)
- Worker registry and health monitoring
- Work distribution and load balancing
- State coordination across distributed steps
- Real-time progress tracking
- Execution mode handling (real-time, replay, scheduled)

**Architecture Pattern:** Manager-Worker (Master-Slave)

**Communication Protocol Options:**
- **Primary:** gRPC (bi-directional streaming, strong typing)
- **Alternative:** NATS (cloud-native, pub-sub patterns)

**Inputs:**
- Test execution plans from DSL Parser
- Worker registration and heartbeats
- Result streams from workers

**Outputs:**
- Work assignments to workers
- Aggregated metrics and results
- Test status updates

**Technology:**
- Rust: `tokio` for async runtime
- gRPC: `tonic` crate
- Job queue: Custom implementation over Sled

---

### 3. Worker Pool

**Responsibility:** Execute test steps, generate data, send requests, collect metrics.

**Key Functions:**
- Receive work assignments from manager
- Execute test steps (protocol requests)
- Generate test data (faker, database sampling)
- Collect step-level metrics
- Stream results back to manager
- Handle step retries and error recovery
- Execute external scripts (Rust functions, Python, WASM)

**Deployment Models:**
- **Local Mode:** Workers on same machine as manager (threads/processes)
- **Distributed Mode:** Workers on separate machines
- **Auto-scaling:** Dynamic worker provisioning based on queue depth

**Inputs:**
- Work assignments (test steps to execute)
- Test data generation instructions
- Context/state from previous steps

**Outputs:**
- Request/response data
- Step-level metrics
- Execution status (success, failure, retry)

**Technology:**
- Rust: `tokio` for async execution
- Protocol-specific clients (see Protocol Engines)

---

### 4. Protocol Engines

**Responsibility:** Abstract protocol-specific communication details, provide unified interface.

**Supported Protocols (MVP → Full):**

#### MVP Phase
- **HTTP/HTTPS:** `reqwest` crate, OAuth2, mTLS support
- **Kafka:** `rdkafka` crate, Avro/Protobuf/JSON support, Schema Registry integration
- **PostgreSQL:** `tokio-postgres` crate, query execution, result parsing

#### Expansion Phase
- **gRPC:** `tonic` crate, reflection, streaming
- **JMS:** ActiveMQ, IBM MQ, RabbitMQ adapters
- **SOAP:** WSDL parsing, XML generation
- **MongoDB:** `mongodb` crate
- **Redis:** `redis` crate
- **Custom protocols:** Plugin system

**Interface Design:**
```rust
trait ProtocolEngine {
    async fn send_request(&self, request: ProtocolRequest) -> Result<ProtocolResponse>;
    async fn observe(&self, observation: ObservationConfig) -> Result<ObservedEvent>;
    fn collect_metrics(&self) -> Vec<Metric>;
    fn protocol_name(&self) -> &str;
}
```

**Key Features:**
- Protocol-agnostic request/response model
- Observation mode (for CDC, event streams)
- Built-in metric collection
- Error handling and retry logic

---

### 5. Embedded Database (Hybrid Architecture)

**Responsibility:** Persist test definitions, metrics, results, state, plugins.

**Strategy: Dual-Database Approach**

#### DuckDB: Analytical Workloads
- **Use cases:** Metrics storage, time-series queries, test results, insights
- **Strengths:** Fast analytical queries, SQL support, excellent aggregation performance
- **Schema:**
  - `test_runs`: Test execution metadata
  - `metrics`: Time-series metrics (high write throughput)
  - `insights`: AI-generated insights
  - `test_scripts`: Test definitions (searchable)

#### Sled: Operational State
- **Use cases:** Job queue, worker registry, distributed locks, runtime context
- **Strengths:** Pure Rust, high-performance key-value, transactional
- **Data:**
  - Job queue (pending, running, completed)
  - Worker heartbeats and status
  - Distributed state (shared counters, locks)
  - Test execution context

**Data Flow:**
```
Test Execution → Metrics → DuckDB (time-series)
              → State → Sled (operational)
              → Results → DuckDB (analytical)
```

**Technology:**
- DuckDB: `duckdb` crate
- Sled: `sled` crate
- Migration tools for schema evolution

---

### 6. Insight Engine (AI Analysis)

**Responsibility:** Analyze test results, identify patterns, generate actionable suggestions.

**Key Functions:**
- **Failure Analysis:** Root cause identification
- **Performance Analysis:** Bottleneck detection, latency patterns
- **Pattern Recognition:** Recurring issues, anomalies
- **Suggestion Generation:** Code locations, fixes, optimizations
- **Trend Analysis:** Compare with baseline, historical data
- **Business Impact:** Estimate consequences (revenue, UX impact)

**Analysis Techniques:**
- **Statistical:** Percentile analysis, outlier detection, correlation
- **Rule-based:** Known failure patterns, threshold violations
- **ML-based (future):** Anomaly detection, predictive analysis

**Output Format (AI-Friendly JSON):**
```json
{
  "insights": [
    {
      "type": "performance_bottleneck",
      "severity": "high",
      "description": "Database queries in checkout flow exceed 500ms at P99",
      "affected_component": "checkout_service",
      "code_locations": ["src/handlers/checkout.rs:142"],
      "suggested_fixes": [
        "Add database index on orders.customer_id",
        "Implement query result caching",
        "Consider read replica for query load"
      ],
      "evidence": {
        "query_time_p99_ms": 847,
        "threshold_ms": 500,
        "affected_requests_pct": 15.3
      }
    }
  ]
}
```

**Technology:**
- Rust statistical libraries
- DuckDB for analytical queries
- Future: Integration with LLM APIs for semantic analysis

---

### 7. Results & APIs

**Responsibility:** Expose test results and control interfaces for external consumers.

**Interfaces:**

#### REST API
```
POST   /api/v1/tests/run           # Submit test for execution
GET    /api/v1/tests/runs/{id}     # Get test run status
GET    /api/v1/tests/runs/{id}/results  # Get results
GET    /api/v1/tests/runs/{id}/insights # Get AI insights
DELETE /api/v1/tests/runs/{id}     # Cancel running test
GET    /api/v1/tests/metrics       # Query metrics
POST   /api/v1/plugins/register    # Register custom plugin
```

#### CLI
```bash
evento run test.yaml
evento status {run-id}
evento results {run-id} --format json|yaml|html
evento schedule test.yaml --cron "0 2 * * *"
evento insights {run-id}
```

#### SDK (for AI Agents)
```rust
// Rust SDK
let client = EventoClient::new("http://evento:8080");
let test = Test::from_yaml("test.yaml")?;
let run = client.submit_test(test).await?;
let results = run.wait_for_completion().await?;
let insights = results.insights();
```

```python
# Python SDK (for AI agents)
from evento_sdk import EventoClient

client = EventoClient("http://evento:8080")
run = client.run_test("test.yaml")
results = run.wait()
insights = results.insights()

for insight in insights.failures:
    print(f"Issue: {insight.description}")
    print(f"Fix: {insight.suggested_fixes}")
```

**Output Formats:**
- JSON (default, AI-friendly)
- YAML (human-readable)
- HTML (dashboard view)
- Prometheus metrics export
- CSV/Parquet (bulk export)

---

## Component Interaction Flow

### Test Execution Flow

```
┌──────────┐
│ AI Agent │
│  or User │
└────┬─────┘
     │
     │ 1. Submit test.yaml
     ▼
┌─────────────────┐
│   DSL Parser    │
│  & Validator    │
└────┬────────────┘
     │
     │ 2. Parsed test plan (DAG)
     ▼
┌─────────────────┐
│ Test Executor   │
│   (Manager)     │
└────┬────────────┘
     │
     │ 3. Queue job
     ▼
┌─────────────────┐
│  Embedded DB    │
│  (Sled: queue)  │
└────┬────────────┘
     │
     │ 4. Dequeue & assign work
     ▼
┌─────────────────┐
│  Worker Pool    │
│ (Distributed)   │
└────┬────────────┘
     │
     │ 5. Execute steps
     ▼
┌─────────────────┐
│ Protocol Engines│
│ (HTTP, Kafka,..)│
└────┬────────────┘
     │
     │ 6. Send requests
     ▼
┌─────────────────┐
│  System Under   │
│      Test       │
└────┬────────────┘
     │
     │ 7. Responses & metrics
     ▼
┌─────────────────┐
│  Worker Pool    │
└────┬────────────┘
     │
     │ 8. Stream results
     ▼
┌─────────────────┐
│ Test Executor   │
│   (Manager)     │
└────┬────────────┘
     │
     │ 9. Aggregate & store
     ▼
┌─────────────────┐
│  Embedded DB    │
│ (DuckDB:metrics)│
└────┬────────────┘
     │
     │ 10. Analyze
     ▼
┌─────────────────┐
│ Insight Engine  │
└────┬────────────┘
     │
     │ 11. Generate insights
     ▼
┌─────────────────┐
│  Results & APIs │
└────┬────────────┘
     │
     │ 12. Return results
     ▼
┌──────────┐
│ AI Agent │
│  or User │
└──────────┘
```

---

## Data Flow Architecture

### Metrics Pipeline

```
Worker Execution
    │
    ├─→ Request Latency ─────┐
    ├─→ Response Status ──────┤
    ├─→ Protocol Metrics ─────┤
    ├─→ Business Metrics ─────┤
    │                         │
    │                         ▼
    │                  ┌─────────────┐
    │                  │   Manager   │
    │                  │ Aggregation │
    │                  └──────┬──────┘
    │                         │
    │                         ▼
    │                  ┌─────────────┐
    │                  │   DuckDB    │
    │                  │ Time-Series │
    │                  └──────┬──────┘
    │                         │
    │                         ├─→ Real-time dashboard
    │                         ├─→ Prometheus export
    │                         └─→ Insight Engine
```

### State Management

```
Test Context State
    │
    ├─→ Extract: orderId ────────┐
    ├─→ Extract: authToken ───────┤
    │                             │
    │                             ▼
    │                      ┌────────────┐
    │                      │    Sled    │
    │                      │  Key-Value │
    │                      └─────┬──────┘
    │                            │
    │                            │ Shared across workers
    │                            │
    └─→ Next Step: use orderId ◄─┘
```

---

## Simulation & Mocking Architecture

evento includes a first-class mocking engine that acts as a **simulation interceptor** at the executor layer. It does *not* spin up actual mock servers on local ports. Instead, it intercepts outbound requests based on the step configuration and immediately returns synthesized responses.

### Real Protocol vs Simulation

The `mock_strategy` controls the simulation boundary:
- **Mocked Steps (Simulation):** When a step has a `mock` block (and the strategy permits it), the engine **never touches the network**. It fabricates a response, applies simulated latency/errors, and proceeds to `extract`/`validate`.
- **Non-Mocked Steps (Real Protocol):** The engine makes actual HTTP requests (via `reqwest`), Kafka publishes/consumes (via `rdkafka`), and DB queries (via `tokio-postgres`).

### Mock Lifecycle Management (MockRuntime)

Because mocks can be stateful (e.g. sequences of responses, failure injection tracking, call counts), the mock state is not global. To handle this, evento introduces the **MockRuntime** component, which strictly ties mock simulation state to the lifecycle of a specific test run.

```
                    Test Run Lifecycle
                    
  ┌──────────────────────────────────────────────┐
  │              TestRun Instance                 │
  │                                               │
  │  ┌──────────────┐    ┌────────────────────┐  │
  │  │ MockRuntime  │    │  ExecutionPlan     │  │
  │  │ (per-run)    │    │                    │  │
  │  │              │    └────────────────────┘  │
  │  │  ┌────────┐  │                            │
  │  │  │VU-0    │  │    ┌────────────────────┐  │
  │  │  │counters│  │    │  VuContext (0..N)  │  │
  │  │  └────────┘  │    │                    │  │
  │  │  ┌────────┐  │    └────────────────────┘  │
  │  │  │VU-1    │  │                            │
  │  │  │counters│  │    ┌────────────────────┐  │
  │  │  └────────┘  │    │  Worker Pool       │  │
  │  │  ...         │    │                    │  │
  │  └──────────────┘    └────────────────────┘  │
  │                                               │
  │  Created: on run start                       │
  │  Destroyed: on run complete/cancel            │
  └──────────────────────────────────────────────┘
```

- A **MockRuntime** is instantiated *per-test-run*.
- It tracks the call counters, sequences, and random number generator state per Virtual User (VU) and per Step.
- The `MockRuntime` intercepts execution right before the `StepExecutor` delegates to a real protocol driver, rendering dynamic templates (like `$request.*` and `$mock.call_count`) and simulating timeouts or errors before injecting the result back into the standard VU context flow.
- The `MockRuntime` is destroyed when the test run ends, ensuring complete isolation between test executions and preventing cross-test state leakage.

---

## Deployment Architecture

### Single-Node Deployment (Development/Small Tests)

```
┌─────────────────────────────────────┐
│      evento (Single Process)        │
│                                      │
│  ┌──────────┐    ┌──────────────┐  │
│  │ Manager  │◄───│ Worker Pool  │  │
│  │          │    │  (threads)   │  │
│  └────┬─────┘    └──────────────┘  │
│       │                              │
│  ┌────▼─────┐    ┌──────────────┐  │
│  │ DuckDB   │    │    Sled      │  │
│  │ Metrics  │    │    State     │  │
│  └──────────┘    └──────────────┘  │
│                                      │
│  API: localhost:8080                │
└─────────────────────────────────────┘
```

### Distributed Deployment (Production/Load Testing)

```
┌────────────────────────────────────────────────────┐
│                  Manager Node                       │
│                                                     │
│  ┌──────────────┐    ┌─────────────────────┐     │
│  │   Manager    │    │  Embedded DB        │     │
│  │  Orchestrator│◄───│ (DuckDB + Sled)     │     │
│  └──────┬───────┘    └─────────────────────┘     │
│         │                                          │
│         │  API: 0.0.0.0:8080                      │
└─────────┼──────────────────────────────────────────┘
          │
          │ gRPC
          │
          ├─────────────┬─────────────┬──────────────┐
          │             │             │              │
          ▼             ▼             ▼              ▼
    ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
    │ Worker 1 │  │ Worker 2 │  │ Worker 3 │  │ Worker N │
    │  Node    │  │  Node    │  │  Node    │  │  Node    │
    └──────────┘  └──────────┘  └──────────┘  └──────────┘
```

### Kubernetes Deployment (Cloud-Native)

```yaml
# Manager Deployment (StatefulSet for persistent storage)
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: evento-manager
spec:
  replicas: 1
  template:
    spec:
      containers:
      - name: manager
        image: evento/manager:latest
        volumeMounts:
        - name: data
          mountPath: /data

# Worker Deployment (Auto-scaling)
apiVersion: apps/v1
kind: Deployment
metadata:
  name: evento-workers
spec:
  replicas: 10
  template:
    spec:
      containers:
      - name: worker
        image: evento/worker:latest

# Horizontal Pod Autoscaler
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: evento-workers-hpa
spec:
  scaleTargetRef:
    name: evento-workers
  minReplicas: 5
  maxReplicas: 100
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

---

## Technology Stack

### Core Platform
- **Language:** Rust (performance, safety, concurrency)
- **Async Runtime:** Tokio
- **Serialization:** Serde (YAML, JSON)
- **Graph Processing:** petgraph

### Databases
- **Analytical:** DuckDB (embedded, SQL, analytics)
- **Operational:** Sled (embedded, key-value, Rust-native)

### Networking & Protocols
- **Manager-Worker:** Tonic (gRPC)
- **HTTP Client:** reqwest
- **Kafka:** rdkafka
- **gRPC:** tonic
- **PostgreSQL:** tokio-postgres
- **WebSocket:** tokio-tungstenite

### APIs & Interfaces
- **REST API:** Axum (web framework)
- **CLI:** clap (argument parsing)
- **SDK:** Native Rust, Python bindings (PyO3)

### Observability
- **Metrics:** Custom + Prometheus exposition format
- **Logging:** tracing crate
- **Tracing:** OpenTelemetry integration (future)

### Data Generation
- **Faker:** fake crate (Rust faker library)
- **Custom generators:** Plugin-based

### External Script Execution
- **Rust Functions:** Native compilation
- **Python:** PyO3 embedded interpreter
- **WebAssembly:** wasmer runtime

---

## Security Architecture

### Authentication & Authorization
```
┌──────────────┐
│  API Request │
└──────┬───────┘
       │
       │ 1. Extract token
       ▼
┌──────────────┐
│   API Gateway│
│  Auth Verify │
└──────┬───────┘
       │
       │ 2. Validate JWT/API Key
       ▼
┌──────────────┐
│ Authorization│
│    Engine    │
└──────┬───────┘
       │
       │ 3. Check permissions
       ▼
┌──────────────┐
│   Process    │
│   Request    │
└──────────────┘
```

### Security Features
- **API Authentication:** JWT tokens, API keys
- **Role-Based Access Control:** Admin, Developer, Agent roles
- **TLS/mTLS:** Secure manager-worker communication
- **Secrets Management:** Integration with HashiCorp Vault, K8s Secrets
- **Audit Logging:** All API calls and test executions logged

---

## Scalability & Performance

### Performance Targets
- **Throughput:** 10,000+ requests/second per worker node
- **Latency:** < 10ms manager-worker communication overhead
- **Workers:** Scale to 1,000+ concurrent workers
- **Test Scale:** Handle 100,000+ concurrent virtual users

### Scalability Strategies

**Horizontal Scaling:**
- Add more worker nodes dynamically
- Manager can coordinate 1,000+ workers
- Kubernetes HPA for auto-scaling

**Vertical Scaling:**
- Workers utilize all CPU cores (tokio multi-threaded runtime)
- Zero-copy optimizations where possible
- Memory-efficient data structures

**Data Partitioning:**
- Metrics partitioned by test_run_id
- DuckDB partitioned tables for large datasets
- Archive old test runs to cold storage (S3, etc.)

---

## Observability & Monitoring

### Internal Metrics (evento self-monitoring)
- Manager queue depth
- Worker utilization
- Database write throughput
- gRPC communication latency
- Memory and CPU usage

### Export Integrations
```
evento Metrics
    │
    ├─→ Prometheus (pull/push)
    ├─→ Grafana (dashboards)
    ├─→ Datadog
    ├─→ New Relic
    └─→ Custom webhook
```

### Logging Strategy
- **Structured Logging:** JSON format
- **Log Levels:** Error, Warn, Info, Debug, Trace
- **Centralized:** Forward to ELK, Loki, CloudWatch

---

## Extensibility & Plugin System

### Plugin Types

1. **Protocol Plugins:** Add new communication protocols
2. **Data Generator Plugins:** Custom fake data generators
3. **Validator Plugins:** Custom validation logic
4. **Transformer Plugins:** Data transformation functions
5. **Metrics Collector Plugins:** Custom metric extraction

### Plugin Interface

```rust
// Protocol plugin example
pub trait ProtocolPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    
    async fn send_request(
        &self,
        request: ProtocolRequest,
    ) -> Result<ProtocolResponse>;
    
    async fn observe(
        &self,
        config: ObservationConfig,
    ) -> Result<ObservedEvent>;
    
    fn metrics(&self) -> Vec<Metric>;
}

// Register plugin
#[no_mangle]
pub extern "C" fn evento_plugin_register() -> Box<dyn ProtocolPlugin> {
    Box::new(MyCustomProtocol::new())
}
```

### Plugin Distribution
- **Local:** Compiled `.so`/`.dylib`/`.dll` files
- **Registry:** Future package registry (crates.io-like)
- **Git:** Direct from git repositories

---

## AI Agent Integration Architecture

### Agent Interaction Model

```
┌─────────────────┐
│   AI Agent      │
│   Framework     │
└────────┬────────┘
         │
         │ 1. Generate test from code
         ▼
┌─────────────────┐
│ evento SDK      │
│ (Python/Rust)   │
└────────┬────────┘
         │
         │ 2. Submit test
         ▼
┌─────────────────┐
│ evento API      │
│ (REST/gRPC)     │
└────────┬────────┘
         │
         │ 3. Execute
         ▼
┌─────────────────┐
│ evento Platform │
└────────┬────────┘
         │
         │ 4. Stream results
         ▼
┌─────────────────┐
│ Insight Engine  │
└────────┬────────┘
         │
         │ 5. Structured insights
         ▼
┌─────────────────┐
│   AI Agent      │
│ (Parse & Act)   │
└─────────────────┘
```

### Key Design Decisions for AI Agents

1. **Machine-Readable Everything:**
   - YAML input (structured, parseable)
   - JSON output (schema-defined)
   - Clear success/failure semantics

2. **Self-Describing Tests:**
   - Test metadata includes purpose, author, version
   - Steps have descriptions explaining intent
   - Results include context for failures

3. **Actionable Insights:**
   - Not just "failed" but "why" and "how to fix"
   - Code location references
   - Multiple suggested solutions ranked by likelihood

4. **Idempotency:**
   - Same test → same results (given same system state)
   - Deterministic data generation (seedable randomness)
   - Reproducible test runs

5. **Streaming Results:**
   - Real-time progress updates
   - Early failure detection
   - Agent can react mid-test

---

## MVP Implementation Roadmap

### Phase 1: Core Foundation (Month 1-3)

**Components to Build:**
1. DSL Parser (YAML → Test Plan)
2. Manager (single-node, in-process workers)
3. 3 Protocol Engines (HTTP, Kafka, PostgreSQL)
4. Embedded DB setup (DuckDB + Sled)
5. Basic metrics collection
6. CLI interface

**Deliverables:**
- Run simple HTTP test
- Kafka produce/consume test
- Database query test
- Store metrics in DuckDB
- Query test results

### Phase 2: Distribution & AI (Month 4-6)

**Components to Build:**
1. gRPC manager-worker protocol
2. Distributed worker deployment
3. Insight Engine (rule-based)
4. REST API
5. Python SDK
6. AI-friendly output format

**Deliverables:**
- Distribute test across multiple workers
- Generate actionable insights
- AI agent can run tests via API
- Example agent integration

### Phase 3: Advanced Features (Month 7-9)

**Components to Build:**
1. External script execution (Rust functions, Python)
2. Plugin system
3. Module system (imports, inheritance)
4. Chaos engineering features
5. Scheduling and replay modes

**Deliverables:**
- Complex control flow (loops, branches)
- Custom protocol via plugin
- Scheduled test execution
- Replay production traffic

### Phase 4: Enterprise & Polish (Month 10-12)

**Components to Build:**
1. Built-in visualization dashboard
2. Team collaboration features
3. RBAC and security
4. More protocol engines (gRPC, JMS, SOAP)
5. CI/CD integrations
6. Performance optimizations

**Deliverables:**
- Web UI for viewing results
- Multi-user support
- Enterprise protocol support
- Production-ready deployment

---

## Critical Design Decisions

### 1. Manager-Worker Protocol: gRPC

**Rationale:**
- Strong typing (protobuf schemas)
- Bi-directional streaming (results, cancellation)
- Wide language support (future polyglot workers)
- Excellent performance
- Built-in auth and encryption

**Alternative Considered:** NATS (more cloud-native, but less typed)

### 2. Embedded Database: DuckDB + Sled

**Rationale:**
- **DuckDB:** Best-in-class for analytical queries, perfect for metrics
- **Sled:** Pure Rust, transactional, perfect for operational state
- No external dependencies (easy deployment)
- High performance

**Alternative Considered:** PostgreSQL (external, more complex deployment)

### 3. DSL: YAML

**Rationale:**
- Human-readable
- Machine-parseable
- Standard format (no learning curve)
- Supports comments
- Extensible via external scripts

**Alternative Considered:** Custom syntax (more power, steeper learning curve)

### 4. Language: Rust

**Rationale:**
- Performance (10x+ over Python)
- Memory safety (no GC pauses during load tests)
- Excellent async support (tokio)
- Growing ecosystem
- Binary distribution (no runtime dependencies)

---

## Risk Mitigation

### Technical Risks

**Risk:** Complexity of distributed coordination
**Mitigation:** Start single-node, add distribution incrementally

**Risk:** Protocol plugin API stability
**Mitigation:** Version plugins, provide migration tools

**Risk:** DuckDB stability with high write throughput
**Mitigation:** Benchmark early, batch writes, consider write-ahead-log tuning

**Risk:** Insight Engine quality (false positives)
**Mitigation:** Start conservative (rule-based), add confidence scores, user feedback loop

### Adoption Risks

**Risk:** Learning curve for DSL
**Mitigation:** Extensive examples, templates, AI agent can generate tests

**Risk:** Competition from established tools
**Mitigation:** Focus on AI-first differentiation, modern DevEx, multi-protocol support

---

## Success Metrics

### Technical Metrics
- **Performance:** 10,000+ req/sec per worker
- **Latency:** < 10ms overhead
- **Scale:** Support 1,000+ workers
- **Reliability:** 99.9% uptime for manager

### Adoption Metrics
- **GitHub Stars:** 1,000+ in first 6 months
- **AI Agent Integrations:** 5+ frameworks
- **Community Contributors:** 50+ contributors
- **Enterprise Pilots:** 10+ companies

### Business Metrics (Future)
- **Open Source Users:** 10,000+ downloads
- **Enterprise Customers:** 100+ paying customers
- **Cloud Service Users:** 1,000+ registered users

---

## Next Steps

1. **Validate Architecture:** Review with potential users and Rust experts
2. **Prototype Core Components:** DSL Parser, Manager, Single Protocol Engine
3. **Benchmark DuckDB:** Verify write throughput for metrics workload
4. **Design gRPC Protocol:** Define manager-worker protobuf schemas
5. **Create Detailed Component Specs:** Break down each component into modules
6. **Set Up Development Environment:** Rust toolchain, CI/CD, testing framework

---

## Open Questions

1. **gRPC vs NATS for manager-worker?** (Start gRPC, evaluate NATS for cloud)
2. **How to handle very large test results?** (Streaming? Pagination? Compression?)
3. **Plugin sandboxing?** (WASM? Linux namespaces? Trust model?)
4. **Multi-tenancy in cloud service?** (Database per tenant? Shared with isolation?)
5. **LLM integration for insight engine?** (Self-hosted? API-based? Privacy concerns?)
