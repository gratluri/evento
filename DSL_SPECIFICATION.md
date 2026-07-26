Network Working Group
Request for Comments: XXXX
Category: Standards Track

# Title: The evento Domain-Specific Language (eDSL) Specification

## Abstract

The evento Domain-Specific Language (eDSL) provides a declarative, extensible, and protocol-agnostic framework for defining test scenarios, load simulations, and operational validations for distributed enterprise systems. Designed for both human operators and autonomous AI agents, eDSL combines the simplicity of YAML-based configuration with the power of embedded scripting, rich data generation, and context-aware flow control. This document specifies the syntax, semantics, and execution model of eDSL.

## 1. Introduction

Enterprise software systems increasingly rely on complex, asynchronous, and multi-protocol interactions spanning monolithic and microservice architectures. Traditional testing tools often restrict users to specific protocols (e.g., HTTP) and require procedural scripting for data generation and state management. The `evento` project addresses these limitations by offering a unified integration testing platform where tests are modeled as directed graphs of stateful steps.

The evento DSL (eDSL) is the configuration language used to define these tests. Its primary goals are:
1. **Protocol Abstraction:** To provide a single vocabulary for interacting with HTTP, Kafka, gRPC, databases, and custom enterprise protocols.
2. **AI-First Design:** To enable autonomous agents to easily parse, generate, and reason about tests and their results.
3. **Data Integrity:** To support relational, stateful data generation combining synthetic (faked) data with real production samples.
4. **Extensibility:** To seamlessly integrate with compiled (Rust) or interpreted (Python, WebAssembly) scripts when declarative boundaries are reached.

### 1.1 Requirements Language

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119.

### 1.2 Terminology and Core Concepts

* **Test:** The top-level definition of a validation suite, encompassing configurations, data sources, and the scenario graph.
* **Scenario:** A collection of steps (nodes) defining the execution flow.
* **Step:** A single unit of work within a scenario (e.g., an HTTP request, a Kafka message, a database query, or a script execution). Each step MUST have a `name` and MAY include a `description` for documentation purposes.
* **Manager:** The central orchestrator node responsible for parsing eDSL, managing state, and distributing work.
* **Worker:** A distributed execution node that runs steps assigned by the Manager.
* **Context:** A localized state dictionary containing extracted variables, test data, and outputs from previous steps, securely passed between the Manager and Workers.
* **AI Agent:** A machine client utilizing the platform to autogenerate tests, analyze failure semantics, and perform iterative system improvements.

### 1.3 Scope of the Specification

This specification covers the syntax of eDSL files, the parsing rules for the core compiler, the runtime behavior of the control flow graph, and the built-in protocol and validation semantics. It does not dictate the implementation details of the underlying test executor or database schemas used by `evento`, except where they directly impact the execution of eDSL constructs.

---

## 2. DSL Document Structure

The eDSL uses YAML 1.2 syntax as its primary serialization format due to its readability and widespread adoption. JSON is technically compatible but NOT RECOMMENDED for direct human authorship. 

### 2.1 File Format

All eDSL documents MUST be valid YAML. Documents SHOULD use the `.yaml` or `.yml` extension. Parsing behavior MUST conform strictly to the YAML 1.2 specification, supporting standard YAML features such as anchors (`&`) and aliases (`*`).

### 2.2 Top-Level Elements

An eDSL document is structured around several distinct top-level keys. A complete test definition MUST contain either a `test` identifier and a `scenario` definition, or an `imports` instruction that pulls in a scenario.

*   `test` (string, REQUIRED): The unique identifier for the test suite. It MUST follow a dot-separated namespace convention (e.g., `sales.buyflow.add-a-line`). The execution engine uses this to auto-derive a hierarchical namespace (e.g., `sales/buyflow/add-a-line`) for organizing results and metrics.
*   `description` (string, OPTIONAL): A human-readable summary of the test.
*   `imports` (list of strings, OPTIONAL): References to external modules, subflows, or scripts.
*   `config` (object, OPTIONAL): Execution configuration (see Section 3).
*   `data_sources` (list of objects, OPTIONAL): External data definitions (see Section 4).
*   `scenario` (object or list, REQUIRED): The execution graph containing test steps (see Section 5).
*   `validation` (object, OPTIONAL): Global business rule assertions evaluated at the end of the scenario.
*   `outputs` (list of objects, OPTIONAL): Result formatting and export destinations.

### 2.3 Metadata and Annotations

Metadata provides contextual information without affecting the execution semantics. 

*   `author`: The human or agent creator.
*   `created`: An ISO-8601 timestamp.
*   `tags`: A list of strings used for filtering test executions.

Implementations MUST ignore unrecognized top-level keys, allowing for custom tooling extensions without breaking test execution.

---

## 3. Test Configuration

The `config` block dictates how the underlying execution engine (the Manager) orchestrates the scenario.

### 3.1 Execution Modes

The platform supports multiple workload models, specified by the `mode` parameter inside the config.

*   `realtime` (default): Executes the scenario immediately. This is standard for functional and load testing.
*   `replay`: Executes based on a captured log of traffic. In this mode, the `source` parameter MUST point to a valid traffic log file, and an optional `speed` multiplier MAY be provided.
*   `scheduled`: Defers execution to a CRON scheduler. When used, a `cron` expression MUST be provided.

### 3.4 Mock Resolution Strategy

When steps define `mock` blocks (see Section 7.6), the `mock_strategy` config field determines globally how the engine resolves step execution:

*   `mock_strategy` (enum, default: `auto`): Controls mock activation.
    *   `auto`: Steps with a `mock` block use their mock response. Steps without `mock` call real services.
    *   `disabled`: All `mock` blocks are ignored. Every step executes against real services.
    *   `required`: Every step MUST define a `mock` block. Steps without one fail immediately. Used for fully offline testing.

```yaml
config:
  mock_strategy: auto
  timeout: 5m
```

> **Future Extension:** A `record` strategy is planned for a future release, which would execute against real services while capturing request/response pairs for automatic mock generation.

### 3.2 Concurrency and Virtual Users

For load and stress testing, the framework spawns isolated execution contexts known as Virtual Users (VUs).

*   `virtual_users` (integer, default: 1): The maximum number of concurrent executions.
*   `ramp_up` (object or duration string): The strategy for spinning up VUs to the target number.
    *   If specified as a string (e.g., `"30s"`), a default `linear` strategy is used over that duration.
    *   If specified as an object, it supports different pluggable strategies: `linear`, `step`, `normal_distribution`, and custom plugins.
*   `duration` (duration string): The total time the test will run. If omitted, the scenario executes exactly once per VU.

```yaml
config:
  duration: 5m
  virtual_users: 100
  ramp_up:
    strategy: step
    duration: 30s
    steps: 5
```

### 3.3 Timeouts and Limits

Global safety rails MUST be defined to prevent runaway processes.

*   `timeout` (duration string, default: "1m"): The maximum allowed time for the entire scenario to complete. If a scenario exceeds this duration, the Manager MUST terminate it and mark the execution as a failure (Timeout).
*   `step_timeout` (duration string, default: "10s"): The default timeout for individual steps if not explicitly defined within the step itself.

## 4. Data Sources and Generation

Robust enterprise testing requires realistic data distributions, spanning synthetic mock data and genuine subsets of production data. 

### 4.1 The Faker Integration (`$faker.*`)

eDSL integrates deeply with a faker library for generating synthetic data. In any templated context string, expressions starting with a `$` variable reference MUST be evaluated by the execution engine at runtime by calling appropriate functions on the respective data source or utility.

*   **Faker Methods:** Supports standard faker categories such as `uuid`, `name`, `address`, `internet`, and `commerce`. (e.g., `${$faker.username()}`).
*   **Random Utility:** For explicit ranges and selections (e.g., `${$random.decimal(10, 1000)}`, `${$cycle(['USD', 'EUR', 'GBP'])}`).

### 4.2 Database and External File Feeds

eDSL supports extracting data feeds directly from databases, CSVs, and JSON files prior to execution. These feeds are globally accessible across all VUs.

```yaml
data_sources:
  - name: customers
    source: database
    connection: postgres://prod-readonly/main
    query: "SELECT id, email, tier FROM customers WHERE active = true"
    sampling: random # Can be 'sequential', 'random', or 'shuffle'
    cache: true
```

*   `source` (enum): Specifies the feed type (`database`, `csv`, `json`).
*   `connection`: URI string pointing to the data source.
*   `query` or `path`: The query string or file path to fetch the data.
*   `sampling`: Determines how records are handed out to VUs.

### 4.3 Referential Integrity across Data Sources

To simulate accurate transactional behavior, synthetic generated data MAY reference external data sources to maintain referential integrity.

```yaml
data_sources:
  - name: fake_orders
    generator: faker
    fields:
      orderId: uuid
      customerEmail: from_source(customers.email)
      amount: decimal(10, 1000)
```
In the example above, `fake_orders.customerEmail` is constrained to draw only from the `email` column fetched by the `customers` data source.

---

## 5. Execution Graph and Control Flow

An eDSL test is modeled not merely as an array of steps, but as a directed graph. The `scenario` element encapsulates this graph. 

### 5.1 Linear Execution

The most basic scenario is an array of steps. Unless directed otherwise, the Manager MUST execute them sequentially.

```yaml
scenario:
  - name: step_one
    protocol: https
    # ...
  - name: step_two
    protocol: https
    # ...
```

### 5.2 Conditional Branching

Steps MAY use a `validate` block alongside `on_success` and `on_failure` directives to control the path of execution.

```yaml
- name: validate_order
  validate:
    - context.order.amount > 0
  on_success: charge_payment
  on_failure: reject_order
```

If `validate` evaluates to true, the step specified by `on_success` is queued. If false, `on_failure` is queued. 

### 5.3 Parallel Execution

The `parallel` block instructs the Manager to execute multiple distinct sub-graphs or steps concurrently.

```yaml
parallel:
  - name: track_analytics
    protocol: kafka
    # ...
  - name: notify_warehouse
    protocol: grpc
    # ...
```
The execution engine MUST wait for all branches of a `parallel` block to complete before advancing past the block, unless an explicit early termination condition is met.

### 5.4 Loops and Iterations

Loops allow repeated execution of steps, either numerically or over collections.

*   `count`: Execute exactly *N* times.
*   `over` and `from`: Iterate over a collection (e.g., an array from a `data_source` or `context`).

```yaml
- name: add_to_cart
  loop:
    count: "${$random.int(1, 5)}"
    over: selected_product
    from: "${$context.products}"
  do:
    protocol: https
    endpoint: /api/cart/items
    body:
      productId: "${$selected_product.id}"
```

### 5.5 Synchronization, Wait States, and Fire-and-Forget

To enforce convergence points in complex graphs, eDSL uses the `wait_for` primitive.

*   `wait_for` (list of step names): Execution MUST block until the listed nodes have successfully completed.
*   `timeout`: Bound the wait duration.
*   `on_timeout` / `on_success`: Branching directives post-wait.

Conversely, steps can be designated as **fire-and-forget** by specifying `async: true`. In this case, the execution graph proceeds immediately to the next steps without blocking on the completion of the asynchronous step.

```yaml
- name: send_telemetry
  protocol: kafka
  async: true
  # ...
```

### 5.6 Retries and Backoff Strategies

To handle transient failures inherent to distributed systems, steps MAY define a `retry` block.

```yaml
retry:
  max_attempts: 3
  backoff: exponential # Can be 'linear', 'exponential', or 'constant'
  delay: 2s # Initial delay
```

## 6. State and Context Management

Testing complex systems often requires maintaining state across multiple steps (e.g., extracting an authentication token from a login response and injecting it into a subsequent request).

### 6.1 The `context` Object

The `context` is a dynamic dictionary available throughout the lifecycle of a scenario execution. Each Virtual User maintains its own isolated context.

Variables can be injected into the context via:
*   Initial data source mapping.
*   Data extraction (`extract`) blocks in steps.
*   Explicit variable declarations using `let` or `set`.

### 6.2 Data Extraction and Variable Assignment

Steps MAY include an `extract` block to parse and store values from a protocol response.

```yaml
- name: login
  protocol: https
  endpoint: /api/login
  extract:
    authToken: response.body.token
    userId: response.body.user.id
```

The syntax for the right-hand side of an assignment SHOULD support dot notation for object traversal (e.g., `response.body.token`) and bracket notation for array indexing (e.g., `response.items[0].id`).

### 6.3 Type System and Expression Interpolation

eDSL is dynamically typed. The context supports primitive types (strings, integers, booleans, floats) and complex types (objects, arrays).

To inject variables into test configuration, eDSL uses interpolation with `${$variable}`.

```yaml
headers:
  Authorization: "Bearer ${$context.authToken}"
```

Inside `${$...}`, basic expressions and method calls are supported (e.g., `${$context.amount * 1.2}`, `${$context.items.length}`).

---

## 7. Protocol Integrations

One of the defining features of eDSL is its multi-protocol abstraction. The `protocol` field on a step determines the specific driver used by the Worker to execute the step.

### 7.1 HTTP/HTTPS

The foundational protocol for REST and web interactions.

*   `endpoint`: The URL path.
*   `method`: GET, POST, PUT, DELETE, PATCH (Default: GET).
*   `headers`: Dictionary of HTTP headers.
*   `body`: The payload (JSON, XML, form-data).
*   `auth`: Built-in support for OAuth2, Basic Auth, mTLS.

### 7.2 Kafka (Publishing and Observing)

For event-driven architectures, eDSL supports both producing and consuming messages.

**Publishing (Producer):**
```yaml
protocol: kafka
topic: orders.new
message:
  format: avro
  schema: order_schema.avsc
  template: |
    { "orderId": "${$context.orderId}" }
```

**Observing (Consumer/CDC):**
When `mode: observe` is set, the step acts as a consumer that scans the topic for a message matching a specified expectation within a timeout window.

```yaml
protocol: kafka
topic: orders.cdc
mode: observe
expect:
  message.operation: INSERT
  message.data.id: "${$context.orderId}"
within: 5s
```

### 7.3 gRPC

Provides support for Protocol Buffers and RPC streaming.

*   `service`: The fully qualified gRPC service method (e.g., `inventory.InventoryService/CheckStock`).
*   `message`: The protobuf payload defined as a dictionary or JSON string mapping.
*   `metadata`: Optional gRPC metadata (headers).

### 7.4 Databases (SQL/NoSQL)

Allows for direct verification or manipulation of database state.

```yaml
protocol: database
connection: postgres://orders-db
query: "SELECT status FROM orders WHERE id = '${$context.orderId}'"
```
For databases, the result set is typically made available under `response.rows` or `result.rows` for extraction or validation.

### 7.5 Generic and Custom Protocols

The platform MUST provide an extensible plugin interface to support enterprise-specific protocols such as JMS, SOAP, IBM MQ, or proprietary binary formats (see Section 9.3).

### 7.6 Step-Level Mocking

In agentic automation workflows, an AI agent MAY author tests for systems that do not yet exist — services that are unimplemented, undeployed, or under active parallel development. eDSL addresses this with first-class **step-level mocking**: each step MAY define a `mock` block that provides a synthetic response when the target service is unavailable or when the test is running in mock mode (§3.4).

Mocks are defined **per-step**, not globally. Each test scenario dictates exactly how each interaction should behave, providing explicit control over the simulated responses. This design enables AI agents to generate complete, self-contained test files that can be validated and dry-run without any live infrastructure.

#### 7.6.1 Mock Definition

A step's `mock` block defines the synthetic response the engine returns instead of (or as a fallback for) calling the real service. The mock is co-located with the step that defines the target protocol, endpoint, and request.

```yaml
- name: create_order
  protocol: https
  endpoint: /api/orders
  method: POST
  body:
    customerId: "${$faker.uuid()}"
    amount: 99.99
  mock:
    response:
      status: 201
      headers:
        Content-Type: application/json
      body:
        orderId: "${$faker.uuid()}"
        status: "created"
        amount: "${$request.body.amount}"
  extract:
    orderId: response.body.orderId
  validate:
    - "response.status == 201"
```

When a `mock` is present and the mock strategy is `auto` or `required` (§3.4), the engine skips the real protocol call and returns the mock response. The step's `extract`, `validate`, and `track_metric` blocks operate on the mock response exactly as they would on a real response.

Key `mock` fields:

*   `response` (object): A single response definition (see §7.6.2).
*   `responses` (list of objects): An ordered sequence of responses for stateful simulation (see §7.6.3).
*   `on_exhausted` (enum): Behavior when `responses` list is exhausted: `repeat_last` (default), `cycle`, `error`.
*   `behavior` (object): Advanced failure injection controls (see §7.6.4).
*   `request_schema` (object, OPTIONAL): JSON Schema or inline schema defining the expected request contract (see §7.6.7).
*   `response_schema` (object, OPTIONAL): JSON Schema or inline schema defining the expected response contract (see §7.6.7).

#### 7.6.2 Dynamic Response Generation

Mock responses support the full eDSL expression engine. In addition to `$faker`, `$context`, and `$random`, mock responses have access to:

*   **`$request.*`** — The incoming request being mocked. Allows echo-back patterns (e.g., `${$request.body.amount}`).
*   **`$mock.call_count`** — The number of times this step's mock has been invoked within the current VU. Enables count-dependent behavior.

```yaml
- name: process_payment
  protocol: https
  endpoint: /api/payments
  method: POST
  body:
    amount: 150.00
    currency: "USD"
  mock:
    response:
      status: 200
      body:
        transactionId: "${$faker.uuid()}"
        amount: "${$request.body.amount}"
        currency: "${$request.body.currency}"
        status: "approved"
        processedAt: "${$faker.iso8601()}"
      latency: "${$random.int(50, 200)}ms"
```

The `latency` field introduces simulated processing time before the mock response is returned, allowing realistic timing in the test flow.

#### 7.6.3 Stateful Mock Sequences

For simulating realistic multi-call interactions (e.g., polling for status changes, paginated APIs), a step's mock MAY define an ordered `responses` list. Each invocation of the step consumes the next response in the sequence.

```yaml
- name: check_job_status
  protocol: https
  endpoint: /api/jobs/123/status
  method: GET
  mock:
    responses:
      - status: 200
        body: { status: "processing", progress: 25 }
      - status: 200
        body: { status: "processing", progress: 75 }
      - status: 200
        body: { status: "completed", result: "success" }
    on_exhausted: repeat_last
```

The `on_exhausted` field determines behavior after all responses are consumed:

*   `repeat_last` (default): The final response repeats indefinitely.
*   `cycle`: The sequence restarts from the first response.
*   `error`: The step fails with a mock exhaustion error.

#### 7.6.4 Failure Injection

Mock `behavior` blocks simulate infrastructure failures for resilience testing. These controls are probabilistic and apply per-invocation:

```yaml
- name: call_inventory_service
  protocol: https
  endpoint: /api/inventory/check
  method: POST
  mock:
    response:
      status: 200
      body:
        available: true
        quantity: "${$random.int(1, 100)}"
    behavior:
      error_rate: 0.3
      error_response:
        status: 503
        body: { error: "Service Unavailable" }
      latency:
        distribution: normal
        mean: 200ms
        stddev: 50ms
      timeout_rate: 0.05
```

*   `error_rate` (float, 0.0–1.0): Fraction of calls that return the `error_response` instead of the normal response.
*   `error_response` (object): The response returned on injected errors.
*   `latency` (object or string): Either a fixed duration string or a distribution object with `distribution` (normal, uniform), `mean`, `stddev`, `min`, `max`.
*   `timeout_rate` (float, 0.0–1.0): Fraction of calls that simulate a complete timeout (no response).

#### 7.6.5 Protocol-Specific Mocks

**HTTP/HTTPS Mocks** return `status`, `headers`, and `body`:

```yaml
mock:
  response:
    status: 200
    headers:
      Content-Type: application/json
      X-Request-ID: "${$faker.uuid()}"
    body:
      data: []
```

**Kafka Mocks** return synthetic messages for `observe` mode steps:

```yaml
- name: observe_order_event
  protocol: kafka
  topic: orders.cdc
  mode: observe
  expect:
    message.operation: INSERT
  within: 5s
  mock:
    response:
      message:
        operation: INSERT
        after:
          id: "${$context.orderId}"
          status: "created"
      delay: 500ms
```

The `delay` field simulates the time between the triggering event and the CDC observation.

**Database Mocks** return synthetic result sets:

```yaml
- name: query_order_status
  protocol: database
  connection: postgres://orders-db
  query: "SELECT status FROM orders WHERE id = '${$context.orderId}'"
  mock:
    response:
      rows:
        - status: "completed"
          amount: "${$random.decimal(10, 1000)}"
          customer_id: "${$context.customerId}"
```

> **Future Extension:** gRPC streaming mocks (simulating multiple messages over time) are planned for a future release. The current specification covers unary gRPC mocks only.

#### 7.6.6 Request and Response Contracts

In agentic development workflows, the AI agent typically knows the expected interface contracts (input/output definitions) before the service is implemented. eDSL supports embedding these contracts alongside mock definitions to ensure that:

1.  **Mock responses conform** to the expected output contract — catching stale mocks.
2.  **Requests conform** to the expected input contract — validating test correctness.
3.  **Real service responses** (when mocks are disabled) still match the contract — detecting API drift.

Contracts are defined using `request_schema` and `response_schema` fields within the `mock` block. These accept inline JSON Schema-compatible definitions:

```yaml
- name: create_order
  protocol: https
  endpoint: /api/orders
  method: POST
  body:
    customerId: "c-123"
    amount: 99.99
  mock:
    request_schema:
      type: object
      properties:
        customerId:
          type: string
        amount:
          type: number
          minimum: 0
      required: [customerId, amount]
    response_schema:
      type: object
      properties:
        orderId:
          type: string
          format: uuid
        status:
          type: string
          enum: [created, pending, failed]
        amount:
          type: number
      required: [orderId, status]
    response:
      status: 201
      body:
        orderId: "${$faker.uuid()}"
        status: "created"
        amount: "${$request.body.amount}"
```

The engine MUST validate mock responses against `response_schema` at parse time (static validation) and MAY validate real responses at runtime when `mock_strategy` is `disabled`.


## 8. Validation and Assertions

Assertions verify that the system under test behaves correctly. eDSL allows assertions at the individual step level (micro) and at the global scenario level (macro).

### 8.1 Protocol-Level Validation

Every step MAY contain a `validate` list containing boolean expressions.

```yaml
validate:
  - response.status == 200
  - response.time < 500ms
  - header['X-Idempotency-Key'] exists
```

If any expression in the `validate` list evaluates to `false`, the step is marked as failed.

### 8.2 Business Rule Validation

Beyond technical validations, eDSL encourages testing business logic by parsing response bodies.

```yaml
validate:
  - json.parse(response.body).status == "approved"
  - json.parse(response.body).amount == context.expectedAmount
```

Global business rules MAY be placed in the top-level `validation` block. These are evaluated after the entire scenario completes.

```yaml
validation:
  business_rules:
    - name: order_completion_rate
      threshold: 0.95
      actual: "${$metrics.completed_orders / metrics.total_orders}"
```

### 8.3 Temporal Assertions

For asynchronous operations (like CDC observations), `within` modifiers enforce timing constraints. The timer for a `within` assertion needs a defined start point. By default, the timer starts when the step begins execution. For complex scenarios, it is highly recommended to explicitly link the timer to a parent event using the `since` parameter.

```yaml
expect:
  message.operation: INSERT
within: 5s # Starts when this step executes

# OR explicitly linked to a previous step:
within:
  duration: 5s
  since: place_order # Timer starts precisely when 'place_order' completes
```
If the expectation is not met before the `within` timer expires, the assertion fails with a timeout error.

---

## 9. Extensibility and Scripting

Declarative DSLs inevitably hit boundaries where complex transformations or proprietary logic are required. eDSL escapes these boundaries via a tiered scripting and extensibility model.

### 9.1 Inline Scripts (Rust, Python, WebAssembly)

For complex decision-making or data manipulation, eDSL allows inline script execution or calls to external script files. 

```yaml
- name: complex_decision
  script: |
    if context.user.tier == "premium":
        return "kafka", "premium-topic"
    else:
        return "https", "/standard-endpoint"
  runtime: python
```

The runtime environment (e.g., Python via embedded interpreters, or Rust via compiled plugins) MUST execute the script within a sandboxed context, passing the current `context` object as a local variable.

### 9.2 Custom Functions and Transformers

Reusable logic can be defined globally and mapped to eDSL identifiers.

```yaml
functions:
  - name: calculate_tax
    language: rust
    source: ./functions/tax.rs

scenario:
  - name: process_order
    transform:
      - input: "${$context.order.amount}"
        function: calculate_tax
        params:
          region: "${$context.customer.region}"
        output_to: tax_amount
```

### 9.3 The Plugin Architecture

To add entirely new protocols or metrics exporters, developers CAN author compiled plugins conforming to the `EventoPlugin` Rust trait.

```rust
pub trait EventoPlugin {
    fn name(&self) -> &str;
    fn supported_protocols(&self) -> Vec<Protocol>;
    fn execute_request(&self, request: Request) -> Result<Response>;
}
```

Plugins are loaded dynamically at startup and registered within the execution engine.

## 10. Observability and Metrics

A core tenet of eDSL is production-grade observability baked in from day one. Tests are not pass/fail binaries; they are continuous telemetry generators.

### 10.1 Built-in Metrics

The execution engine MUST automatically track standard technical telemetry without explicit configuration:
*   `latency`: p50, p90, p95, p99 per step and scenario.
*   `throughput`: requests/messages per second.
*   `error_rate`: percentage of failed steps.
*   `protocol_specific`: e.g., Kafka consumer lag, DB connection pool usage.

### 10.2 Custom Business Metrics (`track_metric`)

eDSL allows tracking arbitrary business values using the `track_metric` block inside steps.

```yaml
track_metric:
  name: payment_approval_rate
  value: json.parse(response.body).status == "approved"
  dimensions:
    payment_method: request.body.method
    customer_tier: context.customer.tier
```

This enables the aggregation of business success rates sliced by dynamic dimensions.

### 10.3 Metric Exporting

All collected metrics (built-in and custom) can be forwarded to external systems via the `outputs` top-level array.

```yaml
outputs:
  - format: prometheus
    endpoint: http://prometheus:9090
  - format: datadog
    api_key: ${DATADOG_API_KEY}
```

---

## 11. Modularity and Composition

To prevent duplication and encourage reusable testing components across teams, eDSL features module imports and inheritance.

### 11.1 Imports and Subflows

A module is a standalone YAML file defining a `module` block instead of a full scenario. Modules define explicit `inputs` and `outputs`.

```yaml
# modules/auth_flow.yaml
module: authenticate_user
inputs: [username, password]
outputs: [authToken, userId]
scenario:
  # ... login steps that extract authToken
```

Main test files import and invoke these modules using `use_module`.

```yaml
imports:
  - modules/auth_flow.yaml

scenario:
  - name: authenticate
    use_module: authenticate_user
    with:
      username: "${$faker.username()}"
      password: "test_password_123"
    outputs_to: auth_context
```

### 11.2 Inheritance (`extends`, `base`)

eDSL supports configuration inheritance allowing teams to define baseline environments (e.g., `staging-base.yaml`) and override specific values.

```yaml
base: api_test_base.yaml
test: specific_order_test
extends: base

config:
  timeout: 30s # Overrides the base configuration timeout
```

## 12. AI Agent Integration

eDSL is explicitly designed to be written, analyzed, and debugged by autonomous AI software engineers.

### 12.1 Machine-Readable Outputs and Formats

While human operators typically view console output or HTML reports, the `outputs` directive supports AI-friendly JSON schema exports.

```yaml
outputs:
  - format: ai_insights
    file: results/insights_${$timestamp}.json
```

This output file MUST contain a structured breakdown of the execution:
1.  **Summary statistics** (Passed, failed, skipped steps).
2.  **Context snapshots** (Variable states before and after failures).

### 12.2 Structure of Failure Analysis and Insights

The platform's Insight Engine provides detailed failure explanations and proposed code locations.

A JSON response from the test execution looks like:
```json
{
  "failures": [
    {
      "step": "observe_cdc",
      "reason": "timeout_exceeded",
      "expected": "CDC event within 5s",
      "suggested_fixes": [
        "Check CDC connector configuration",
        "Verify database CDC is enabled",
        "Increase timeout threshold"
      ]
    }
  ]
}
```
This structured format allows agents to immediately digest the feedback loop, modify either the system under test or the eDSL file, and re-execute.

---

## 13. Security Considerations

Due to the powerful nature of eDSL (capable of arbitrary code execution via scripts and manipulating production-level connections), stringent security measures MUST be adhered to.

### 13.1 Secrets Management

eDSL files MUST NOT contain hardcoded credentials. The engine MUST evaluate environment variables transparently within context strings using the standard `${ENV_VAR}` syntax.

```yaml
headers:
  Authorization: "Bearer ${API_ACCESS_TOKEN}"
```

### 13.2 Injection and Execution Risks

When evaluating inline scripts (Section 9.1), the execution environment MUST heavily sandbox the scripts:
1.  Disallow arbitrary filesystem access.
2.  Limit memory and execution time.
3.  WebAssembly (Wasm) is the RECOMMENDED sandboxing environment for custom logic.

---

## 14. Examples

This section provides 50 complete, runnable eDSL examples progressing from trivial single-step tests to complex, multi-feature integration scenarios. Each example corresponds to a validated parser test case and can be used directly as a template or reference implementation.

---

### Category 1: Basic Scenarios (Examples 1–5)

*Single step, minimal configuration. The simplest possible eDSL documents.*

#### Example 1 — Minimal Test Plan

The absolute minimum: a test identifier and a single scenario step.

```yaml
test: minimal
scenario:
  - name: noop
    protocol: https
    endpoint: /health
```

#### Example 2 — `name` Alias for `test`

The `name` key is accepted as an alias for `test` for backward compatibility.

```yaml
name: aliased_test
scenario:
  - name: step1
    protocol: https
    endpoint: /
```

#### Example 3 — Single GET Request

An explicit HTTP GET with a named method.

```yaml
test: simple_get
scenario:
  - name: fetch_users
    protocol: https
    endpoint: /api/v1/users
    method: GET
```

#### Example 4 — Single POST with Body

A step with a JSON payload submitted via POST.

```yaml
test: post_test
scenario:
  - name: create_user
    protocol: https
    endpoint: /api/users
    method: POST
    body:
      username: "testuser"
      email: "test@example.com"
```

#### Example 5 — Step with Description

Steps MAY carry a human-readable `description` for documentation purposes (§1.2).

```yaml
test: described_steps
scenario:
  - name: health_check
    description: "Verifies the API is alive and responding"
    protocol: https
    endpoint: /health
    method: GET
```

---

### Category 2: Multi-Step Linear Flows (Examples 6–10)

*Sequential step execution, headers, complex bodies, and template interpolation.*

#### Example 6 — Two Sequential Steps

Steps listed in an array are executed in order (§5.1).

```yaml
test: two_steps
scenario:
  - name: step_one
    protocol: https
    endpoint: /first
  - name: step_two
    protocol: https
    endpoint: /second
```

#### Example 7 — Steps with HTTP Headers

Arbitrary headers are passed as a dictionary (§7.1).

```yaml
test: headers_test
scenario:
  - name: authenticated_call
    protocol: https
    endpoint: /api/protected
    method: GET
    headers:
      Authorization: "Bearer token123"
      Accept: "application/json"
      X-Request-ID: "req-001"
```

#### Example 8 — Five-Step Pipeline

A realistic multi-step flow exercising login, profile, settings, ordering, and logout.

```yaml
test: pipeline
scenario:
  - name: login
    protocol: https
    endpoint: /api/login
    method: POST
  - name: get_profile
    protocol: https
    endpoint: /api/profile
    method: GET
  - name: update_settings
    protocol: https
    endpoint: /api/settings
    method: PUT
  - name: submit_order
    protocol: https
    endpoint: /api/orders
    method: POST
  - name: logout
    protocol: https
    endpoint: /api/logout
    method: POST
```

#### Example 9 — Step with Complex Nested Body

Deeply nested JSON payloads (objects, arrays, nested objects) are fully supported.

```yaml
test: complex_body
scenario:
  - name: create_order
    protocol: https
    endpoint: /api/orders
    method: POST
    body:
      customer:
        id: "c-123"
        name: "John Doe"
      items:
        - productId: "p-001"
          quantity: 2
          price: 29.99
        - productId: "p-002"
          quantity: 1
          price: 49.99
      shipping:
        method: "express"
        address:
          street: "123 Main St"
          city: "Springfield"
```

#### Example 10 — Interpolation Templates and Extraction

Demonstrates `$faker`, `$context` interpolation (§6.3), and variable extraction (§6.2).

```yaml
test: interpolation
scenario:
  - name: login
    protocol: https
    endpoint: /api/login
    method: POST
    body:
      username: "${$faker.username()}"
      password: "test_password_123"
    extract:
      authToken: response.body.token
  - name: get_orders
    protocol: https
    endpoint: /api/orders
    method: GET
    headers:
      Authorization: "Bearer ${$context.authToken}"
```

---

### Category 3: Configuration Variations (Examples 11–15)

*Execution modes, virtual users, ramp-up strategies, and timeouts (§3).*

#### Example 11 — Realtime Mode with Virtual Users

Standard load test: 100 VUs for 5 minutes (§3.1, §3.2).

```yaml
test: load_test
config:
  mode: realtime
  virtual_users: 100
  duration: 5m
scenario:
  - name: hit_api
    protocol: https
    endpoint: /api/stress
```

#### Example 12 — Replay Mode

Replay captured traffic at 2× speed (§3.1).

```yaml
test: replay_test
config:
  mode: replay
  source: /var/logs/traffic.log
  speed: 2.0
scenario:
  - name: replay_step
    protocol: https
    endpoint: /api/replay
```

#### Example 13 — Scheduled Mode

CRON-driven execution: every 6 hours (§3.1).

```yaml
test: scheduled_test
config:
  mode: scheduled
  cron: "0 */6 * * *"
scenario:
  - name: periodic_check
    protocol: https
    endpoint: /health
```

#### Example 14 — Simple Duration Ramp-Up

When `ramp_up` is a string, a `linear` strategy is implied over that duration (§3.2).

```yaml
test: ramp_simple
config:
  virtual_users: 50
  ramp_up: "30s"
  duration: 5m
scenario:
  - name: load_step
    protocol: https
    endpoint: /api/load
```

#### Example 15 — Structured Step Ramp-Up with Timeouts

Explicit `step` strategy with global and per-step timeouts (§3.2, §3.3).

```yaml
test: ramp_structured
config:
  virtual_users: 100
  duration: 5m
  ramp_up:
    strategy: step
    duration: 30s
    steps: 5
  timeout: 10m
  step_timeout: 15s
scenario:
  - name: load_step
    protocol: https
    endpoint: /api/load
```

---

### Category 4: Data Sources (Examples 16–20)

*Database feeds, CSV/JSON files, faker generation, and referential integrity (§4).*

#### Example 16 — Database Data Source

Fetch rows from a live database with random sampling and caching (§4.2).

```yaml
test: db_source
data_sources:
  - name: customers
    source: database
    connection: "postgres://prod-readonly/main"
    query: "SELECT id, email, tier FROM customers WHERE active = true"
    sampling: random
    cache: true
scenario:
  - name: use_data
    protocol: https
    endpoint: /api/test
```

#### Example 17 — CSV Data Source

Read records sequentially from a CSV file (§4.2).

```yaml
test: csv_source
data_sources:
  - name: products
    source: csv
    path: ./data/products.csv
    sampling: sequential
scenario:
  - name: use_products
    protocol: https
    endpoint: /api/products
```

#### Example 18 — JSON Data Source

Read records from a JSON file with shuffle sampling (§4.2).

```yaml
test: json_source
data_sources:
  - name: configs
    source: json
    path: ./data/configs.json
    sampling: shuffle
scenario:
  - name: use_configs
    protocol: https
    endpoint: /api/configs
```

#### Example 19 — Faker with Referential Integrity

Synthetic data constrained by a live data source to maintain referential integrity (§4.3).

```yaml
test: faker_source
data_sources:
  - name: customers
    source: database
    connection: "postgres://readonly/main"
    query: "SELECT id, email FROM customers"
  - name: fake_orders
    generator: faker
    fields:
      orderId: uuid
      customerEmail: "from_source(customers.email)"
      amount: "decimal(10, 1000)"
scenario:
  - name: use_fake
    protocol: https
    endpoint: /api/orders
```

#### Example 20 — Multiple Data Sources

Combining database, CSV, and JSON feeds in a single test (§4.2).

```yaml
test: multi_source
data_sources:
  - name: users
    source: database
    connection: "postgres://db/users"
    query: "SELECT * FROM users LIMIT 100"
  - name: products
    source: csv
    path: ./products.csv
  - name: regions
    source: json
    path: ./regions.json
scenario:
  - name: test_step
    protocol: https
    endpoint: /api/test
```

---

### Category 5: Branching and Validation (Examples 21–25)

*Conditional execution, chained branching, and assertion-driven flow control (§5.2, §8).*

#### Example 21 — Simple Validation

A single assertion on response status (§8.1).

```yaml
test: validation_test
scenario:
  - name: check_api
    protocol: https
    endpoint: /api/status
    validate:
      - "response.status == 200"
```

#### Example 22 — Multiple Validations

Multiple assertion expressions on a single step (§8.1).

```yaml
test: multi_validation
scenario:
  - name: api_call
    protocol: https
    endpoint: /api/data
    validate:
      - "response.status == 200"
      - "response.time < 500ms"
      - "header['Content-Type'] exists"
```

#### Example 23 — Conditional Branching

`on_success` / `on_failure` directives route the execution graph based on validation results (§5.2).

```yaml
test: branching
scenario:
  - name: validate_order
    validate:
      - "context.order.amount > 0"
    on_success: charge_payment
    on_failure: reject_order
  - name: charge_payment
    protocol: https
    endpoint: /api/charge
    method: POST
  - name: reject_order
    protocol: https
    endpoint: /api/reject
    method: POST
```

#### Example 24 — Chained Branching

Multi-level decision trees with cascading branch conditions.

```yaml
test: chained_branching
scenario:
  - name: check_user_tier
    validate:
      - "context.user.tier == 'premium'"
    on_success: premium_flow
    on_failure: standard_flow
  - name: premium_flow
    validate:
      - "context.user.balance > 1000"
    on_success: vip_processing
    on_failure: standard_premium
  - name: vip_processing
    protocol: https
    endpoint: /api/vip
  - name: standard_premium
    protocol: https
    endpoint: /api/premium
  - name: standard_flow
    protocol: https
    endpoint: /api/standard
```

#### Example 25 — Branching with Extract and Validate

Combining extraction (§6.2) with conditional branching for data-driven flow control.

```yaml
test: extract_and_branch
scenario:
  - name: place_order
    protocol: https
    endpoint: /api/orders
    method: POST
    body:
      amount: 150.00
    extract:
      orderId: response.body.orderId
      orderStatus: response.body.status
  - name: check_status
    validate:
      - "context.orderStatus == 'approved'"
    on_success: confirm_order
    on_failure: flag_for_review
  - name: confirm_order
    protocol: https
    endpoint: /api/orders/confirm
    method: POST
  - name: flag_for_review
    protocol: https
    endpoint: /api/orders/review
    method: POST
```

---

### Category 6: Parallel Execution (Examples 26–30)

*Concurrent branches, mixed linear/parallel flows, and nested parallelism (§5.3).*

#### Example 26 — Simple Parallel (Two Branches)

Two independent branches execute concurrently (§5.3).

```yaml
test: parallel_basic
scenario:
  - name: parallel_step
    parallel:
      - - name: track_analytics
          protocol: https
          endpoint: /api/analytics
      - - name: notify_warehouse
          protocol: https
          endpoint: /api/warehouse
```

#### Example 27 — Parallel Branches with Multiple Steps

Each branch can contain its own sequential subgraph.

```yaml
test: parallel_multi_step
scenario:
  - name: parallel_ops
    parallel:
      - - name: branch1_step1
          protocol: https
          endpoint: /api/b1/s1
        - name: branch1_step2
          protocol: https
          endpoint: /api/b1/s2
      - - name: branch2_step1
          protocol: https
          endpoint: /api/b2/s1
        - name: branch2_step2
          protocol: https
          endpoint: /api/b2/s2
        - name: branch2_step3
          protocol: https
          endpoint: /api/b2/s3
```

#### Example 28 — Three Parallel Branches

Fan-out to three concurrent notification channels.

```yaml
test: parallel_three
scenario:
  - name: triple_parallel
    parallel:
      - - name: send_email
          protocol: https
          endpoint: /api/email
      - - name: send_sms
          protocol: https
          endpoint: /api/sms
      - - name: send_push
          protocol: https
          endpoint: /api/push
```

#### Example 29 — Linear Steps Before and After Parallel

Parallel blocks can be embedded anywhere in a linear flow. Steps before and after execute sequentially.

```yaml
test: linear_then_parallel
scenario:
  - name: authenticate
    protocol: https
    endpoint: /api/login
    method: POST
  - name: parallel_ops
    parallel:
      - - name: fetch_profile
          protocol: https
          endpoint: /api/profile
      - - name: fetch_settings
          protocol: https
          endpoint: /api/settings
  - name: render_dashboard
    protocol: https
    endpoint: /api/dashboard
```

#### Example 30 — Nested Parallel

Parallel blocks within parallel branches for deeply concurrent execution graphs.

```yaml
test: nested_parallel
scenario:
  - name: outer_parallel
    parallel:
      - - name: inner_parallel_a
          parallel:
            - - name: deep_a1
                protocol: https
                endpoint: /deep/a1
            - - name: deep_a2
                protocol: https
                endpoint: /deep/a2
      - - name: branch_b
          protocol: https
          endpoint: /branch/b
```

---

### Category 7: Loops and Iterations (Examples 31–35)

*Count-based loops, collection iteration, multi-step loop bodies, and nesting (§5.4).*

#### Example 31 — Simple Count Loop

Execute a step exactly N times (§5.4).

```yaml
test: count_loop
scenario:
  - name: repeat_call
    loop:
      count: 5
    do:
      - name: hit_api
        protocol: https
        endpoint: /api/ping
```

#### Example 32 — Collection Loop

Iterate over a context collection using `over`/`from` (§5.4).

```yaml
test: collection_loop
scenario:
  - name: add_to_cart
    loop:
      over: selected_product
      from: "${$context.products}"
    do:
      - name: add_item
        protocol: https
        endpoint: /api/cart/items
        method: POST
        body:
          productId: "${$selected_product.id}"
```

#### Example 33 — Combined Count and Collection Loop

Both `count` and `over`/`from` can be used together — execute at most N items from a collection (§5.4).

```yaml
test: hybrid_loop
scenario:
  - name: process_items
    loop:
      count: "${$random.int(1, 5)}"
      over: item
      from: "${$context.items}"
    do:
      - name: process
        protocol: https
        endpoint: /api/process
        method: POST
        body:
          itemId: "${$item.id}"
```

#### Example 34 — Loop with Multi-Step Body

The `do` block can contain multiple sequential steps forming a complete subgraph.

```yaml
test: loop_subgraph
scenario:
  - name: order_loop
    loop:
      count: 3
    do:
      - name: create_order
        protocol: https
        endpoint: /api/orders
        method: POST
      - name: verify_order
        protocol: https
        endpoint: /api/orders/verify
        method: GET
      - name: confirm_order
        protocol: https
        endpoint: /api/orders/confirm
        method: POST
```

#### Example 35 — Nested Loops

Loops can be nested: an outer loop containing an inner loop for matrix-style iteration.

```yaml
test: nested_loops
scenario:
  - name: outer_loop
    loop:
      count: 3
    do:
      - name: inner_loop
        loop:
          count: 2
        do:
          - name: deep_call
            protocol: https
            endpoint: /api/deep
```

---

### Category 8: Context and Extraction (Examples 36–40)

*Variable extraction, chained context propagation, metrics tracking, and interpolation (§6, §10.2).*

#### Example 36 — Basic Extract

Extract two variables from a login response into the context (§6.2).

```yaml
test: extract_test
scenario:
  - name: login
    protocol: https
    endpoint: /api/login
    method: POST
    extract:
      authToken: response.body.token
      userId: response.body.user.id
```

#### Example 37 — Chained Extraction Across Steps

Variables extracted in one step are injected into subsequent steps via `$context` (§6.1, §6.3).

```yaml
test: chained_extract
scenario:
  - name: login
    protocol: https
    endpoint: /api/login
    method: POST
    body:
      username: "admin"
    extract:
      token: response.body.token
  - name: get_orders
    protocol: https
    endpoint: /api/orders
    method: GET
    headers:
      Authorization: "Bearer ${$context.token}"
    extract:
      firstOrderId: "response.body.orders[0].id"
  - name: get_order_detail
    protocol: https
    endpoint: "/api/orders/${$context.firstOrderId}"
    method: GET
    extract:
      orderStatus: response.body.status
```

#### Example 38 — Extract with Validation

Extraction and validation can coexist on the same step.

```yaml
test: extract_validate
scenario:
  - name: create_entity
    protocol: https
    endpoint: /api/entities
    method: POST
    body:
      type: "widget"
    extract:
      entityId: response.body.id
      createdAt: response.body.createdAt
    validate:
      - "response.status == 201"
      - "response.body.id exists"
```

#### Example 39 — Custom Business Metric Tracking

Track a custom metric with dimensions for aggregated analysis (§10.2).

```yaml
test: metric_tracking
scenario:
  - name: process_payment
    protocol: https
    endpoint: /api/payments
    method: POST
    track_metric:
      name: payment_approval_rate
      value: "json.parse(response.body).status == 'approved'"
      dimensions:
        payment_method: request.body.method
        customer_tier: context.customer.tier
```

#### Example 40 — Context Interpolation in Body and Headers

Template expressions in both headers and body demonstrate full context propagation (§6.3).

```yaml
test: interpolation_body
scenario:
  - name: setup
    protocol: https
    endpoint: /api/setup
    extract:
      sessionId: response.body.sessionId
      config: response.body.config
  - name: use_context
    protocol: https
    endpoint: /api/action
    method: POST
    headers:
      X-Session: "${$context.sessionId}"
    body:
      action: "process"
      configRef: "${$context.config.id}"
      timestamp: "${$faker.iso8601()}"
```

---

### Category 9: Retries and Synchronization (Examples 41–45)

*Retry strategies, wait barriers, fire-and-forget steps, and temporal assertions (§5.5, §5.6, §8.3).*

#### Example 41 — Simple Retry (Exponential Backoff)

Retry a flaky endpoint up to 3 times with exponential backoff starting at 2 seconds (§5.6).

```yaml
test: retry_test
scenario:
  - name: flaky_call
    protocol: https
    endpoint: /api/flaky
    retry:
      max_attempts: 3
      backoff: exponential
      delay: 2s
```

#### Example 42 — Retry with Linear Backoff and Validation

Combine retry logic with validation — retries trigger only on assertion failures (§5.6, §8.1).

```yaml
test: retry_linear
scenario:
  - name: call_with_retry
    protocol: https
    endpoint: /api/unreliable
    retry:
      max_attempts: 5
      backoff: linear
      delay: 1s
    validate:
      - "response.status == 200"
```

#### Example 43 — Wait-For Synchronization Barrier

Block execution until named parallel tasks complete. Includes timeout-based branching (§5.5).

```yaml
test: sync_test
scenario:
  - name: parallel_work
    parallel:
      - - name: task_a
          protocol: https
          endpoint: /api/task_a
      - - name: task_b
          protocol: https
          endpoint: /api/task_b
  - name: aggregate
    protocol: https
    endpoint: /api/aggregate
    wait_for:
      - task_a
      - task_b
    timeout: 30s
    on_timeout: handle_timeout
  - name: handle_timeout
    protocol: https
    endpoint: /api/timeout_handler
```

#### Example 44 — Async Fire-and-Forget

Steps marked `async: true` do not block the execution graph (§5.5).

```yaml
test: async_test
scenario:
  - name: main_call
    protocol: https
    endpoint: /api/main
    method: POST
  - name: send_telemetry
    protocol: https
    endpoint: /api/telemetry
    async: true
  - name: continue_flow
    protocol: https
    endpoint: /api/next
```

#### Example 45 — Temporal Assertions (`within`)

Both simple duration and structured `since`-linked temporal assertions (§8.3). Useful for CDC verification.

```yaml
test: within_test
scenario:
  - name: place_order
    protocol: https
    endpoint: /api/orders
    method: POST
  - name: check_cdc_simple
    protocol: kafka
    topic: orders.cdc
    mode: observe
    expect:
      message.operation: INSERT
    within: 5s
  - name: check_cdc_structured
    protocol: kafka
    topic: orders.cdc
    mode: observe
    expect:
      message.operation: UPDATE
    within:
      duration: 5s
      since: place_order
```

---

### Category 10: Complex Integration Scenarios (Examples 46–50)

*Full end-to-end flows combining multiple DSL features, deep nesting, modules, scripting, and the complete feature set.*

#### Example 46 — Full E-Commerce Checkout Flow

Combines: metadata, config (step ramp-up), data sources, extraction, loops, parallel branches, retries, validation, and metric outputs.

```yaml
test: ecommerce_checkout
description: "End-to-end e-commerce checkout with CDC verification"
metadata:
  author: "Evento Agent"
  created: "2026-07-19T00:00:00Z"
  tags:
    - e2e
    - checkout
config:
  mode: realtime
  virtual_users: 10
  duration: 5m
  ramp_up:
    strategy: step
    duration: 30s
    steps: 5
  timeout: 10m
data_sources:
  - name: customers
    source: database
    connection: "${DB_URI}"
    query: "SELECT id FROM test_customers"
scenario:
  - name: login
    description: "Authenticate user and get JWT"
    protocol: https
    endpoint: /api/login
    method: POST
    body:
      username: "${$faker.username()}"
      password: "test123"
    extract:
      authToken: response.body.token
      userId: response.body.userId
    validate:
      - "response.status == 200"
  - name: browse_products
    protocol: https
    endpoint: /api/products
    method: GET
    headers:
      Authorization: "Bearer ${$context.authToken}"
    extract:
      products: response.body.items
  - name: add_items_loop
    loop:
      count: 3
      over: product
      from: "${$context.products}"
    do:
      - name: add_to_cart
        protocol: https
        endpoint: /api/cart/items
        method: POST
        headers:
          Authorization: "Bearer ${$context.authToken}"
        body:
          productId: "${$product.id}"
          quantity: 1
  - name: place_order
    protocol: https
    endpoint: /api/orders
    method: POST
    headers:
      Authorization: "Bearer ${$context.authToken}"
    extract:
      orderId: response.body.orderId
    validate:
      - "response.status == 201"
    retry:
      max_attempts: 2
      backoff: exponential
      delay: 1s
  - name: verify_notifications
    parallel:
      - - name: check_email
          protocol: https
          endpoint: /api/notifications/email
          validate:
            - "response.status == 200"
      - - name: check_sms
          protocol: https
          endpoint: /api/notifications/sms
          validate:
            - "response.status == 200"
outputs:
  - format: prometheus
    endpoint: "http://prometheus:9090"
  - format: ai_insights
    file: "results/insights_${$timestamp}.json"
```

#### Example 47 — Six-Level Deep Nesting

Stress-tests the parser with deeply nested parallel and loop constructs: parallel → loop → parallel → loop → parallel → leaf steps.

```yaml
test: deep_nesting
description: "6 levels of depth testing parser robustness"
scenario:
  - name: level1
    parallel:
      - - name: level2_branch_a
          loop:
            count: 2
          do:
            - name: level3_loop_body
              parallel:
                - - name: level4_parallel_a
                    loop:
                      count: 1
                    do:
                      - name: level5_inner
                        parallel:
                          - - name: level6_leaf_a
                              protocol: https
                              endpoint: /api/deep/a
                              validate:
                                - "response.status == 200"
                          - - name: level6_leaf_b
                              protocol: https
                              endpoint: /api/deep/b
                - - name: level4_parallel_b
                    protocol: https
                    endpoint: /api/level4b
      - - name: level2_branch_b
          protocol: https
          endpoint: /api/level2b
```

#### Example 48 — Module Use and Custom Functions

Demonstrates `imports`, `functions`, `use_module`, `with`/`outputs_to`, and `transform` pipelines (§9.2, §11.1).

```yaml
test: module_test
imports:
  - modules/auth_flow.yaml
  - modules/payment.yaml
functions:
  - name: calculate_tax
    language: rust
    source: ./functions/tax.rs
  - name: format_address
    language: python
    source: ./functions/address.py
scenario:
  - name: authenticate
    use_module: authenticate_user
    with:
      username: "${$faker.username()}"
      password: "test_password_123"
    outputs_to: auth_context
  - name: process_order
    protocol: https
    endpoint: /api/orders
    method: POST
    transform:
      - input: "${$context.order.amount}"
        function: calculate_tax
        params:
          region: "${$context.customer.region}"
        output_to: tax_amount
```

#### Example 49 — Scripting, Inheritance, and Global Validation

Combines `base`/`extends` inheritance (§11.2), inline scripting with `runtime` (§9.1), and global `validation` business rules (§8.2).

```yaml
test: scripted_test
base: api_test_base.yaml
extends: base
config:
  timeout: 30s
scenario:
  - name: complex_decision
    script: |
      if context.user.tier == "premium":
          return "kafka", "premium-topic"
      else:
          return "https", "/standard-endpoint"
    runtime: python
  - name: process_result
    protocol: https
    endpoint: /api/process
    method: POST
validation:
  business_rules:
    - name: order_completion_rate
      threshold: 0.95
      actual: "${$metrics.completed_orders / metrics.total_orders}"
    - name: avg_latency
      threshold: 500.0
      actual: "${$metrics.avg_latency_ms}"
```

#### Example 50 — Mega Scenario (All Features)

The ultimate integration test exercising every eDSL feature simultaneously: metadata, imports, config (structured ramp-up, timeouts), three data sources (database, CSV, faker with referential integrity), custom functions, module invocation, extraction, validation, loops with retries, parallel branches with async, transforms, custom metric tracking, conditional branching, global business rules, and multi-format outputs.

```yaml
test: mega_integration
description: "Comprehensive test exercising every DSL feature"
metadata:
  author: "Evento CI"
  created: "2026-07-19T20:00:00Z"
  tags:
    - integration
    - full
    - ci
imports:
  - modules/auth.yaml
config:
  mode: realtime
  virtual_users: 50
  duration: 10m
  ramp_up:
    strategy: step
    duration: 1m
    steps: 10
  timeout: 15m
  step_timeout: 30s
data_sources:
  - name: users
    source: database
    connection: "${DB_URI}"
    query: "SELECT id, email FROM users WHERE test = true"
    sampling: random
    cache: true
  - name: products
    source: csv
    path: ./data/products.csv
    sampling: sequential
  - name: fake_orders
    generator: faker
    fields:
      orderId: uuid
      customerEmail: "from_source(users.email)"
      amount: "decimal(10, 1000)"
functions:
  - name: calc_tax
    language: rust
    source: ./functions/tax.rs
scenario:
  - name: auth
    use_module: authenticate
    with:
      username: "${$faker.username()}"
      password: "${$faker.password()}"
    outputs_to: auth
  - name: browse
    protocol: https
    endpoint: /api/products
    method: GET
    headers:
      Authorization: "Bearer ${$context.auth.token}"
    extract:
      productList: response.body.items
    validate:
      - "response.status == 200"
      - "response.body.items.length > 0"
  - name: shopping_loop
    description: "Add multiple products to cart"
    loop:
      count: "${$random.int(1, 5)}"
      over: product
      from: "${$context.productList}"
    do:
      - name: add_to_cart
        protocol: https
        endpoint: /api/cart
        method: POST
        headers:
          Authorization: "Bearer ${$context.auth.token}"
        body:
          productId: "${$product.id}"
          quantity: 1
        validate:
          - "response.status == 200"
        retry:
          max_attempts: 2
          backoff: constant
          delay: 500ms
  - name: checkout
    protocol: https
    endpoint: /api/checkout
    method: POST
    headers:
      Authorization: "Bearer ${$context.auth.token}"
    extract:
      orderId: response.body.orderId
      totalAmount: response.body.total
    validate:
      - "response.status == 201"
    transform:
      - input: "${$context.totalAmount}"
        function: calc_tax
        params:
          region: "${$context.auth.region}"
        output_to: taxAmount
    track_metric:
      name: checkout_success_rate
      value: "response.status == 201"
      dimensions:
        region: "${$context.auth.region}"
  - name: post_checkout_verification
    parallel:
      - - name: verify_email
          protocol: https
          endpoint: /api/verify/email
          validate:
            - "response.status == 200"
          timeout: 10s
      - - name: verify_inventory
          protocol: https
          endpoint: /api/verify/inventory
          validate:
            - "response.status == 200"
      - - name: send_analytics
          protocol: https
          endpoint: /api/analytics
          async: true
  - name: conditional_refund
    validate:
      - "context.totalAmount > 500"
    on_success: process_vip_refund
    on_failure: standard_completion
  - name: process_vip_refund
    protocol: https
    endpoint: /api/refund/vip
    method: POST
    retry:
      max_attempts: 3
      backoff: exponential
      delay: 2s
  - name: standard_completion
    protocol: https
    endpoint: /api/complete
    method: POST
validation:
  business_rules:
    - name: checkout_rate
      threshold: 0.90
      actual: "${$metrics.successful_checkouts / metrics.total_attempts}"
    - name: latency_p99
      threshold: 2000.0
      actual: "${$metrics.checkout_p99_ms}"
outputs:
  - format: prometheus
    endpoint: "http://prometheus:9090"
  - format: ai_insights
    file: "results/mega_${$timestamp}.json"
```

---

### Category 11: Step-Level Mocking (Examples 51–55)

*Mock definitions, dynamic responses, stateful sequences, failure injection, and full integration with mocks (§7.6).*

#### Example 51 — Simple HTTP Mock with Static Response

A step with a co-located mock that returns a static JSON response.

```yaml
test: simple_http_mock
config:
  mock_strategy: auto
scenario:
  - name: get_user
    protocol: https
    endpoint: /api/users/123
    method: GET
    mock:
      response:
        status: 200
        headers:
          Content-Type: application/json
        body:
          id: "123"
          name: "Jane Doe"
          email: "jane@example.com"
    validate:
      - "response.status == 200"
    extract:
      userName: response.body.name
```

#### Example 52 — Mock with Dynamic Response and Contract Schema

Demonstrates `$faker`, `$request` interpolation, and request/response contract schemas (§7.6.2, §7.6.6).

```yaml
test: dynamic_mock_with_contract
scenario:
  - name: create_order
    protocol: https
    endpoint: /api/orders
    method: POST
    body:
      customerId: "${$faker.uuid()}"
      amount: 149.99
      currency: "USD"
    mock:
      request_schema:
        type: object
        properties:
          customerId:
            type: string
          amount:
            type: number
            minimum: 0
          currency:
            type: string
            enum: [USD, EUR, GBP]
        required: [customerId, amount]
      response_schema:
        type: object
        properties:
          orderId:
            type: string
          status:
            type: string
            enum: [created, pending, failed]
          amount:
            type: number
        required: [orderId, status]
      response:
        status: 201
        body:
          orderId: "${$faker.uuid()}"
          status: "created"
          amount: "${$request.body.amount}"
          processedAt: "${$faker.iso8601()}"
    extract:
      orderId: response.body.orderId
    validate:
      - "response.status == 201"
```

#### Example 53 — Stateful Mock Sequence (Polling Pattern)

Demonstrates ordered response sequences with `on_exhausted` for simulating a job polling workflow (§7.6.3).

```yaml
test: stateful_mock_polling
scenario:
  - name: submit_job
    protocol: https
    endpoint: /api/jobs
    method: POST
    body:
      type: "data_export"
    mock:
      response:
        status: 202
        body:
          jobId: "job-42"
          status: "accepted"
    extract:
      jobId: response.body.jobId
  - name: poll_status
    protocol: https
    endpoint: "/api/jobs/${$context.jobId}/status"
    method: GET
    mock:
      responses:
        - status: 200
          body: { status: "processing", progress: 25 }
        - status: 200
          body: { status: "processing", progress: 75 }
        - status: 200
          body: { status: "completed", result: "success" }
      on_exhausted: repeat_last
    validate:
      - "response.status == 200"
```

#### Example 54 — Failure Injection Mock

Demonstrates probabilistic error injection with latency distribution for resilience testing (§7.6.4).

```yaml
test: failure_injection_mock
scenario:
  - name: call_flaky_service
    protocol: https
    endpoint: /api/inventory/check
    method: POST
    body:
      productId: "p-001"
    mock:
      response:
        status: 200
        body:
          available: true
          quantity: "${$random.int(1, 100)}"
      behavior:
        error_rate: 0.3
        error_response:
          status: 503
          body:
            error: "Service Unavailable"
            retryAfter: 5
        latency:
          distribution: normal
          mean: 200ms
          stddev: 50ms
        timeout_rate: 0.05
    retry:
      max_attempts: 3
      backoff: exponential
      delay: 1s
```

#### Example 55 — Full Integration Test with All Services Mocked

A complete end-to-end checkout flow where every service interaction is mocked — ready to run without any live infrastructure. Demonstrates progressive fidelity: set `mock_strategy: disabled` to run against real services.

```yaml
test: fully_mocked_checkout
description: "Complete e-commerce checkout with all services mocked"
config:
  mock_strategy: auto
  virtual_users: 5
  duration: 2m
scenario:
  - name: login
    protocol: https
    endpoint: /api/auth/login
    method: POST
    body:
      username: "${$faker.username()}"
      password: "test123"
    mock:
      request_schema:
        type: object
        properties:
          username: { type: string }
          password: { type: string }
        required: [username, password]
      response:
        status: 200
        body:
          token: "${$faker.uuid()}"
          userId: "${$faker.uuid()}"
    extract:
      authToken: response.body.token
      userId: response.body.userId
  - name: browse_products
    protocol: https
    endpoint: /api/products
    method: GET
    headers:
      Authorization: "Bearer ${$context.authToken}"
    mock:
      response:
        status: 200
        body:
          items:
            - id: "prod-001"
              name: "Widget A"
              price: 29.99
            - id: "prod-002"
              name: "Widget B"
              price: 49.99
    extract:
      products: response.body.items
  - name: place_order
    protocol: https
    endpoint: /api/orders
    method: POST
    headers:
      Authorization: "Bearer ${$context.authToken}"
    body:
      userId: "${$context.userId}"
      items:
        - productId: "prod-001"
          quantity: 2
    mock:
      response_schema:
        type: object
        properties:
          orderId: { type: string }
          total: { type: number }
          status: { type: string, enum: [created, pending] }
        required: [orderId, total, status]
      response:
        status: 201
        body:
          orderId: "${$faker.uuid()}"
          total: 59.98
          status: "created"
    extract:
      orderId: response.body.orderId
    validate:
      - "response.status == 201"
  - name: verify_order_in_db
    protocol: database
    connection: postgres://orders-db
    query: "SELECT status FROM orders WHERE id = '${$context.orderId}'"
    mock:
      response:
        rows:
          - status: "created"
            order_id: "${$context.orderId}"
  - name: observe_order_event
    protocol: kafka
    topic: orders.cdc
    mode: observe
    expect:
      message.operation: INSERT
    within: 5s
    mock:
      response:
        message:
          operation: INSERT
          after:
            id: "${$context.orderId}"
            status: "created"
        delay: 200ms
  - name: send_notification
    protocol: https
    endpoint: /api/notifications
    method: POST
    body:
      userId: "${$context.userId}"
      type: "order_confirmation"
      orderId: "${$context.orderId}"
    mock:
      response:
        status: 202
        body:
          notificationId: "${$faker.uuid()}"
          queued: true
    validate:
      - "response.status == 202"
```

---

## 15. References

1.  RFC 2119: Key words for use in RFCs to Indicate Requirement Levels.
2.  YAML Ain't Markup Language (YAML) Version 1.2.
3.  The `evento` project Vision and Architecture documents (2026).
