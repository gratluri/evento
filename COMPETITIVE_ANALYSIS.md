# Competitive Analysis: evento vs Existing Load Testing Tools

## Date: July 18, 2026

This document captures our analysis of existing load testing tools and how `evento` can differentiate itself in the market.

---

## Current Market Landscape

### JMeter
**Message Generation Capabilities:**
- Supports Velocity and Groovy templates for dynamic payload generation
- Uses "Feeders" to inject CSV/database data into requests
- Can use JSR223 PreProcessors (Groovy/Java) for complex data generation
- Faker data generation requires custom scripting or plugins

**Limitations:**
- Template syntax is clunky and requires mixing Java/Groovy code
- Multi-protocol support exists but feels bolted-on (requires different samplers for each protocol)
- Configuration is XML-heavy and not developer-friendly
- Poor support for modern protocols (gRPC requires plugins, Kafka support is limited)
- Ancient reporting and visualization capabilities

---

### Gatling
**Message Generation Capabilities:**
- Built on Scala, so you write code-as-tests (not templates)
- Supports DataFaker library integration for realistic fake data ([official guide](https://docs.gatling.io/guides/optimize-scripts/generate-test-data/))
- Feeders can pull from CSV, JSON, JDBC databases
- Strong template support for request bodies using Scala string interpolation

**Limitations:**
- Requires Scala knowledge (barrier to entry for most teams)
- Primarily HTTP/WebSocket focused; other protocols need extensions
- Not truly a DSL - you're writing Scala code
- Limited enterprise protocol ecosystem

---

### k6
**Message Generation Capabilities:**
- JavaScript-based scripting (accessible to web developers)
- Extension ecosystem supports Kafka with Avro, Protobuf, JSON Schema ([xk6-kafka extension](https://github.com/mostafa/xk6-kafka))
- Can generate data using JS libraries (faker.js, etc.)
- Modular architecture for custom data generation

**Limitations:**
- No built-in templating engine - you build strings in JavaScript
- Extension model requires rebuilding k6 binary for new protocols
- Limited enterprise protocol support out-of-box (JMS, mainframe protocols)
- Primarily HTTP-focused with extensions as afterthought

---

### Locust
**Message Generation Capabilities:**
- Python-based, so you use Python libraries for data generation (Faker, etc.)
- Very flexible - can "call internal libraries, integrate with feature flags, token services" ([source](https://thelinuxcode.com/load-testing-with-locust-in-2026-a-practical-python-first-guide/))
- Easy to integrate with databases, files for test data
- Low barrier to entry for Python developers

**Limitations:**
- No DSL - pure Python code (low-level, verbose)
- Primarily HTTP-focused; other protocols require custom client code
- Message templating is DIY with Python string formatting
- Performance limitations due to Python runtime

---

## evento Differentiation Strategy

### 1. True Protocol Abstraction in DSL

**Example:**
```yaml
scenario: payment_flow
steps:
  - name: receive_order
    protocol: kafka
    topic: orders.new
    message:
      format: avro
      schema: order_schema.avsc
      template: |
        {
          "orderId": {{ faker.uuid() }},
          "amount": {{ random.decimal(10, 1000) }},
          "currency": {{ cycle(['USD', 'EUR', 'GBP']) }}
        }
  
  - name: call_inventory
    protocol: grpc
    service: inventory.InventoryService/CheckStock
    message:
      format: protobuf
      template: |
        product_id: "{{ steps.receive_order.message.productId }}"
        quantity: {{ steps.receive_order.message.quantity }}
```

**Why this matters:** One unified DSL for all protocols. Competitors force you to learn different APIs/syntax for each protocol.

---

### 2. Stateful Test Flows with Context Passing

**Example:**
```yaml
- name: create_user
  protocol: https
  endpoint: /api/users
  extract:
    userId: response.body.id
    authToken: response.headers['X-Auth-Token']

- name: update_profile
  protocol: https
  endpoint: /api/users/{{ context.userId }}
  headers:
    Authorization: "Bearer {{ context.authToken }}"
```

**Current tools:** Require manual scripting to extract and pass values between steps. JMeter's "correlation" is notoriously painful and error-prone.

**evento advantage:** First-class context management built into the DSL with type-safe variable passing.

---

### 3. Built-in Smart Data Generation with Relationships

**Example:**
```yaml
data_sources:
  - name: customers
    source: database
    connection: postgres://prod-readonly
    query: "SELECT id, email, tier FROM customers WHERE active = true"
    sampling: random
    cache: true
  
  - name: fake_orders
    generator: faker
    fields:
      orderId: uuid
      customerEmail: from_source(customers.email)  # Maintains referential integrity!
      amount: decimal(10, 1000)
      timestamp: datetime.recent(days=7)
```

**Unique feature:** Mixing real production data with faker-generated data while maintaining referential integrity. Competitors do one or the other, not both elegantly.

**Use cases:**
- Test with real customer distribution patterns but synthetic PII
- Maintain foreign key relationships across test data
- Replay production patterns with modified volumes

---

### 4. Protocol-Aware Validation & Business Metrics

**Example:**
```yaml
- name: payment_request
  protocol: https
  validate:
    - http.status == 200
    - response.time < 500ms
    - json.parse(response.body).status == "approved"  # Business rule
    - header['X-Idempotency-Key'] exists
  
  track_metric:
    name: payment_approval_rate
    value: json.parse(response.body).status == "approved"
    dimensions:
      payment_method: request.body.method
      customer_tier: context.customer.tier
```

**Why better:** Built-in parsing for each format (JSON, XML, Protobuf, Avro) plus custom business metrics tracked automatically. Competitors require custom code for business-level assertions.

**Impact:** Track not just "did it respond?" but "did the business logic succeed?"

---

### 5. Enterprise Protocol Support Out-of-Box

**Supported protocols (day one or roadmap):**
```yaml
protocols:
  - https (with OAuth2, mTLS, custom auth)
  - grpc (with reflection, custom metadata)
  - kafka (with Schema Registry, SASL/SSL)
  - jms (ActiveMQ, IBM MQ, RabbitMQ)
  - database (SQL, CQL, MongoDB queries)
  - soap (with WSDL parsing)
  - custom (extensible plugin system)
```

**Gap in market:** JMeter, Gatling, k6, and Locust are HTTP-first tools. Enterprise systems use JMS, mainframe protocols, database queries, and proprietary messaging systems. These are afterthoughts in existing tools.

**evento positioning:** Enterprise integration testing platform, not just HTTP load tester.

---

### 6. Hybrid Declarative + Procedural DSL

**Example:**
```yaml
# Declarative for simple cases
- name: simple_request
  protocol: https
  endpoint: /api/health
  
# Drop into Rust/scripting for complex logic
- name: complex_decision
  script: |
    if context.user.tier == "premium" {
      send_to("kafka", "premium-topic")
    } else {
      send_to("https", "/standard-endpoint")
    }
  runtime: rust  # Embedded Rust interpreter or compile-time evaluation
```

**Advantage:** 
- Gatling forces you into Scala
- JMeter's scripting is ugly Groovy embedded in XML
- k6 forces everything into JavaScript
- evento gives you an escape hatch without leaving the tool ecosystem

---

### 7. Production-Grade Observability Built-in

**Example:**
```yaml
# Auto-exports to Prometheus, Datadog, CloudWatch, Grafana
metrics:
  default_exports:
    - latency_p50, latency_p95, latency_p99
    - request_rate, error_rate, success_rate
    - protocol_specific:
        kafka: [lag, consumer_offset, partition_distribution]
        grpc: [stream_duration, message_count]
        database: [query_time, row_count, connection_pool_usage]
  
  custom_metrics:
    - name: order_value_distribution
      type: histogram
      buckets: [10, 50, 100, 500, 1000]
      
  export_to:
    - prometheus:
        endpoint: localhost:9090
    - datadog:
        api_key: ${DATADOG_API_KEY}
```

**Gap in market:** 
- k6 has good HTTP metrics but lacks protocol-specific observability
- JMeter's reporting is from the early 2000s
- Gatling has decent reporting but not real-time streaming
- evento could have modern observability as a first-class citizen from day one

---

## Summary: evento's Competitive Moat

### What Makes evento 10x Better

1. **One DSL, all protocols** - competitors are HTTP tools with protocol extensions bolted on
2. **Enterprise protocols first-class** - JMS, mainframe, legacy systems (massive underserved market)
3. **Smart data generation** - faker + database + relationships in one coherent model
4. **Business-level validation** - not just "HTTP 200" but "order approved AND inventory reserved"
5. **Rust performance** - handle 10x more load per node than Python/JavaScript tools
6. **Modern DevEx** - YAML/TOML DSL that feels like Terraform/Kubernetes configs, not XML hell
7. **Production-grade observability** - protocol-aware metrics, streaming to modern observability platforms

---

## The Elevator Pitch

> **"JMeter and Gatling are HTTP load testers. evento is an enterprise integration testing platform."**
> 
> Test your complete business flows: Kafka → gRPC → Database → HTTP with one unified script, real production data patterns, and business-level assertions - not just response codes.

---

## Target Market Positioning

### Primary Target
- **Enterprises with microservices/distributed systems** that use heterogeneous protocols
- **Companies migrating from monoliths** who need to test complex async interactions
- **Teams frustrated with JMeter/Gatling** for multi-protocol scenarios

### Secondary Target
- **DevOps/SRE teams** who need load testing integrated into CI/CD
- **Performance testing consultancies** looking for modern tooling
- **Cloud-native companies** building event-driven architectures

### Wedge Strategy
1. **Open source core** - build community, get adoption
2. **Enterprise features** - team collaboration, test management, compliance reporting
3. **Cloud service** - distributed execution, managed infrastructure
4. **Professional services** - implementation, training, custom protocol development

---

## Next Steps for Development

1. **Validate with potential users** - interview 20+ teams about their current pain points
2. **Define MVP scope** - pick 3 protocols + 3 message formats for v0.1
3. **Build DSL prototype** - prove the concept with simple working examples
4. **Performance benchmarks** - demonstrate Rust's advantage over competitors
5. **YC application narrative** - tie technical advantages to market opportunity

---

## Open Questions for Further Research

1. What's the actual market size for enterprise testing tools?
2. How do enterprises currently buy testing tools (bottom-up vs top-down)?
3. What compliance/security requirements exist for this category?
4. Can we partner with observability platforms (Datadog, New Relic) for distribution?
5. What open source business model works best (open core vs cloud service)?
