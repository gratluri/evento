#[cfg(test)]
mod tests {
    use crate::dsl::dsl_parser::*;

    // =========================================================================
    // Category 1: Basic Scenarios (1-5) — Single step, minimal config
    // =========================================================================

    #[test]
    fn test_01_minimal_test_plan() {
        let yaml = r#"
test: minimal
scenario:
  - name: noop
    protocol: https
    endpoint: /health
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.test, "minimal");
        assert_eq!(plan.scenario.len(), 1);
        assert_eq!(plan.scenario[0].name, "noop");
    }

    #[test]
    fn test_02_name_alias_normalizes_to_test() {
        // Spec says use `test`; we also accept `name` as alias
        let yaml = r#"
name: aliased_test
scenario:
  - name: step1
    protocol: https
    endpoint: /
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.test, "aliased_test");
    }

    #[test]
    fn test_03_single_get_request() {
        let yaml = r#"
test: simple_get
scenario:
  - name: fetch_users
    protocol: https
    endpoint: /api/v1/users
    method: GET
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let step = &plan.scenario[0];
        assert_eq!(step.protocol.as_deref(), Some("https"));
        assert_eq!(step.method.as_deref(), Some("GET"));
        assert_eq!(step.endpoint.as_deref(), Some("/api/v1/users"));
    }

    #[test]
    fn test_04_single_post_with_body() {
        let yaml = r#"
test: post_test
scenario:
  - name: create_user
    protocol: https
    endpoint: /api/users
    method: POST
    body:
      username: "testuser"
      email: "test@example.com"
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let step = &plan.scenario[0];
        assert_eq!(step.method.as_deref(), Some("POST"));
        assert!(step.body.is_some());
    }

    #[test]
    fn test_05_step_with_description() {
        let yaml = r#"
test: described_steps
scenario:
  - name: health_check
    description: "Verifies the API is alive and responding"
    protocol: https
    endpoint: /health
    method: GET
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(
            plan.scenario[0].description.as_deref(),
            Some("Verifies the API is alive and responding")
        );
    }

    // =========================================================================
    // Category 2: Multi-Step Linear (6-10) — Sequential steps
    // =========================================================================

    #[test]
    fn test_06_two_sequential_steps() {
        let yaml = r#"
test: two_steps
scenario:
  - name: step_one
    protocol: https
    endpoint: /first
  - name: step_two
    protocol: https
    endpoint: /second
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.scenario.len(), 2);
        assert_eq!(plan.scenario[0].name, "step_one");
        assert_eq!(plan.scenario[1].name, "step_two");
    }

    #[test]
    fn test_07_steps_with_headers() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let headers = plan.scenario[0].headers.as_ref().unwrap();
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer token123");
        assert_eq!(headers.len(), 3);
    }

    #[test]
    fn test_08_five_step_pipeline() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.scenario.len(), 5);
        let names: Vec<&str> = plan.scenario.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["login", "get_profile", "update_settings", "submit_order", "logout"]);
    }

    #[test]
    fn test_09_step_with_complex_body() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert!(plan.scenario[0].body.is_some());
    }

    #[test]
    fn test_10_steps_with_interpolation_templates() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let extract = plan.scenario[0].extract.as_ref().unwrap();
        assert_eq!(extract.get("authToken").unwrap(), "response.body.token");
    }

    // =========================================================================
    // Category 3: Config Variations (11-15) — Modes, ramp-up, timeouts
    // =========================================================================

    #[test]
    fn test_11_realtime_mode_with_vus() {
        let yaml = r#"
test: load_test
config:
  mode: realtime
  virtual_users: 100
  duration: 5m
scenario:
  - name: hit_api
    protocol: https
    endpoint: /api/stress
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let config = plan.config.unwrap();
        assert_eq!(config.mode.as_deref(), Some("realtime"));
        assert_eq!(config.virtual_users, Some(100));
        assert_eq!(config.duration.as_deref(), Some("5m"));
    }

    #[test]
    fn test_12_replay_mode() {
        let yaml = r#"
test: replay_test
config:
  mode: replay
  source: /var/logs/traffic.log
  speed: 2.0
scenario:
  - name: replay_step
    protocol: https
    endpoint: /api/replay
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let config = plan.config.unwrap();
        assert_eq!(config.mode.as_deref(), Some("replay"));
        assert_eq!(config.source.as_deref(), Some("/var/logs/traffic.log"));
        assert_eq!(config.speed, Some(2.0));
    }

    #[test]
    fn test_13_scheduled_mode() {
        let yaml = r#"
test: scheduled_test
config:
  mode: scheduled
  cron: "0 */6 * * *"
scenario:
  - name: periodic_check
    protocol: https
    endpoint: /health
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let config = plan.config.unwrap();
        assert_eq!(config.mode.as_deref(), Some("scheduled"));
        assert_eq!(config.cron.as_deref(), Some("0 */6 * * *"));
    }

    #[test]
    fn test_14_ramp_up_simple_duration() {
        let yaml = r#"
test: ramp_simple
config:
  virtual_users: 50
  ramp_up: "30s"
  duration: 5m
scenario:
  - name: load_step
    protocol: https
    endpoint: /api/load
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let config = plan.config.unwrap();
        match config.ramp_up.unwrap() {
            RampUp::Duration(d) => assert_eq!(d, "30s"),
            _ => panic!("Expected RampUp::Duration"),
        }
    }

    #[test]
    fn test_15_ramp_up_structured_step_strategy() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let config = plan.config.unwrap();
        assert_eq!(config.timeout.as_deref(), Some("10m"));
        assert_eq!(config.step_timeout.as_deref(), Some("15s"));
        match config.ramp_up.unwrap() {
            RampUp::Structured(r) => {
                assert_eq!(r.strategy, "step");
                assert_eq!(r.duration, "30s");
                assert_eq!(r.steps, Some(5));
            }
            _ => panic!("Expected RampUp::Structured"),
        }
    }

    // =========================================================================
    // Category 4: Data Sources (16-20)
    // =========================================================================

    #[test]
    fn test_16_database_data_source() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let ds = &plan.data_sources.unwrap()[0];
        assert_eq!(ds.name, "customers");
        assert_eq!(ds.source.as_deref(), Some("database"));
        assert_eq!(ds.sampling.as_deref(), Some("random"));
        assert_eq!(ds.cache, Some(true));
    }

    #[test]
    fn test_17_csv_data_source() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let ds = &plan.data_sources.unwrap()[0];
        assert_eq!(ds.source.as_deref(), Some("csv"));
        assert_eq!(ds.path.as_deref(), Some("./data/products.csv"));
    }

    #[test]
    fn test_18_json_data_source() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let ds = &plan.data_sources.unwrap()[0];
        assert_eq!(ds.source.as_deref(), Some("json"));
        assert_eq!(ds.sampling.as_deref(), Some("shuffle"));
    }

    #[test]
    fn test_19_faker_data_source_with_referential_integrity() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let sources = plan.data_sources.unwrap();
        assert_eq!(sources.len(), 2);
        let faker = &sources[1];
        assert_eq!(faker.generator.as_deref(), Some("faker"));
        let fields = faker.fields.as_ref().unwrap();
        assert_eq!(fields.get("orderId").unwrap(), "uuid");
    }

    #[test]
    fn test_20_multiple_data_sources() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.data_sources.unwrap().len(), 3);
    }

    // =========================================================================
    // Category 5: Branching (21-25) — validate/on_success/on_failure
    // =========================================================================

    #[test]
    fn test_21_simple_validation() {
        let yaml = r#"
test: validation_test
scenario:
  - name: check_api
    protocol: https
    endpoint: /api/status
    validate:
      - "response.status == 200"
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let validates = plan.scenario[0].validate.as_ref().unwrap();
        assert_eq!(validates.len(), 1);
        assert_eq!(validates[0], "response.status == 200");
    }

    #[test]
    fn test_22_multiple_validations() {
        let yaml = r#"
test: multi_validation
scenario:
  - name: api_call
    protocol: https
    endpoint: /api/data
    validate:
      - "response.status == 200"
      - "response.time < 500ms"
      - "header['Content-Type'] exists"
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.scenario[0].validate.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_23_conditional_branching() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let step = &plan.scenario[0];
        assert_eq!(step.on_success.as_deref(), Some("charge_payment"));
        assert_eq!(step.on_failure.as_deref(), Some("reject_order"));
    }

    #[test]
    fn test_24_chained_branching() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.scenario.len(), 5);
        assert_eq!(plan.scenario[1].on_success.as_deref(), Some("vip_processing"));
    }

    #[test]
    fn test_25_branching_with_extract_and_validate() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let extract = plan.scenario[0].extract.as_ref().unwrap();
        assert_eq!(extract.get("orderId").unwrap(), "response.body.orderId");
        assert_eq!(extract.get("orderStatus").unwrap(), "response.body.status");
    }

    // =========================================================================
    // Category 6: Parallel Execution (26-30)
    // =========================================================================

    #[test]
    fn test_26_simple_parallel_two_branches() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let parallel = plan.scenario[0].parallel.as_ref().unwrap();
        assert_eq!(parallel.len(), 2);
        assert_eq!(parallel[0][0].name, "track_analytics");
        assert_eq!(parallel[1][0].name, "notify_warehouse");
    }

    #[test]
    fn test_27_parallel_branches_with_multiple_steps() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let parallel = plan.scenario[0].parallel.as_ref().unwrap();
        assert_eq!(parallel[0].len(), 2); // branch 1 has 2 steps
        assert_eq!(parallel[1].len(), 3); // branch 2 has 3 steps
    }

    #[test]
    fn test_28_parallel_three_branches() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let parallel = plan.scenario[0].parallel.as_ref().unwrap();
        assert_eq!(parallel.len(), 3);
    }

    #[test]
    fn test_29_parallel_after_linear_steps() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.scenario.len(), 3);
        assert!(plan.scenario[1].parallel.is_some());
        assert!(plan.scenario[0].parallel.is_none());
    }

    #[test]
    fn test_30_nested_parallel() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let outer = plan.scenario[0].parallel.as_ref().unwrap();
        let inner = outer[0][0].parallel.as_ref().unwrap();
        assert_eq!(inner.len(), 2);
        assert_eq!(inner[0][0].name, "deep_a1");
    }

    // =========================================================================
    // Category 7: Loops and Iterations (31-35)
    // =========================================================================

    #[test]
    fn test_31_simple_count_loop() {
        let yaml = r#"
test: count_loop
scenario:
  - name: repeat_call
    loop:
      count: 5
    do:
      - name: hit_api
        protocol: https
        endpoint: /api/ping
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let loop_config = plan.scenario[0].loop_config.as_ref().unwrap();
        assert!(loop_config.count.is_some());
    }

    #[test]
    fn test_32_collection_loop() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let loop_config = plan.scenario[0].loop_config.as_ref().unwrap();
        assert_eq!(loop_config.over.as_deref(), Some("selected_product"));
        assert_eq!(loop_config.from.as_deref(), Some("${$context.products}"));
    }

    #[test]
    fn test_33_loop_with_count_and_collection() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let lc = plan.scenario[0].loop_config.as_ref().unwrap();
        assert!(lc.count.is_some());
        assert!(lc.over.is_some());
    }

    #[test]
    fn test_34_loop_with_multi_step_body() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        match plan.scenario[0].do_steps.as_ref().unwrap() {
            DoBlock::Multiple(steps) => assert_eq!(steps.len(), 3),
            _ => panic!("Expected DoBlock::Multiple"),
        }
    }

    #[test]
    fn test_35_nested_loops() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        // Outer loop
        let outer_do = plan.scenario[0].do_steps.as_ref().unwrap();
        match outer_do {
            DoBlock::Multiple(steps) => {
                let inner = &steps[0];
                assert!(inner.loop_config.is_some());
                assert!(inner.do_steps.is_some());
            }
            _ => panic!("Expected DoBlock::Multiple for outer"),
        }
    }

    // =========================================================================
    // Category 8: Context & Extraction (36-40)
    // =========================================================================

    #[test]
    fn test_36_basic_extract() {
        let yaml = r#"
test: extract_test
scenario:
  - name: login
    protocol: https
    endpoint: /api/login
    method: POST
    extract:
      authToken: response.body.token
      userId: response.body.user.id
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let extract = plan.scenario[0].extract.as_ref().unwrap();
        assert_eq!(extract.len(), 2);
        assert_eq!(extract.get("authToken").unwrap(), "response.body.token");
    }

    #[test]
    fn test_37_chained_extraction_across_steps() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.scenario.len(), 3);
        for step in &plan.scenario {
            assert!(step.extract.is_some());
        }
    }

    #[test]
    fn test_38_extract_with_validation() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let step = &plan.scenario[0];
        assert!(step.extract.is_some());
        assert!(step.validate.is_some());
        assert_eq!(step.validate.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_39_track_metric() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let metric = plan.scenario[0].track_metric.as_ref().unwrap();
        assert_eq!(metric.name, "payment_approval_rate");
        assert_eq!(metric.dimensions.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_40_context_interpolation_in_body() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.scenario.len(), 2);
        assert!(plan.scenario[0].extract.is_some());
    }

    // =========================================================================
    // Category 9: Retries & Synchronization (41-45)
    // =========================================================================

    #[test]
    fn test_41_simple_retry() {
        let yaml = r#"
test: retry_test
scenario:
  - name: flaky_call
    protocol: https
    endpoint: /api/flaky
    retry:
      max_attempts: 3
      backoff: exponential
      delay: 2s
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let retry = plan.scenario[0].retry.as_ref().unwrap();
        assert_eq!(retry.max_attempts, 3);
        assert_eq!(retry.backoff.as_deref(), Some("exponential"));
        assert_eq!(retry.delay.as_deref(), Some("2s"));
    }

    #[test]
    fn test_42_retry_linear_backoff() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let retry = plan.scenario[0].retry.as_ref().unwrap();
        assert_eq!(retry.backoff.as_deref(), Some("linear"));
        assert_eq!(retry.max_attempts, 5);
    }

    #[test]
    fn test_43_wait_for_sync() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let wait = plan.scenario[1].wait_for.as_ref().unwrap();
        assert_eq!(wait.len(), 2);
        assert_eq!(wait[0], "task_a");
        assert_eq!(plan.scenario[1].timeout.as_deref(), Some("30s"));
        assert_eq!(plan.scenario[1].on_timeout.as_deref(), Some("handle_timeout"));
    }

    #[test]
    fn test_44_async_fire_and_forget() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.scenario[1].r#async, Some(true));
        assert!(plan.scenario[0].r#async.is_none());
    }

    #[test]
    fn test_45_within_simple_and_structured() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        // Simple within
        match plan.scenario[1].within.as_ref().unwrap() {
            WithinConfig::Duration(d) => assert_eq!(d, "5s"),
            _ => panic!("Expected WithinConfig::Duration"),
        }
        // Structured within
        match plan.scenario[2].within.as_ref().unwrap() {
            WithinConfig::Structured(w) => {
                assert_eq!(w.duration, "5s");
                assert_eq!(w.since.as_deref(), Some("place_order"));
            }
            _ => panic!("Expected WithinConfig::Structured"),
        }
    }

    // =========================================================================
    // Category 10: Complex Integration Scenarios (46-50)
    // =========================================================================

    #[test]
    fn test_46_full_ecommerce_flow() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.test, "ecommerce_checkout");
        assert_eq!(plan.scenario.len(), 5);
        // Verify config
        let config = plan.config.as_ref().unwrap();
        assert_eq!(config.virtual_users, Some(10));
        // Verify data sources
        assert_eq!(plan.data_sources.as_ref().unwrap().len(), 1);
        // Verify loop
        assert!(plan.scenario[2].loop_config.is_some());
        // Verify parallel
        assert!(plan.scenario[4].parallel.is_some());
        // Verify outputs
        assert_eq!(plan.outputs.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_47_six_level_deep_nesting() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        // Navigate to level 6
        let l1 = &plan.scenario[0];
        let l2 = &l1.parallel.as_ref().unwrap()[0][0]; // level2_branch_a
        assert_eq!(l2.name, "level2_branch_a");
        assert!(l2.loop_config.is_some());

        match l2.do_steps.as_ref().unwrap() {
            DoBlock::Multiple(steps) => {
                let l3 = &steps[0]; // level3_loop_body
                assert_eq!(l3.name, "level3_loop_body");
                let l4 = &l3.parallel.as_ref().unwrap()[0][0]; // level4_parallel_a
                assert_eq!(l4.name, "level4_parallel_a");
                match l4.do_steps.as_ref().unwrap() {
                    DoBlock::Multiple(inner) => {
                        let l5 = &inner[0];
                        let l6 = &l5.parallel.as_ref().unwrap()[0][0];
                        assert_eq!(l6.name, "level6_leaf_a");
                    }
                    _ => panic!("Expected DoBlock::Multiple at level 5"),
                }
            }
            _ => panic!("Expected DoBlock::Multiple at level 3"),
        }
    }

    #[test]
    fn test_48_module_use_and_functions() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        // Verify imports
        let imports = plan.imports.as_ref().unwrap();
        assert_eq!(imports.len(), 2);
        // Verify functions
        let functions = plan.functions.as_ref().unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].language, "rust");
        // Verify module use
        assert_eq!(plan.scenario[0].use_module.as_deref(), Some("authenticate_user"));
        assert_eq!(plan.scenario[0].outputs_to.as_deref(), Some("auth_context"));
        // Verify transform
        let transform = plan.scenario[1].transform.as_ref().unwrap();
        assert_eq!(transform[0].function, "calculate_tax");
        assert_eq!(transform[0].output_to, "tax_amount");
    }

    #[test]
    fn test_49_scripting_and_inheritance() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        // Verify inheritance
        assert_eq!(plan.base.as_deref(), Some("api_test_base.yaml"));
        assert_eq!(plan.extends.as_deref(), Some("base"));
        // Verify scripting
        assert!(plan.scenario[0].script.is_some());
        assert_eq!(plan.scenario[0].runtime.as_deref(), Some("python"));
        // Verify global validation
        let rules = plan.validation.as_ref().unwrap().business_rules.as_ref().unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name, "order_completion_rate");
        assert_eq!(rules[0].threshold, Some(0.95));
    }

    #[test]
    fn test_50_mega_scenario_all_features() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();

        // Top-level structure
        assert_eq!(plan.test, "mega_integration");
        assert!(plan.description.is_some());
        assert_eq!(plan.imports.as_ref().unwrap().len(), 1);
        assert_eq!(plan.functions.as_ref().unwrap().len(), 1);
        assert_eq!(plan.data_sources.as_ref().unwrap().len(), 3);
        assert_eq!(plan.outputs.as_ref().unwrap().len(), 2);

        // Config
        let config = plan.config.as_ref().unwrap();
        assert_eq!(config.virtual_users, Some(50));
        match config.ramp_up.as_ref().unwrap() {
            RampUp::Structured(r) => assert_eq!(r.steps, Some(10)),
            _ => panic!("Expected structured ramp_up"),
        }

        // Scenario structure
        assert_eq!(plan.scenario.len(), 8);

        // Module use
        assert!(plan.scenario[0].use_module.is_some());

        // Extract & validate
        assert!(plan.scenario[1].extract.is_some());
        assert!(plan.scenario[1].validate.is_some());

        // Loop
        assert!(plan.scenario[2].loop_config.is_some());
        assert!(plan.scenario[2].do_steps.is_some());

        // Transform & track_metric
        assert!(plan.scenario[3].transform.is_some());
        assert!(plan.scenario[3].track_metric.is_some());

        // Parallel with 3 branches
        let parallel = plan.scenario[4].parallel.as_ref().unwrap();
        assert_eq!(parallel.len(), 3);

        // Branching
        assert!(plan.scenario[5].on_success.is_some());
        assert!(plan.scenario[5].on_failure.is_some());

        // Retry
        assert!(plan.scenario[6].retry.is_some());

        // Global validation
        let rules = plan.validation.as_ref().unwrap().business_rules.as_ref().unwrap();
        assert_eq!(rules.len(), 2);
    }

    // =========================================================================
    // Category 11: Step-Level Mocking (51-60) — §7.6
    // =========================================================================

    #[test]
    fn test_51_simple_http_mock() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let config = plan.config.as_ref().unwrap();
        assert_eq!(config.mock_strategy.as_deref(), Some("auto"));
        let mock = plan.scenario[0].mock.as_ref().unwrap();
        let response = mock.response.as_ref().unwrap();
        assert_eq!(response.status, Some(200));
        assert!(response.headers.is_some());
        assert!(response.body.is_some());
    }

    #[test]
    fn test_52_mock_with_dynamic_response_and_contract() {
        let yaml = r#"
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let mock = plan.scenario[0].mock.as_ref().unwrap();
        // Contract schemas
        assert!(mock.request_schema.is_some());
        assert!(mock.response_schema.is_some());
        // Response
        let response = mock.response.as_ref().unwrap();
        assert_eq!(response.status, Some(201));
        assert!(response.body.is_some());
    }

    #[test]
    fn test_53_kafka_cdc_mock_with_delay() {
        let yaml = r#"
test: kafka_mock
scenario:
  - name: place_order
    protocol: https
    endpoint: /api/orders
    method: POST
    mock:
      response:
        status: 201
        body:
          orderId: "ord-123"
    extract:
      orderId: response.body.orderId
  - name: observe_cdc
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        // Kafka step mock
        let mock = plan.scenario[1].mock.as_ref().unwrap();
        let response = mock.response.as_ref().unwrap();
        assert!(response.message.is_some());
        assert_eq!(response.delay.as_deref(), Some("500ms"));
    }

    #[test]
    fn test_54_mock_glob_endpoint_and_method_list() {
        let yaml = r#"
test: mock_with_details
scenario:
  - name: get_user_by_id
    protocol: https
    endpoint: /api/users/456
    method: GET
    mock:
      response:
        status: 200
        body:
          id: "456"
          name: "${$faker.name()}"
        latency: "${$random.int(50, 200)}ms"
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let mock = plan.scenario[0].mock.as_ref().unwrap();
        let response = mock.response.as_ref().unwrap();
        assert_eq!(response.status, Some(200));
        assert!(response.latency.is_some());
    }

    #[test]
    fn test_55_stateful_mock_sequence() {
        let yaml = r#"
test: stateful_mock
scenario:
  - name: poll_status
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let mock = plan.scenario[0].mock.as_ref().unwrap();
        let responses = mock.responses.as_ref().unwrap();
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0].status, Some(200));
        assert_eq!(responses[2].status, Some(200));
        assert_eq!(mock.on_exhausted.as_deref(), Some("repeat_last"));
    }

    #[test]
    fn test_56_failure_injection_mock() {
        let yaml = r#"
test: failure_injection
scenario:
  - name: call_flaky
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
          quantity: 42
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
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let mock = plan.scenario[0].mock.as_ref().unwrap();
        let behavior = mock.behavior.as_ref().unwrap();
        assert_eq!(behavior.error_rate, Some(0.3));
        assert_eq!(behavior.timeout_rate, Some(0.05));
        // Error response
        let err_resp = behavior.error_response.as_ref().unwrap();
        assert_eq!(err_resp.status, Some(503));
        // Latency distribution
        match behavior.latency.as_ref().unwrap() {
            MockLatency::Distribution(d) => {
                assert_eq!(d.distribution, "normal");
                assert_eq!(d.mean.as_deref(), Some("200ms"));
                assert_eq!(d.stddev.as_deref(), Some("50ms"));
            }
            _ => panic!("Expected MockLatency::Distribution"),
        }
    }

    #[test]
    fn test_57_database_mock() {
        let yaml = r#"
test: db_mock
scenario:
  - name: query_orders
    protocol: database
    connection: postgres://orders-db
    query: "SELECT status, amount FROM orders WHERE id = 'ord-123'"
    mock:
      response:
        rows:
          - status: "completed"
            amount: 99.99
          - status: "pending"
            amount: 49.50
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        let mock = plan.scenario[0].mock.as_ref().unwrap();
        let response = mock.response.as_ref().unwrap();
        let rows = response.rows.as_ref().unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_58_mock_strategy_in_config() {
        let yaml_auto = r#"
test: strategy_auto
config:
  mock_strategy: auto
  timeout: 5m
scenario:
  - name: step1
    protocol: https
    endpoint: /api/test
    mock:
      response:
        status: 200
        body: { ok: true }
"#;
        let plan = TestPlan::from_yaml_str(yaml_auto).unwrap();
        assert_eq!(plan.config.as_ref().unwrap().mock_strategy.as_deref(), Some("auto"));

        let yaml_required = r#"
test: strategy_required
config:
  mock_strategy: required
scenario:
  - name: step1
    protocol: https
    endpoint: /api/test
    mock:
      response:
        status: 200
        body: { ok: true }
"#;
        let plan2 = TestPlan::from_yaml_str(yaml_required).unwrap();
        assert_eq!(plan2.config.as_ref().unwrap().mock_strategy.as_deref(), Some("required"));

        let yaml_disabled = r#"
test: strategy_disabled
config:
  mock_strategy: disabled
scenario:
  - name: step1
    protocol: https
    endpoint: /api/test
"#;
        let plan3 = TestPlan::from_yaml_str(yaml_disabled).unwrap();
        assert_eq!(plan3.config.as_ref().unwrap().mock_strategy.as_deref(), Some("disabled"));
        assert!(plan3.scenario[0].mock.is_none());
    }

    #[test]
    fn test_59_multiple_mocks_across_protocols() {
        let yaml = r#"
test: multi_protocol_mocks
scenario:
  - name: http_call
    protocol: https
    endpoint: /api/users
    method: GET
    mock:
      response:
        status: 200
        body:
          users: []
  - name: kafka_observe
    protocol: kafka
    topic: events.users
    mode: observe
    expect:
      message.type: USER_CREATED
    within: 3s
    mock:
      response:
        message:
          type: USER_CREATED
          data:
            id: "u-001"
        delay: 100ms
  - name: db_check
    protocol: database
    connection: postgres://users-db
    query: "SELECT count(*) FROM users"
    mock:
      response:
        rows:
          - count: 42
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();
        assert_eq!(plan.scenario.len(), 3);
        // All three steps have mocks
        for step in &plan.scenario {
            assert!(step.mock.is_some(), "Step '{}' should have a mock", step.name);
        }
        // HTTP mock
        assert_eq!(plan.scenario[0].mock.as_ref().unwrap().response.as_ref().unwrap().status, Some(200));
        // Kafka mock
        assert!(plan.scenario[1].mock.as_ref().unwrap().response.as_ref().unwrap().message.is_some());
        // DB mock
        assert!(plan.scenario[2].mock.as_ref().unwrap().response.as_ref().unwrap().rows.is_some());
    }

    #[test]
    fn test_60_full_integration_with_mocks() {
        let yaml = r#"
test: fully_mocked_checkout
description: "Complete e-commerce checkout with all services mocked"
config:
  mock_strategy: auto
  virtual_users: 5
  duration: 2m
  timeout: 5m
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
    validate:
      - "response.status == 200"
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
  - name: add_items_loop
    loop:
      count: 2
      over: product
      from: "${$context.products}"
    do:
      - name: add_to_cart
        protocol: https
        endpoint: /api/cart
        method: POST
        body:
          productId: "${$product.id}"
          quantity: 1
        mock:
          response:
            status: 200
            body:
              cartItemId: "${$faker.uuid()}"
  - name: place_order
    protocol: https
    endpoint: /api/orders
    method: POST
    body:
      userId: "${$context.userId}"
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
  - name: verify_notifications
    parallel:
      - - name: verify_order_db
          protocol: database
          connection: postgres://orders-db
          query: "SELECT status FROM orders WHERE id = '${$context.orderId}'"
          mock:
            response:
              rows:
                - status: "created"
      - - name: observe_cdc
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
              delay: 200ms
      - - name: send_notification
          protocol: https
          endpoint: /api/notifications
          method: POST
          mock:
            response:
              status: 202
              body:
                queued: true
"#;
        let plan = TestPlan::from_yaml_str(yaml).unwrap();

        // Top-level structure
        assert_eq!(plan.test, "fully_mocked_checkout");
        assert!(plan.description.is_some());
        let config = plan.config.as_ref().unwrap();
        assert_eq!(config.mock_strategy.as_deref(), Some("auto"));
        assert_eq!(config.virtual_users, Some(5));

        // Step 1: login with mock + request_schema
        let login_mock = plan.scenario[0].mock.as_ref().unwrap();
        assert!(login_mock.request_schema.is_some());
        assert_eq!(login_mock.response.as_ref().unwrap().status, Some(200));

        // Step 2: browse with mock
        assert!(plan.scenario[1].mock.is_some());

        // Step 3: loop with mock inside do block
        assert!(plan.scenario[2].loop_config.is_some());
        match plan.scenario[2].do_steps.as_ref().unwrap() {
            DoBlock::Multiple(steps) => {
                assert!(steps[0].mock.is_some());
            }
            _ => panic!("Expected DoBlock::Multiple"),
        }

        // Step 4: order with response_schema
        let order_mock = plan.scenario[3].mock.as_ref().unwrap();
        assert!(order_mock.response_schema.is_some());

        // Step 5: parallel with mocks on each branch
        let parallel = plan.scenario[4].parallel.as_ref().unwrap();
        assert_eq!(parallel.len(), 3);
        // DB mock
        assert!(parallel[0][0].mock.as_ref().unwrap().response.as_ref().unwrap().rows.is_some());
        // Kafka mock
        assert!(parallel[1][0].mock.as_ref().unwrap().response.as_ref().unwrap().message.is_some());
        // HTTP mock
        assert_eq!(parallel[2][0].mock.as_ref().unwrap().response.as_ref().unwrap().status, Some(202));
    }
}
