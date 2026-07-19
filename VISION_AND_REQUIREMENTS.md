# evento: Vision and Requirements

## Date: July 18, 2026

---

## Core Vision

**evento is an intelligent validation and optimization platform designed for agentic software development.**

### Not Just a Testing Tool

evento is being built from the ground up to enable AI agents to:
- **Functionally validate** complex distributed system flows
- **Perform load and performance analysis** with business context
- **Extract key insights** from test executions
- **Iteratively improve** software under construction

### Primary Users

1. **AI Agents** (first-class citizen) - autonomous software development agents
2. **Developers** - using evento as part of their workflow
3. **QA Engineers** - functional and performance validation
4. **DevOps/SRE** - production readiness and reliability testing

---

## Key Differentiator: AI-First Design

### Why This Matters

Current testing tools (JMeter, Gatling, k6, Locust) were designed for human operators. evento is designed for **machine consumption and generation** while remaining human-readable.

### What AI Agents Need

1. **Structured, parseable output** - not just logs, but actionable insights
2. **Self-describing tests** - DSL that explains what it's testing and why
3. **Iterative refinement** - ability to modify tests based on observed behavior
4. **Semantic understanding** - business-level metrics, not just technical metrics
5. **Failure diagnosis** - not just "this failed" but "why and how to fix"

---

## DSL Design Philosophy

### Declarative YAML Foundation

**Core principle:** Simple things should be simple, complex things should be possible.

```yaml
# Simple declarative test
test: order_creation
scenario:
  - name: create_order
    protocol: https
    endpoint: /api/orders
    method: POST
    body:
      template: order_create.json
      data:
        customerId: {{ faker.uuid() }}
        amount: {{ random.decimal(10, 1000) }}
```

### Extensibility Through External Code

**Principle:** When declarative syntax hits limits, seamlessly call external code.

```yaml
# Complex logic delegates to external code
test: complex_order_flow
imports:
  - ./transformers/order_transformer.rs
  - ./validators/business_rules.py
  
scenario:
  - name: transform_order
    script: order_transformer::transform
    input: "{{ context.raw_order }}"
    output_to: transformed_order
    
  - name: validate_business_rules
    script: business_rules.validate_order
    input: "{{ context.transformed_order }}"
    expect: 
      valid: true
```

**Supported external code:**
- Rust functions (compiled for performance)
- Python scripts (for rapid prototyping, ML integrations)
- JavaScript/TypeScript (for web-focused transformations)
- WebAssembly modules (for custom compiled logic)

### Functions, Transformers, and Mappers

```yaml
# Reusable functions
functions:
  - name: calculate_tax
    language: rust
    source: ./functions/tax.rs
    
  - name: enrich_customer_data
    language: python
    source: ./functions/customer_enrichment.py

# Use in flow
scenario:
  - name: process_order
    transform:
      - input: "{{ context.order.amount }}"
        function: calculate_tax
        params:
          region: "{{ context.customer.region }}"
        output_to: tax_amount
        
      - input: "{{ context.customer.id }}"
        function: enrich_customer_data
        output_to: customer_profile
```

---

## Flow Control: Beyond Simple Sequences

### Inverted Graph Model

**Not just linear steps** - evento supports complex control flow:

```yaml
scenario:
  graph:
    # Start nodes
    start:
      - order_received
      - inventory_check
    
    # Parallel execution
    parallel:
      - name: order_received
        protocol: kafka
        topic: orders.new
        next: 
          - validate_order
          - notify_warehouse
      
      - name: inventory_check
        protocol: grpc
        service: inventory.Check
        next: reserve_inventory
    
    # Conditional branching
    - name: validate_order
      validate:
        - order.amount > 0
        - order.customerId exists
      on_success: charge_payment
      on_failure: reject_order
    
    # Loops and iterations
    - name: retry_payment
      protocol: https
      endpoint: /api/payments
      retry:
        max_attempts: 3
        backoff: exponential
        on_each_attempt:
          - log_attempt
      on_success: confirm_order
      on_max_retries: cancel_order
    
    # Convergence points
    - name: confirm_order
      wait_for:
        - charge_payment
        - reserve_inventory
      then: send_confirmation
```

### Event-Driven Flows

**Your CDC example - observing causality across systems:**

```yaml
scenario:
  - name: receive_order_event
    protocol: kafka
    topic: orders.new
    extract:
      orderId: message.orderId
      timestamp: message.timestamp
    trigger: update_database
  
  - name: update_database
    protocol: database
    connection: postgres://orders-db
    query: |
      INSERT INTO orders (id, customer_id, amount, status)
      VALUES ('{{ context.orderId }}', '{{ context.customerId }}', {{ context.amount }}, 'pending')
    trigger: observe_cdc
  
  - name: observe_cdc
    protocol: kafka
    topic: orders.cdc
    mode: observe
    expect:
      message.operation: INSERT
      message.table: orders
      message.data.id: "{{ context.orderId }}"
    within: 5s  # Must occur within 5 seconds
    on_timeout: flag_cdc_issue
    on_success: validate_cdc_content
  
  - name: validate_cdc_content
    validate:
      - cdc_event.data.amount == context.amount
      - cdc_event.data.customer_id == context.customerId
    track_metric:
      name: cdc_latency
      value: "{{ cdc_event.timestamp - context.timestamp }}"
```

**Key features:**
- **Temporal assertions** - "within 5 seconds"
- **Causal tracking** - observe downstream effects
- **Multi-protocol orchestration** - Kafka → Database → Kafka observation

---

## Modularity and Reusability

### Subflow Modules

```yaml
# File: modules/auth_flow.yaml
module: authenticate_user
inputs:
  - username
  - password
outputs:
  - authToken
  - userId
  
steps:
  - name: login
    protocol: https
    endpoint: /api/auth/login
    body:
      username: "{{ inputs.username }}"
      password: "{{ inputs.password }}"
    extract:
      authToken: response.body.token
      userId: response.body.user.id
```

```yaml
# File: test_checkout.yaml
test: checkout_flow
imports:
  - modules/auth_flow.yaml
  - modules/payment_flow.yaml
  - modules/inventory_flow.yaml

scenario:
  - name: authenticate
    use_module: authenticate_user
    with:
      username: "{{ faker.username() }}"
      password: "test_password_123"
    outputs_to: auth_context
  
  - name: add_to_cart
    protocol: https
    headers:
      Authorization: "Bearer {{ auth_context.authToken }}"
    endpoint: /api/cart
    # ... rest of flow
```

### Inheritance

```yaml
# Base test configuration
base: api_test_base.yaml

test: specific_order_test
extends: base

# Override specific behaviors
config:
  timeout: 30s  # Override base timeout
  
scenario:
  # Inherits auth flow from base
  - inherit: base.authenticate
  
  # Add specific steps
  - name: create_large_order
    protocol: https
    # ...
```

---

## AI Agent Integration

### Machine-Readable Test Results

```yaml
# Output format designed for AI consumption
result:
  test_id: "order_flow_test_20260718_143022"
  status: partial_failure
  duration_ms: 2341
  
  summary:
    total_steps: 15
    passed: 12
    failed: 2
    skipped: 1
  
  failures:
    - step: observe_cdc
      reason: timeout_exceeded
      expected: "CDC event within 5s"
      actual: "No CDC event received"
      context:
        orderId: "uuid-1234"
        database_write_succeeded: true
        kafka_topic_accessible: true
      suggested_fixes:
        - "Check CDC connector configuration"
        - "Verify database CDC is enabled"
        - "Increase timeout threshold"
      
    - step: validate_inventory_update
      reason: assertion_failed
      expected: "inventory.quantity == 95"
      actual: "inventory.quantity == 100"
      context:
        initial_quantity: 100
        order_quantity: 5
        inventory_service_response_code: 200
      suggested_fixes:
        - "Verify inventory reservation logic"
        - "Check for race conditions in inventory service"
  
  insights:
    performance:
      - metric: cdc_latency_p99
        value: 4.2s
        threshold: 5s
        status: warning
        recommendation: "P99 latency close to timeout threshold - consider increasing buffer"
    
    patterns:
      - observation: "3 out of 100 CDC events exceeded 3s latency"
        severity: low
        suggestion: "Investigate CDC connector performance during peak load"
    
    business_metrics:
      - name: order_success_rate
        value: 0.98
        trend: stable
        
      - name: inventory_consistency_rate
        value: 0.87
        trend: declining
        alert: "Inventory consistency issues detected"
```

### Agent-Friendly Commands

```bash
# AI agent can invoke evento programmatically
evento run test_suite.yaml \
  --output-format json \
  --include-suggestions \
  --trace-level verbose \
  --export-metrics prometheus

# Agent parses structured output
evento analyze results/test_run_123.json \
  --identify-patterns \
  --suggest-improvements \
  --compare-baseline results/baseline.json
```

### Iterative Improvement Loop

```yaml
# AI agent workflow
workflow:
  1. Generate initial test based on code analysis
  2. Run test with evento
  3. Parse structured results
  4. Identify failure patterns
  5. Modify code OR test based on insights
  6. Re-run and validate
  7. Repeat until quality thresholds met

# evento supports this by providing:
# - Structured, actionable output
# - Baseline comparison
# - Trend analysis
# - Suggested fixes tied to code locations
```

---

## Extensibility Architecture

### Plugin System

```rust
// Rust plugin interface
pub trait EventoPlugin {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    
    // Protocol support
    fn supported_protocols(&self) -> Vec<Protocol>;
    fn execute_request(&self, request: Request) -> Result<Response>;
    
    // Data generation
    fn generate_data(&self, schema: &Schema) -> Result<Value>;
    
    // Validation
    fn validate_response(&self, response: &Response, rules: &[Rule]) -> ValidationResult;
    
    // Metrics
    fn collect_metrics(&self) -> Vec<Metric>;
}
```

```yaml
# Use custom plugins
test: custom_protocol_test
plugins:
  - name: my_custom_protocol
    path: ./plugins/custom_protocol.so
    
scenario:
  - name: test_custom_system
    protocol: my_custom_protocol
    # Plugin handles execution
```

### Extensibility Points

1. **Protocols** - add new communication protocols
2. **Data generators** - custom fake data generators
3. **Validators** - custom validation logic
4. **Transformers** - data transformation functions
5. **Metrics collectors** - custom metric extraction
6. **Output formats** - custom reporting formats
7. **Execution strategies** - custom load patterns

---

## Programming Capability Assessment

### Current Thinking: Hybrid Approach

**Tier 1: Built-in DSL Features (80% use cases)**
- Standard control flow (if/else, loops, parallel)
- Protocol interactions
- Data generation and validation
- Metrics collection

**Tier 2: Scripting Extensions (15% use cases)**
- Custom transformations
- Complex business logic
- External system integrations

**Tier 3: Plugin Development (5% use cases)**
- New protocol support
- Custom execution strategies
- Specialized metrics collectors

### Extensibility Over Feature Bloat

**Philosophy:** Rather than building every feature, build a solid core with clear extension points.

```yaml
# Core is minimal but powerful
core_features:
  - YAML DSL parser
  - Multi-protocol execution engine
  - Data generation framework
  - Validation engine
  - Metrics collection and export
  - Result analysis and insights

extension_mechanisms:
  - Rust function calls (compiled performance)
  - Python/JS scripting (flexibility)
  - WebAssembly plugins (portable extensions)
  - External tool integration (CLI tools, APIs)
```

---

## Example: Complete E-Commerce Flow Test

```yaml
test: complete_ecommerce_flow
description: "Validate entire order lifecycle with CDC, inventory, and payment"
author: "AI Agent v2.3"
created: "2026-07-18T14:30:00Z"

imports:
  - modules/auth.yaml
  - modules/payment.yaml
  - transformers/order_enrichment.rs

config:
  duration: 5m
  virtual_users: 100
  ramp_up: 30s

data_sources:
  - name: test_customers
    source: database
    connection: postgres://test-db/customers
    query: "SELECT id, email, tier FROM customers WHERE test_account = true"
    
  - name: test_products
    generator: faker
    fields:
      productId: uuid
      name: commerce.product_name
      price: number.decimal(10, 1000, 2)
      inventory: number.int(50, 500)

scenario:
  graph:
    start: authenticate
    
    - name: authenticate
      use_module: auth.login
      with:
        email: "{{ data.test_customers.email }}"
      outputs_to: session
      next: browse_products
    
    - name: browse_products
      protocol: https
      endpoint: /api/products
      headers:
        Authorization: "Bearer {{ session.authToken }}"
      validate:
        - response.status == 200
        - response.body.products.length > 0
      extract:
        products: response.body.products
      next: add_to_cart
    
    - name: add_to_cart
      loop:
        count: "{{ random.int(1, 5) }}"
        over: selected_product
        from: "{{ context.products }}"
      do:
        protocol: https
        endpoint: /api/cart/items
        method: POST
        body:
          productId: "{{ selected_product.id }}"
          quantity: "{{ random.int(1, 3) }}"
      next: checkout
    
    - name: checkout
      protocol: https
      endpoint: /api/checkout
      method: POST
      extract:
        orderId: response.body.orderId
        totalAmount: response.body.total
      next:
        - observe_order_event
        - charge_payment
    
    - name: observe_order_event
      protocol: kafka
      topic: orders.created
      mode: observe
      expect:
        message.orderId: "{{ context.orderId }}"
      within: 3s
      next: verify_order_in_db
    
    - name: verify_order_in_db
      protocol: database
      connection: postgres://orders-db
      query: "SELECT * FROM orders WHERE id = '{{ context.orderId }}'"
      validate:
        - result.rows.length == 1
        - result.rows[0].status == 'pending'
      next: observe_cdc_event
    
    - name: observe_cdc_event
      protocol: kafka
      topic: orders.cdc
      mode: observe
      expect:
        message.operation: INSERT
        message.table: orders
        message.after.id: "{{ context.orderId }}"
      within: 5s
      track_metric:
        name: cdc_propagation_latency
        value: "{{ now() - context.checkout_timestamp }}"
      next: wait_for_completion
    
    - name: charge_payment
      use_module: payment.charge
      with:
        orderId: "{{ context.orderId }}"
        amount: "{{ context.totalAmount }}"
        customerId: "{{ session.userId }}"
      outputs_to: payment_result
      retry:
        max_attempts: 3
        backoff: exponential
      on_success: update_inventory
      on_failure: cancel_order
    
    - name: update_inventory
      parallel:
        over: cart_item
        from: "{{ context.cart_items }}"
      do:
        protocol: grpc
        service: inventory.InventoryService/Reserve
        message:
          productId: "{{ cart_item.productId }}"
          quantity: "{{ cart_item.quantity }}"
          orderId: "{{ context.orderId }}"
      validate:
        - all.response.status == "RESERVED"
      next: wait_for_completion
    
    - name: wait_for_completion
      wait_for:
        - observe_cdc_event
        - update_inventory
      timeout: 15s
      on_timeout: flag_incomplete_order
      on_success: verify_consistency
    
    - name: verify_consistency
      script: |
        # Custom Rust function for complex validation
        validators::verify_order_consistency(
          order_id: context.orderId,
          expected_items: context.cart_items,
          db_connection: "postgres://orders-db",
          inventory_service: "inventory.InventoryService"
        )
      on_success: complete
      on_failure: log_consistency_issue

validation:
  business_rules:
    - name: order_completion_rate
      threshold: 0.95
      actual: "{{ metrics.completed_orders / metrics.total_orders }}"
      
    - name: cdc_propagation_p99
      threshold: 5s
      actual: "{{ metrics.cdc_propagation_latency.p99 }}"
      
    - name: inventory_consistency
      threshold: 1.0
      actual: "{{ metrics.inventory_matches / metrics.orders_checked }}"

outputs:
  - format: json
    file: results/ecommerce_test_{{ timestamp }}.json
    
  - format: prometheus
    endpoint: http://prometheus:9090
    
  - format: ai_insights
    file: results/insights_{{ timestamp }}.json
    include:
      - failure_patterns
      - performance_bottlenecks
      - suggested_improvements
      - code_locations_to_investigate
```

---

## Next Steps

### Phase 1: Core Architecture (Months 1-3)
1. Design and implement YAML DSL parser
2. Build multi-protocol execution engine (start with HTTP, Kafka, gRPC)
3. Implement basic data generation (faker integration)
4. Create result analysis and insight engine

### Phase 2: AI Integration (Months 4-6)
5. Design machine-readable output format
6. Implement suggestion engine
7. Build baseline comparison and trend analysis
8. Create programmatic API for agent integration

### Phase 3: Extensibility (Months 7-9)
9. Implement external script calling (Rust, Python)
10. Build plugin system
11. Create module and inheritance system
12. Documentation and examples

### Phase 4: Enterprise Features (Months 10-12)
13. Distributed execution
14. Advanced observability integrations
15. Enterprise protocol support (JMS, SOAP, etc.)
16. Team collaboration features

---

## Execution Model

### Manager-Worker Architecture

**Core Concept:** evento uses a distributed manager-worker model for scalable test execution.

#### Manager Responsibilities
- Parse and validate DSL test definitions
- Queue test jobs
- Distribute work to available workers
- Aggregate results from workers
- Coordinate distributed test execution
- Maintain test state and orchestration

#### Worker Responsibilities
- Execute assigned test steps
- Generate test data
- Send protocol requests
- Collect metrics
- Report results back to manager

#### Distribution Model
- **Single Machine Mode:** Manager and workers on same machine (development/small tests)
- **Distributed Mode:** Manager orchestrates workers across multiple machines (load testing)
- **Auto-scaling:** Workers can be dynamically added/removed based on load

#### Manager-Worker Protocol
**Options to evaluate:**
1. **gRPC** - High performance, bi-directional streaming, strong typing
2. **ZeroMQ** - Low latency, flexible patterns, language agnostic
3. **NATS** - Cloud-native messaging, excellent for distributed scenarios
4. **Custom binary protocol over TCP** - Maximum performance, full control

**Recommendation:** Start with gRPC for strong typing and streaming support, evaluate NATS for cloud-native deployments.

### Execution Semantics

#### Real-Time Execution
```yaml
execution:
  mode: realtime
  duration: 5m
  virtual_users: 100
  ramp_up: 30s
```
- Execute tests immediately as they're submitted
- Live metric streaming
- Interactive feedback

#### Replay Execution
```yaml
execution:
  mode: replay
  source: production_traffic_capture.log
  speed: 1.5x  # Replay at 1.5x speed
  time_range:
    start: "2026-07-15T10:00:00Z"
    end: "2026-07-15T11:00:00Z"
```
- Replay captured production traffic
- Useful for regression testing
- Can adjust replay speed

#### Scheduled Execution
```yaml
execution:
  mode: scheduled
  schedule:
    cron: "0 2 * * *"  # Daily at 2 AM
    timezone: UTC
  on_failure:
    notify:
      - slack: "#alerts"
      - email: "team@example.com"
```
- Periodic test execution
- CI/CD integration
- Automated regression testing

### State Management

```yaml
# Extract values from one step
- name: create_order
  extract:
    orderId: response.body.id
    timestamp: response.headers.timestamp
  
# Reuse in subsequent steps
- name: get_order_status
  protocol: https
  endpoint: /api/orders/{{ context.orderId }}
  
# Share state across distributed workers
- name: shared_counter
  state:
    type: distributed
    key: order_counter
    operation: increment
```

---

## Observability and Results

### Comprehensive Metrics (Parity with Gatling/k6/JMeter++)

#### Request-Level Metrics
- **Latency:** min, max, mean, median, p50, p90, p95, p99, p99.9
- **Throughput:** requests per second
- **Error rates:** by type, by endpoint, by status code
- **Success rate:** percentage of successful requests

#### Protocol-Specific Metrics
```yaml
metrics:
  http:
    - connection_time
    - dns_lookup_time
    - tls_handshake_time
    - time_to_first_byte
    - download_time
    - status_code_distribution
    
  kafka:
    - produce_latency
    - consume_latency
    - partition_distribution
    - message_size_distribution
    - consumer_lag
    
  grpc:
    - stream_duration
    - message_count
    - compression_ratio
    
  database:
    - query_execution_time
    - row_count
    - connection_pool_utilization
```

#### Business-Level Metrics
```yaml
custom_metrics:
  - name: order_approval_rate
    calculate: "successful_orders / total_orders"
    dimensions:
      - customer_tier
      - payment_method
      - region
    
  - name: inventory_consistency_rate
    calculate: "consistent_inventory_checks / total_inventory_checks"
```

### Integration Points

#### Phase 1: External Observability Platforms
```yaml
export:
  prometheus:
    endpoint: "http://prometheus:9090"
    push_interval: 10s
    
  grafana:
    dashboard_templates: true
    auto_provision: true
    
  datadog:
    api_key: ${DATADOG_API_KEY}
    tags:
      - env:test
      - service:evento
```

#### Phase 2: Built-in Observability (Long-term Goal)

**Embedded Time-Series Database:**
- Store all metrics internally
- Query interface for analysis
- Built-in visualization dashboard
- Export to standard formats (CSV, JSON, Parquet)

**AI-Friendly Result Format:**
```json
{
  "test_run_id": "uuid",
  "timestamp": "2026-07-18T14:30:00Z",
  "status": "completed",
  "metrics": {
    "http_requests": {
      "total": 10000,
      "success": 9850,
      "failed": 150,
      "latency_p99_ms": 245.3
    }
  },
  "insights": {
    "performance_issues": [
      {
        "type": "latency_spike",
        "severity": "high",
        "description": "P99 latency exceeded 500ms during minute 3-5",
        "affected_endpoint": "/api/checkout",
        "suggested_investigation": [
          "Check database query performance",
          "Review payment gateway response times"
        ],
        "code_locations": [
          "src/handlers/checkout.rs:45"
        ]
      }
    ],
    "business_impact": {
      "estimated_failed_transactions": 150,
      "revenue_impact_usd": 45000
    }
  }
}
```

---

## Data Management

### Embedded Database Strategy

**Requirements:**
- **Performant:** Handle high-throughput metric writes
- **Rust-native:** First-class Rust integration
- **Embeddable:** No external dependencies
- **Queryable:** SQL or SQL-like query interface
- **Durable:** ACID guarantees for critical data

**Recommended: DuckDB (Embedded)**
- Blazingly fast analytical queries
- Full SQL support
- Excellent Rust bindings
- Perfect for time-series and metrics
- Can export to Parquet for long-term storage

**Alternative: Sled (Rust-native)**
- Pure Rust embedded database
- High performance key-value store
- Good for state management
- Less mature than DuckDB for analytics

**Hybrid Approach:**
```rust
// DuckDB for metrics and analytics
evento_metrics_db: DuckDB
  - test_runs table
  - metrics table (time-series)
  - insights table

// Sled for operational state
evento_state_db: Sled
  - active_jobs queue
  - worker_registry
  - distributed_locks
  - test_context (key-value)
```

### Data Schema

```sql
-- Test definitions
CREATE TABLE test_scripts (
  id UUID PRIMARY KEY,
  name VARCHAR,
  version INT,
  yaml_content TEXT,
  created_at TIMESTAMP,
  updated_at TIMESTAMP
);

-- Custom functions/plugins
CREATE TABLE plugins (
  id UUID PRIMARY KEY,
  name VARCHAR,
  language VARCHAR, -- rust, python, wasm
  source_code TEXT,
  compiled_binary BLOB,
  version VARCHAR,
  registered_at TIMESTAMP
);

-- Test execution state
CREATE TABLE test_runs (
  id UUID PRIMARY KEY,
  test_script_id UUID,
  status VARCHAR, -- queued, running, completed, failed
  started_at TIMESTAMP,
  completed_at TIMESTAMP,
  config JSONB,
  results JSONB
);

-- Metrics (time-series)
CREATE TABLE metrics (
  timestamp TIMESTAMP,
  test_run_id UUID,
  metric_name VARCHAR,
  metric_value DOUBLE,
  dimensions JSONB,
  worker_id VARCHAR
);

-- AI Insights
CREATE TABLE insights (
  id UUID PRIMARY KEY,
  test_run_id UUID,
  insight_type VARCHAR,
  severity VARCHAR,
  description TEXT,
  suggested_actions JSONB,
  code_locations JSONB,
  created_at TIMESTAMP
);
```

---

## Chaos Engineering Integration

### AI-Driven Chaos Tests

**Concept:** AI agents generate chaos scenarios to validate system resilience.

```yaml
test: chaos_order_system
chaos:
  enabled: true
  scenarios:
    - type: network_latency
      target: inventory_service
      latency: 
        min: 100ms
        max: 2000ms
      probability: 0.3  # 30% of requests
      
    - type: service_failure
      target: payment_gateway
      failure_rate: 0.05  # 5% failure rate
      failure_types:
        - timeout
        - connection_refused
        - 500_error
        
    - type: message_loss
      target: kafka
      topic: orders.created
      loss_rate: 0.02  # 2% message loss
      
    - type: database_slowdown
      target: orders_db
      queries_affected: "SELECT * FROM orders WHERE*"
      slowdown_factor: 5x

scenario:
  - name: normal_order_flow
    # ... regular test steps
    
  observe:
    - metric: order_completion_rate
      expect_min: 0.93  # Should handle 93%+ despite chaos
      
    - metric: retry_success_rate
      expect_min: 0.95  # Retries should recover most failures
      
    - metric: data_consistency
      expect: 1.0  # No data corruption despite failures
```

**AI Agent Use Cases:**
1. **Hypothesis Testing:** "What if payment gateway has 10% error rate?"
2. **Boundary Discovery:** "At what latency does the system start failing?"
3. **Resilience Validation:** "Does the retry logic actually work?"

---

## Cloud-Native Focus

### Kubernetes Integration (Optional, Not Required)

```yaml
# Deploy evento in Kubernetes
apiVersion: apps/v1
kind: Deployment
metadata:
  name: evento-manager
spec:
  replicas: 1
  template:
    spec:
      containers:
      - name: evento-manager
        image: evento/manager:latest
        
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: evento-workers
spec:
  replicas: 10  # Auto-scale based on queue depth
  template:
    spec:
      containers:
      - name: evento-worker
        image: evento/worker:latest
```

### Testing-Tool-as-a-Service

**Core Idea:** evento is accessible as an API for AI agents, regardless of deployment.

```bash
# AI Agent calls evento service
curl -X POST https://evento.example.com/api/v1/tests/run \
  -H "Authorization: Bearer $AI_AGENT_TOKEN" \
  -H "Content-Type: application/yaml" \
  --data-binary @test_definition.yaml

# Get results
curl https://evento.example.com/api/v1/tests/runs/{run_id}/results \
  -H "Authorization: Bearer $AI_AGENT_TOKEN"
```

**Deployment Options:**
- **Self-hosted:** Run evento in your own infrastructure
- **Cloud service:** Managed evento service (future SaaS offering)
- **Embedded:** evento as a library in AI agent runtime

---

## Core Pitch: AI-First Testing Tool

### Why AI-First Matters

**Traditional testing tools were designed for humans:**
- Visual dashboards
- Click-through UIs
- Human-readable reports

**AI agents need:**
- **Machine-readable input/output** (YAML in, JSON out)
- **Semantic understanding** (not just "failed" but "why and how to fix")
- **Programmatic control** (API-first, not UI-first)
- **Structured insights** (actionable suggestions tied to code locations)

### Agent Workflow

```
1. AI Agent reads user story
   ↓
2. Agent generates code
   ↓
3. Agent generates evento test (functional, integration, performance)
   ↓
4. Agent calls evento API to run test
   ↓
5. evento executes test, returns structured results
   ↓
6. Agent parses results, identifies issues
   ↓
7. Agent modifies code OR test based on insights
   ↓
8. Loop until quality gates pass
```

### Example: AI Agent Using evento

```python
# AI Agent workflow
agent = SoftwareDevAgent()

# Step 1: Generate code from user story
code = agent.generate_code(user_story)

# Step 2: Generate evento test
test_yaml = agent.generate_evento_test(code, user_story)

# Step 3: Run test
result = evento_client.run_test(test_yaml)

# Step 4: Parse results
if result.status == "failed":
    # Agent gets structured insights
    for failure in result.failures:
        print(f"Issue: {failure.description}")
        print(f"Location: {failure.code_locations}")
        print(f"Suggestions: {failure.suggested_fixes}")
    
    # Agent decides: fix code or fix test
    if agent.is_code_issue(failure):
        code = agent.fix_code(code, failure)
    else:
        test_yaml = agent.fix_test(test_yaml, failure)
    
    # Retry
    result = evento_client.run_test(test_yaml)
```

---

## Phased Approach

### Phase 1: Core Foundation (MVP - 3 months)
- YAML DSL parser
- Manager-worker architecture (single machine)
- 3 protocols: HTTP, Kafka, PostgreSQL
- Basic data generation (faker)
- Metrics collection (Prometheus export)
- Embedded database (DuckDB)

### Phase 2: AI Integration (3 months)
- Structured result format
- Insight engine
- Suggestion generation
- API for programmatic access
- AI agent example integrations

### Phase 3: Scale and Extend (3 months)
- Distributed workers
- More protocols (gRPC, JMS)
- Plugin system
- Advanced chaos engineering
- Built-in visualization

### Phase 4: Enterprise & Polish (3 months)
- Team collaboration features
- Scheduling and CI/CD
- Advanced observability
- Enterprise protocols
- Documentation and examples

---

## Open Questions

1. **What AI agent frameworks should we target first?** (AutoGPT, LangChain, custom agents?)
2. **How do we balance YAML simplicity with power?** (When does complexity warrant moving to code?)
3. **What's the plugin distribution model?** (Package manager? Registry? Git repos?)
4. **How deep should the "insight engine" go?** (Statistical analysis? ML-based pattern detection?)
5. **What's the right abstraction level for protocols?** (High-level "send message" vs low-level control?)

---

## Success Criteria

### For AI Agents
- Agent can generate valid evento test from code analysis
- Agent can parse results and identify root causes
- Agent can iteratively improve software based on evento feedback
- 90% of agent-generated tests run without human intervention

### For Human Users
- Developer can write a basic test in < 10 minutes
- Complex multi-protocol test is possible without external code
- Test results are immediately actionable
- Performance is 10x better than Python-based tools

### For Enterprise Adoption
- Supports 10+ protocols out of box
- Integrates with existing CI/CD pipelines
- Provides compliance and audit trails
- Scales to 10,000+ req/sec per node
