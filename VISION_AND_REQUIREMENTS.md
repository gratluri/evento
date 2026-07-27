# evento: Vision and Requirements

## Date: July 27, 2026 (Updated)
### Original: July 18, 2026

---

## Core Vision

### The Problem

Modern enterprise software doesn't live in a single process. An order placed through a web frontend cascades through API gateways, message brokers, databases, CDC pipelines, payment processors, and inventory systems — often spanning dozens of services written in different languages, owned by different teams, running on different infrastructure. Yet the tools we use to validate these systems are stuck in a simpler era.

**JMeter** gives you a GUI to click through HTTP requests. **Gatling** gives you Scala scripts to hammer endpoints. **k6** gives you JavaScript to write load tests. **Locust** gives you Python. All of them share the same fundamental limitation: they think about testing as "send a request, check the response." They are **single-protocol, single-paradigm, human-operated tools** designed for a world where "testing" meant verifying that one API returned the right JSON.

That world no longer exists.

When an AI agent generates a microservice that publishes to Kafka, writes to PostgreSQL, triggers a CDC event, and expects a downstream Cassandra materialized view to be eventually consistent within 5 seconds — what tool validates that? When the agent needs to understand *why* a checkout flow fails under load at the exact moment the inventory service starts returning 503s — what tool provides that causal analysis? When the same agent needs to run the test again with modified chaos parameters, parse the structured results, fix the code, and re-validate — all without a human touching a keyboard — what tool closes that loop?

**Nothing does. That is why evento exists.**

### What evento Is

**evento is an intelligent, multi-protocol validation and optimization platform designed from the ground up for the age of agentic software development.**

It is not a load testing tool with extra features bolted on. It is not an API testing framework that happens to support Kafka. It is a fundamentally new kind of system that treats test execution as a **directed graph of stateful, protocol-agnostic steps** orchestrated through a declarative DSL, executed by a Rust-native engine, and designed to produce **machine-readable, causally-rich output** that both humans and AI agents can act on.

evento's core thesis is simple: **the testing tool and the code-generation agent must speak the same language.** If an AI agent can read a YAML file to understand what a test does, modify it to test a different hypothesis, submit it to an API, receive structured failure diagnostics with suggested fixes tied to specific code locations, and iterate until quality gates pass — then software development has crossed a threshold. The agent is no longer just writing code. It is *validating* code. It is *understanding* systems. It is closing the loop from intent to verified behavior, autonomously.

### Core Principles

1. **AI-First, Human-Friendly.** Every input and output is designed for machine consumption — YAML in, structured JSON out, programmatic API access — while remaining readable and authorable by humans. The DSL is simple enough for a developer to write a test in 10 minutes, and structured enough for an agent to generate one in 10 seconds.

2. **Protocol-Agnostic Orchestration.** A single test can span HTTP, Kafka, PostgreSQL, Cassandra, EventHub, gRPC, and embedded scripts. The DSL abstracts away protocol differences; the engine handles the wiring. A step is a step, whether it sends an HTTP POST or inserts a row into a database.

3. **Temporal and Causal Awareness.** evento doesn't just check that things happened — it checks that things happened *in the right order, within the right time window, and with the right causal relationships.* Temporal assertions like `within: 5s`, async observation patterns, and CDC tracking are first-class DSL constructs, not afterthoughts.

4. **Declarative Power, Scripting Escape Hatch.** 80% of testing scenarios are expressible in pure YAML: send a request, extract a value, validate a response, loop over a collection, branch on success or failure. For the remaining 20% — payload transformation, complex math, custom hashing — the Rhai embedded scripting engine provides a zero-overhead, sandboxed, Rust-native escape hatch, directly inside the YAML.

5. **Failure as a First-Class Concept.** Tests don't just pass or fail. Every step records its context before and after execution, latency, retry attempts, and the exact failure reason. The control flow graph supports `on_success` / `on_failure` branching, so recovery logic is part of the test definition, not an external concern.

6. **Rust-Native Performance.** The engine is written entirely in Rust. No GC pauses, no interpreter overhead, no JVM warmup. Protocol executors run as async Tokio tasks. The embedded Sled database provides sub-millisecond telemetry writes. The Rhai scripting engine compiles to native AST without a VM. This is not a wrapper around a scripting language — it is a systems-level tool.

---

## Primary Users

1. **AI Agents** (first-class citizen) — autonomous software development agents that generate, execute, analyze, and iterate on tests programmatically via the REST API
2. **Developers** — using evento to validate multi-service flows during development with a simple CLI and YAML files
3. **QA Engineers** — building comprehensive functional and performance validation suites with the full DSL
4. **DevOps/SRE** — production readiness, chaos engineering, and reliability testing

---

## What Has Been Built (Current State)

### ✅ Implemented and Validated

#### Core Engine
- **YAML DSL Parser** — Full eDSL parser supporting all top-level elements, step definitions, nested configurations, mock blocks, chaos injection, loop semantics, and temporal constructs. Comprehensive test suite with 59,000+ bytes of parser tests.
- **Execution Compiler** — Transforms parsed DSL into an `ExecutionPlan` with dependency-resolved `ExecutionTask` nodes, supporting both implicit sequential and explicit `wait_for` dependency graphs.
- **VuWorker Runtime** — The core execution loop that walks the task graph, manages `VuContext` state, handles retry/within temporal polling, step-level timeouts, and control flow branching (`on_success` / `on_failure`).
- **VuContext & Interpolation** — Thread-local context with `${variable}` interpolation, variable extraction from protocol responses, and boolean rule evaluation for `validate:` blocks.

#### Protocol Executors
- **HTTP** — Full `reqwest`-based executor with method support (GET, POST, PUT, DELETE), JSON body serialization, header injection, response body/header extraction, and cookie support.
- **PostgreSQL** — `sqlx`-based executor with parameterized queries, row extraction, and connection pooling.
- **Cassandra** — `scylla`-based executor with CQL query execution, keyspace management, and eventual consistency patterns.
- **Kafka / EventHub** — `rskafka`-based executor with topic publish, partition awareness, and message serialization.
- **Rhai Scripting** — Embedded `rhai` engine that injects `VuContext.variables` as a Rhai `Map`, executes user-defined scripts, and extracts mutations back into the context. Supports math, conditionals, string operations, and custom logic.

#### Control Flow Graph
- **Sequential Execution** — Implicit step-by-step ordering with dependency tracking.
- **`on_success` / `on_failure` Branching** — Steps can route to named recovery or skip steps based on execution outcome.
- **`within` Temporal Polling** — Steps can poll with a deadline (e.g., `within: 5s`), retrying until a condition is met or the duration expires.
- **`loop` Semantics** — Numeric count loops (`count: 5`), collection iteration (`over: item, from: ${list}`), and dynamic count resolution from context variables.
- **`async` / `wait_for` Synchronization** — Steps can declare `async: true` to execute in the background, with downstream steps using `wait_for: [step_name]` to synchronize.
- **Step-Level Timeouts** — Each step can declare a `timeout:` that wraps the protocol execution in a `tokio::time::timeout`, failing the step if exceeded.
- **Run-Level Timeout** — Global `timeout:` in config that caps the entire run duration.

#### Mock and Chaos Simulation
- **Mock Strategy** (`auto` / `disabled` / `required`) — Global config that controls whether steps use mock responses, real protocol calls, or require mocks. Script-only steps bypass mock requirements.
- **Mock Responses** — Steps can define `mock.response` with `status`, `latency`, and `body` to simulate external services without network calls.
- **Chaos Injection** — `mock.behavior` supports `error_rate` (probabilistic failure injection), `error_response` (custom error codes), and simulated latency.

#### Infrastructure
- **Admin REST API** — Actix-web server on port 8080 with endpoints for submitting runs (`POST /api/v1/tests/run`), listing runs, querying status, fetching step-level results, system metrics history, and an async audit demo.
- **Admin Dashboard UI** — Single-page HTML dashboard with real-time run monitoring, step-level telemetry inspection, namespace filtering, system metrics visualization, and execution history.
- **CLI Client** (`evento-client`) — Command-line tool for submitting YAML test plans, querying run status, and listing runs.
- **Sled Embedded Storage** — All run plans, execution states, step-level telemetry (context_before, context_after, latency, status), and VU context are persisted to an embedded Sled key-value store.
- **Docker Support** — Dockerfile and `docker-compose.yaml` for containerized deployment with PostgreSQL, Cassandra, and Kafka infrastructure.
- **Simulator Server** — Built-in mock HTTP server on port 8081 for testing without external dependencies.

#### Documentation
- **DSL Specification** (`DSL_SPECIFICATION.md`) — 2,500-line RFC-style specification covering all eDSL syntax, semantics, and execution model.
- **Architecture Document** (`ARCHITECTURE.md`) — Detailed system architecture documentation.
- **Competitive Analysis** (`COMPETITIVE_ANALYSIS.md`) — Comparative analysis against JMeter, Gatling, k6, Locust, and other tools.

### 🔲 Planned (Future Phases)

#### Phase: Load Testing & Observability
- **Ramp-up Configuration** — Gradual virtual user scaling with configurable ramp-up patterns.
- **Prometheus / Grafana Integration** — Real-time metric export to external observability platforms.
- **`track_metric`** — Custom business metric collection and aggregation within test runs.
- **Built-in Visualization** — Embedded time-series charts for latency distributions, error rates, and throughput.

#### Phase: Data Generation & External Feeds
- **Faker Integration** — Synthetic data generation (UUIDs, names, addresses, commerce data) via built-in faker functions.
- **External Data Sources** — Pull test data from databases, CSV files, or API endpoints.
- **Template Engine** — JSON/XML body templates with data binding.

#### Phase: Execution Modes
- **Replay Mode** — Replay captured production traffic at configurable speed.
- **Scheduled Mode** — CRON-based recurring test execution.
- **Parallel Execution** — True parallel step execution within a single run (current model is sequential with async background tasks).

#### Phase: Modularity & Extensibility
- **Subflow Modules** — Reusable test fragments with typed inputs/outputs.
- **Inheritance** — Base test configurations that can be extended and overridden.
- **Plugin System** — Rust trait-based plugin interface for custom protocols, validators, and data generators.
- **Step-Level Mocking** — Per-step mock override independent of global strategy.

#### Phase: Advanced Scripting
- **Multi-Runtime Support** — Python and WebAssembly script runtimes alongside Rhai.
- **Custom Function Libraries** — Reusable script function registries.
- **Script File References** — External `.rhai` script files referenced from YAML.

#### Phase: Enterprise Features
- **Distributed Workers** — Manager-worker architecture for multi-machine load generation.
- **Team Collaboration** — Shared test libraries, RBAC, and audit trails.
- **CI/CD Integration** — Native integration with GitHub Actions, GitLab CI, Jenkins.
- **Global Validation Rules** — End-of-run business rule assertions across all collected metrics.

---

## DSL Design Philosophy

### Declarative YAML Foundation

**Core principle:** Simple things should be simple, complex things should be possible.

```yaml
# Simple functional test — a developer writes this in 5 minutes
test: orders.create_order
scenario:
  - name: create_order
    protocol: http
    endpoint: http://localhost:8080/api/orders
    method: POST
    body:
      customerId: "customer-123"
      amount: 99.99
    extract:
      orderId: json.id
    validate:
      - response.status_code == 201
```

### Embedded Scripting with Rhai

**When declarative syntax hits limits, Rhai provides a zero-overhead escape hatch — directly inline.**

```yaml
# Complex logic stays in the YAML — no external files needed
- name: calculate_discount
  runtime: rhai
  script: |
    let base = parse_int(context.price);
    let mult = parse_float(context.multiplier);
    let raw_score = to_float(base) * mult;
    
    if context.is_premium == "true" {
        raw_score = raw_score + 50.0;
    }
    
    context.final_price = raw_score.to_string();
    context.hash = (raw_score * 3.14).to_int().to_string();
```

**Why Rhai:**
- Pure Rust — compiles to native AST, no VM overhead
- Sandboxed — no filesystem or network access from scripts
- Familiar syntax — JavaScript/Rust-like, no learning curve
- Bidirectional context — reads from and writes back to `VuContext`

### Control Flow as a Graph

**Not just linear steps** — evento supports complex execution patterns:

```yaml
scenario:
  - name: place_order
    protocol: http
    endpoint: /api/orders
    method: POST
    extract:
      orderId: json.id
    on_success: verify_database
    on_failure: log_error

  - name: verify_database
    protocol: postgres
    query: "SELECT status FROM orders WHERE id = '${orderId}'"
    within: 5s     # Poll until found or timeout
    validate:
      - result.status == "pending"
    on_success: verify_kafka
    on_failure: flag_db_issue

  - name: verify_kafka
    protocol: kafka
    topic: orders.created
    validate:
      - message.orderId == "${orderId}"

  - name: log_error
    protocol: script
    script: |
      context.error_logged = "true";

  - name: flag_db_issue
    protocol: http
    endpoint: /api/alerts
    method: POST
    body:
      issue: "Order ${orderId} not found in database within 5s"
```

**Key features:**
- **Temporal assertions** — `within: 5s` polls with a deadline
- **Causal branching** — `on_success` / `on_failure` route execution based on outcome
- **Multi-protocol orchestration** — HTTP → PostgreSQL → Kafka in a single flow
- **Loop semantics** — iterate over collections or fixed counts
- **Async synchronization** — `async: true` + `wait_for` for background tasks

---

## Execution Model

### Architecture

evento uses a **single-binary server** architecture (with distributed workers planned for future phases):

```
┌─────────────────────────────────────────────────┐
│                  evento server                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
│  │ REST API │  │ Admin UI │  │ Simulator    │  │
│  │ :8080    │  │ :8080    │  │ Mock :8081   │  │
│  └────┬─────┘  └──────────┘  └──────────────┘  │
│       │                                         │
│  ┌────▼─────────────────────────────────────┐   │
│  │           Run Manager                    │   │
│  │  ┌──────────┐  ┌──────────────────────┐  │   │
│  │  │ Compiler │  │ VuWorker Pool        │  │   │
│  │  │ (DSL →   │  │ ┌────┐ ┌────┐ ┌────┐│  │   │
│  │  │  Plan)   │  │ │VU 0│ │VU 1│ │VU n││  │   │
│  │  └──────────┘  │ └────┘ └────┘ └────┘│  │   │
│  │                └──────────────────────┘  │   │
│  └──────────────────────────────────────────┘   │
│       │                                         │
│  ┌────▼──────────────────────────────────────┐  │
│  │  Protocol Executors                       │  │
│  │  HTTP │ Postgres │ Cassandra │ Kafka │Rhai│  │
│  └───────────────────────────────────────────┘  │
│       │                                         │
│  ┌────▼──────────────────────────────────────┐  │
│  │  Sled Embedded Storage                    │  │
│  │  Plans │ State │ Telemetry │ Context      │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│              evento-client (CLI)                 │
│  run --plan test.yaml                           │
│  status --run-id <uuid>                         │
│  list                                           │
└─────────────────────────────────────────────────┘
```

### Execution Flow

1. **Submit** — User or AI agent sends a YAML plan via CLI or REST API
2. **Compile** — The Compiler parses the DSL, resolves namespaces, and produces an `ExecutionPlan` with dependency-ordered `ExecutionTask` nodes
3. **Dispatch** — The RunManager spawns VuWorkers (one per virtual user), each walking the task graph independently
4. **Execute** — Each VuWorker executes steps in order, routing to the appropriate Protocol Executor, managing `VuContext` state, handling retries/timeouts/branching
5. **Record** — Every step writes telemetry to Sled: context_before, context_after, latency_ms, status, retry_attempt
6. **Complete** — RunManager marks the run as `Completed` and results are queryable via API

### State Management

```yaml
# Extract values from one step
- name: create_order
  protocol: http
  endpoint: /api/orders
  method: POST
  extract:
    orderId: json.id
    timestamp: json.created_at

# Interpolate into subsequent steps
- name: get_order_status
  protocol: http
  endpoint: "/api/orders/${orderId}"

# Transform with Rhai scripting
- name: compute_signature
  runtime: rhai
  script: |
    context.sig = context.orderId + "_" + context.timestamp;
```

---

## AI Agent Integration

### Machine-Readable I/O

The entire evento interface is designed for programmatic consumption:

**Input:** Declarative YAML — parseable, generatable, diffable
**Output:** Structured JSON — step-level telemetry with full context snapshots

```json
{
  "run_id": "4d3531d1-4798-483b-aa66-b23d9e21b3cb",
  "status": "Success",
  "context_before": {
    "base_score": "150",
    "is_premium": "true",
    "multiplier": "2.5"
  },
  "context_after": {
    "base_score": "150",
    "final_score": "425.0",
    "is_premium": "true",
    "multiplier": "2.5",
    "verification_hash": "1334"
  },
  "latency_ms": 1,
  "retry_attempt": 1
}
```

### Agent Workflow

```
1. AI Agent reads user story or code change
   ↓
2. Agent generates evento YAML test
   ↓
3. Agent submits via REST API: POST /api/v1/tests/run
   ↓
4. evento compiles, executes, records telemetry
   ↓
5. Agent queries results: GET /api/v1/tests/runs/{id}/results
   ↓
6. Agent parses structured JSON, identifies failures
   ↓
7. Agent modifies code OR test based on context_before/context_after diffs
   ↓
8. Loop until all steps report status: "Success"
```

### Why This Matters for Agentic Development

Current AI coding agents can generate code, but they cannot **close the verification loop** for distributed systems. They can write a function and run a unit test. They cannot:

- Validate that a Kafka consumer processed a message within 3 seconds
- Verify that a CDC pipeline propagated a database write to a downstream read model
- Test that a payment retry mechanism recovers from a 503 after 2 attempts
- Confirm that an eventually-consistent system converges within an SLA window

evento gives agents the ability to **reason about distributed behavior across time, across protocols, and across service boundaries.** This is the missing piece between "AI writes code" and "AI validates systems."

---

## Chaos Engineering

### Declarative Chaos

Chaos scenarios are embedded directly in the DSL, not bolted on as a separate tool:

```yaml
mock:
  behavior:
    error_rate: 0.1          # 10% of requests fail
    error_response:
      status: 503
    latency:
      fixed: 200ms           # Simulate network delay
  response:
    status: 200
    body: '{"ok": true}'
```

**AI Agent Use Cases:**
1. **Hypothesis Testing:** "What happens if the payment gateway has 10% error rate?"
2. **Boundary Discovery:** "At what latency does the retry logic start failing?"
3. **Resilience Validation:** "Does the `on_failure` recovery path actually execute?"

---

## Observability and Results

### Step-Level Telemetry

Every step execution records:
- `context_before` — full variable state entering the step
- `context_after` — full variable state after extraction/mutation
- `status` — `Success` or `Failed(reason)`
- `latency_ms` — wall-clock execution time
- `retry_attempt` — which attempt this was (for retry/within loops)
- `executed_at` — UTC timestamp

### Planned Metrics (Future Phases)

- **Latency distributions:** min, max, mean, p50, p90, p95, p99
- **Throughput:** requests per second per protocol
- **Error rates:** by step, by protocol, by status code
- **Business metrics:** custom `track_metric` aggregations
- **Prometheus export:** real-time push to external monitoring

---

## Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Language | Rust (Edition 2024) | Zero-cost abstractions, no GC, async-native |
| Async Runtime | Tokio | Industry-standard Rust async executor |
| HTTP Client | reqwest 0.11 | Battle-tested HTTP with TLS, cookies, redirect |
| PostgreSQL | sqlx 0.7 | Compile-time checked queries, async |
| Cassandra | scylla 0.14 | High-performance CQL driver |
| Kafka | rskafka 0.5 | Pure-Rust Kafka client, no librdkafka |
| Scripting | rhai 1.25 | Embedded scripting, Rust-native, sandboxed |
| Web Framework | actix-web 4.4 | High-performance HTTP server |
| Storage | sled 0.34 | Embedded key-value store, zero-config |
| Serialization | serde + serde_yaml + serde_json | Universal Rust serialization |
| CLI | clap 4.0 | Derive-based argument parsing |

---

## Success Criteria

### For AI Agents
- Agent can generate valid evento YAML from code analysis
- Agent can submit tests via REST API and parse structured JSON results
- Agent can identify root cause from `context_before` / `context_after` diffs
- Agent can iteratively improve software based on evento feedback
- 90% of agent-generated tests run without human intervention

### For Human Users
- Developer can write a basic multi-step test in < 10 minutes
- Complex multi-protocol test is possible without external code (Rhai covers the gap)
- Test results are immediately actionable with step-level context snapshots
- Performance is orders of magnitude better than Python/JVM-based tools

### For Enterprise Adoption
- Supports 5+ protocols out of the box (HTTP, PostgreSQL, Cassandra, Kafka, Rhai)
- Containerized deployment with Docker Compose
- Embedded storage requires zero external infrastructure for dev/test
- Full DSL specification enables tooling ecosystem development

---

## Open Questions

1. **Distributed Workers:** What is the right coordination protocol for multi-machine load generation? (gRPC, NATS, or custom binary?)
2. **Data Generation:** Should faker integration be built-in or provided via Rhai script libraries?
3. **Insight Engine:** How deep should automated failure analysis go? (Pattern matching? Statistical? ML-based?)
4. **Plugin Distribution:** Package manager, Git-based registry, or WASM module store?
5. **MCP Integration:** How should evento expose itself to AI agents via Model Context Protocol tooling?

---

## Competitive Positioning

| Capability | JMeter | Gatling | k6 | Locust | **evento** |
|------------|--------|---------|----|---------|----|
| Multi-protocol in single test | ❌ | ❌ | ❌ | ❌ | ✅ |
| Temporal assertions (`within`) | ❌ | ❌ | ❌ | ❌ | ✅ |
| Control flow graph (on_success/on_failure) | ❌ | Limited | ❌ | ❌ | ✅ |
| Embedded scripting (zero-overhead) | ❌ | ❌ | ✅ (JS) | ✅ (Py) | ✅ (Rhai) |
| Machine-readable structured output | ❌ | Partial | Partial | ❌ | ✅ |
| Declarative YAML DSL | ❌ | ❌ | ❌ | ❌ | ✅ |
| REST API for programmatic access | ❌ | ❌ | Partial | ❌ | ✅ |
| Rust-native performance | ❌ | ❌ | ❌ (Go) | ❌ | ✅ |
| Step-level context telemetry | ❌ | ❌ | ❌ | ❌ | ✅ |
| Chaos injection in DSL | ❌ | ❌ | ❌ | ❌ | ✅ |

---

*This document reflects the state of evento as of July 27, 2026. It is a living document that will be updated as new capabilities are implemented and validated.*
