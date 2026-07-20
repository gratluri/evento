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

*   `test` (string, REQUIRED): The unique identifier for the test suite.
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

### 14.1 Simple HTTP API Test

```yaml
test: simple_api_check
scenario:
  - name: fetch_users
    protocol: https
    endpoint: /api/v1/users
    method: GET
    validate:
      - response.status == 200
      - response.body.data.length > 0
```

### 14.2 Complex Event-Driven E-Commerce Flow

```yaml
test: checkout_flow
data_sources:
  - name: customers
    source: database
    connection: ${DB_URI}
    query: "SELECT id FROM test_customers"

scenario:
  - name: place_order
    protocol: https
    endpoint: /api/orders
    method: POST
    body:
      customerId: "${$data.customers.id}"
      amount: "${$random.decimal(10, 100)}"
    extract:
      orderId: response.body.orderId

  - name: verify_cdc_event
    protocol: kafka
    topic: orders.cdc
    mode: observe
    expect:
      message.operation: INSERT
      message.after.id: "${$context.orderId}"
    within:
      duration: 3s
      since: place_order
```

---

## 15. References

1.  RFC 2119: Key words for use in RFCs to Indicate Requirement Levels.
2.  YAML Ain't Markup Language (YAML) Version 1.2.
3.  The `evento` project Vision and Architecture documents (2026).
