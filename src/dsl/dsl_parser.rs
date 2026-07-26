// =============================================================================
// Evento DSL Parser — Comprehensive AST Types
// Covers all features specified in DSL_SPECIFICATION.md
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};
use std::fs;

// =============================================================================
// Top-Level: TestPlan
// DSL Spec §2.2 — Top-Level Elements
// =============================================================================

/// The root AST node representing a complete eDSL test document.
///
/// The spec (§2.2) defines `test` as the REQUIRED identifier. We also accept
/// `name` as an alias for backward compatibility, but `test` is canonical.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestPlan {
    /// The unique identifier for the test suite (§2.2). Primary key.
    #[serde(alias = "name")]
    pub test: String,

    /// Human-readable summary of the test (§2.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// References to external modules, subflows, or scripts (§2.2, §11.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Vec<String>>,

    /// Execution configuration (§3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<Config>,

    /// External data definitions (§4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_sources: Option<Vec<DataSource>>,

    /// Custom reusable functions (§9.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<Function>>,

    /// The execution graph containing test steps (§5). REQUIRED.
    pub scenario: Vec<Step>,

    /// Global business rule assertions evaluated after scenario completion (§8.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ValidationBlock>,

    /// Result formatting and export destinations (§10.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<Output>>,

    /// Metadata and annotations (§2.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// Base configuration file for inheritance (§11.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,

    /// Extends directive for configuration inheritance (§11.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
}

impl TestPlan {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file {:?}", path.as_ref()))?;
        Self::from_yaml_str(&content)
    }

    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        let plan: TestPlan = serde_yaml::from_str(yaml)
            .with_context(|| "Failed to parse YAML test plan")?;
        Ok(plan)
    }
}

// =============================================================================
// Metadata (§2.3)
// =============================================================================

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

// =============================================================================
// Config (§3)
// =============================================================================

/// Test execution configuration.
///
/// Controls mode, concurrency, ramp-up strategy, and global timeouts.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// Execution mode: realtime (default), replay, scheduled (§3.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,

    /// For replay mode: path to the traffic log file (§3.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// For replay mode: speed multiplier (§3.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,

    /// For scheduled mode: CRON expression (§3.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron: Option<String>,

    /// Total test duration (§3.2). E.g., "5m", "30s".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,

    /// Maximum number of concurrent Virtual Users (§3.2). Default: 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_users: Option<u32>,

    /// Strategy for spinning up VUs (§3.2). Can be a duration string or object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ramp_up: Option<RampUp>,

    /// Maximum allowed time for the entire scenario (§3.3). Default: "1m".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// Default timeout for individual steps (§3.3). Default: "10s".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_timeout: Option<String>,

    /// Mock resolution strategy (§3.4): auto, disabled, required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_strategy: Option<String>,
}

/// Ramp-up strategy (§3.2).
///
/// Can be specified as either:
/// - A simple duration string: `"30s"` (implies linear strategy)
/// - A structured object: `{ strategy: step, duration: 30s, steps: 5 }`
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum RampUp {
    /// Simple duration string implying linear ramp-up.
    Duration(String),
    /// Structured ramp-up with explicit strategy.
    Structured(RampUpConfig),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RampUpConfig {
    /// Ramp-up strategy: linear, step, normal_distribution (§3.2).
    pub strategy: String,
    /// Duration over which to ramp up.
    pub duration: String,
    /// Number of steps (for step strategy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,
}

// =============================================================================
// Data Sources (§4)
// =============================================================================

/// External data source definition (§4.2, §4.3).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DataSource {
    /// Unique name for this data source.
    pub name: String,

    /// Feed type: database, csv, json (§4.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// For faker-based generation (§4.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,

    /// URI pointing to the data source (§4.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,

    /// SQL query or similar (§4.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// File path for csv/json sources (§4.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// How records are handed to VUs: sequential, random, shuffle (§4.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<String>,

    /// Whether to cache the data source (§4.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<bool>,

    /// For faker generator: field definitions (§4.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<HashMap<String, String>>,
}

// =============================================================================
// Step — The Core Execution Node (§5, §6, §7, §8, §9)
// =============================================================================

/// A single unit of work within a scenario.
///
/// Steps can represent protocol calls, parallel branches, loops,
/// synchronization points, module invocations, or script executions.
/// This struct covers ALL step variants from the spec.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Step {
    /// Step identifier. REQUIRED for all steps.
    pub name: String,

    /// Human-readable description of what this step does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // --- Protocol Execution Fields (§7) ---

    /// Protocol driver: https, kafka, grpc, database, etc. (§7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,

    /// URL path or full URL for HTTP steps (§7.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// HTTP method: GET, POST, PUT, DELETE, PATCH (§7.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// HTTP headers (§7.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,

    /// Request payload — flexible to support JSON, XML, etc. (§7.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_yaml::Value>,

    // --- Kafka Fields (§7.2) — deferred for detailed impl ---

    /// Kafka topic name (§7.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,

    /// Kafka message definition (§7.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_yaml::Value>,

    /// Kafka/gRPC mode: e.g., "observe" for CDC consumers (§7.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,

    /// Expected message matching criteria for observe mode (§7.2, §8.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect: Option<HashMap<String, serde_yaml::Value>>,

    /// Temporal assertion: duration or structured with `since` (§8.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub within: Option<WithinConfig>,

    // --- gRPC Fields (§7.3) ---

    /// Fully qualified gRPC service method (§7.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,

    // --- Database Fields (§7.4) ---

    /// Database connection URI (§7.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,

    /// SQL query (§7.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    // --- Control Flow (§5) ---

    /// Fire-and-forget flag (§5.5). If true, execution continues immediately.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#async: Option<bool>,

    /// Conditional branching: boolean expression assertions (§5.2, §8.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<Vec<String>>,

    /// Step to execute on validation success (§5.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_success: Option<String>,

    /// Step to execute on validation failure (§5.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,

    /// Parallel execution branches (§5.3). Each branch is a subgraph (list of steps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel: Option<Vec<Vec<Step>>>,

    /// Loop configuration (§5.4).
    #[serde(rename = "loop")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_config: Option<LoopConfig>,

    /// Loop body: the subgraph of steps to execute per iteration (§5.4).
    /// Supports a single step or a list of steps (complete subgraph).
    #[serde(rename = "do")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_steps: Option<DoBlock>,

    /// Synchronization barrier: list of step names to wait for (§5.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_for: Option<Vec<String>>,

    /// Step-level timeout override (§5.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// On-timeout branching directive (§5.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_timeout: Option<String>,

    /// Retry configuration (§5.6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryConfig>,

    // --- State Management (§6) ---

    /// Extract variables from protocol response into context (§6.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<HashMap<String, String>>,

    // --- Metrics (§10.2) ---

    /// Custom business metric tracking (§10.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_metric: Option<TrackMetric>,

    // --- Scripting (§9) ---

    /// Inline script content (§9.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,

    /// Script runtime: python, rust, wasm (§9.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,

    /// Custom function transformation pipeline (§9.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<Vec<Transform>>,

    // --- Modularity (§11) ---

    /// Module to invoke (§11.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_module: Option<String>,

    /// Input parameters for module invocation (§11.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<HashMap<String, serde_yaml::Value>>,

    /// Context key to store module outputs (§11.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs_to: Option<String>,

    // --- Mocking (§7.6) ---

    /// Step-level mock definition (§7.6). Provides synthetic responses
    /// when the target service is unavailable or when running in mock mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock: Option<MockConfig>,
}

// =============================================================================
// Loop Configuration (§5.4)
// =============================================================================

/// Loop control parameters.
///
/// Supports numeric iteration (`count`) and collection iteration (`over`/`from`).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoopConfig {
    /// Execute exactly N times. Can be a number or templated expression (§5.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<serde_yaml::Value>,

    /// Iterator variable name for collection loops (§5.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub over: Option<String>,

    /// Source collection expression for iteration (§5.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

// =============================================================================
// Do Block (§5.4) — Loop Body
// =============================================================================

/// The body of a loop. Supports a single step or a list of steps (complete subgraph).
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum DoBlock {
    /// A single step as the loop body.
    Single(Box<Step>),
    /// Multiple steps forming a subgraph as the loop body.
    Multiple(Vec<Step>),
    /// Inline step fields (protocol, endpoint, etc.) without a name wrapper.
    Inline(Box<InlineStepBody>),
}

/// Represents inline step fields used directly inside a `do` block
/// without wrapping in a named step.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InlineStepBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_yaml::Value>,
}

// =============================================================================
// Retry Configuration (§5.6)
// =============================================================================

/// Retry strategy for transient failures (§5.6).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,

    /// Backoff strategy: linear, exponential, constant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff: Option<String>,

    /// Initial delay between retries. E.g., "2s".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,
}

// =============================================================================
// Within Configuration (§8.3) — Temporal Assertions
// =============================================================================

/// Temporal constraint for async assertions.
///
/// Can be a simple duration string or structured with explicit `since`.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum WithinConfig {
    /// Simple duration string: "5s".
    Duration(String),
    /// Structured: { duration: "5s", since: "step_name" }.
    Structured(WithinStructured),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WithinStructured {
    pub duration: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
}

// =============================================================================
// Track Metric (§10.2)
// =============================================================================

/// Custom business metric tracking (§10.2).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TrackMetric {
    /// Metric name.
    pub name: String,

    /// Metric value expression.
    pub value: String,

    /// Dimensions for slicing the metric.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<HashMap<String, String>>,
}

// =============================================================================
// Scripting & Functions (§9)
// =============================================================================

/// Reusable function definition (§9.2).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Function {
    /// Function identifier.
    pub name: String,

    /// Implementation language: rust, python, wasm.
    pub language: String,

    /// Path to the source file.
    pub source: String,
}

/// Transform pipeline step (§9.2).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Transform {
    /// Input expression.
    pub input: String,

    /// Function to apply.
    pub function: String,

    /// Additional parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_yaml::Value>>,

    /// Context key to store the result.
    pub output_to: String,
}

// =============================================================================
// Validation Block (§8.2) — Global Business Rules
// =============================================================================

/// Global validation block evaluated after scenario completion (§8.2).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ValidationBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_rules: Option<Vec<BusinessRule>>,
}

/// A single business rule assertion (§8.2).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BusinessRule {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    pub actual: String,
}

// =============================================================================
// Outputs (§10.3) — Metric Exporting
// =============================================================================

/// Output/export destination for metrics (§10.3).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Output {
    /// Export format: prometheus, datadog, ai_insights, json, etc.
    pub format: String,

    /// Target endpoint for push-based exports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Output file path for file-based exports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,

    /// API key for authenticated exports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

// =============================================================================
// Mock Configuration (§7.6) — Step-Level Mocking
// =============================================================================

/// Step-level mock definition (§7.6.1).
///
/// Provides synthetic responses for a step when the target service is
/// unavailable or when running in mock mode. Supports static responses,
/// dynamic generation, stateful sequences, failure injection, and
/// request/response contract schemas.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MockConfig {
    /// Single response definition (§7.6.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<MockResponse>,

    /// Ordered sequence of responses for stateful simulation (§7.6.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responses: Option<Vec<MockResponse>>,

    /// Behavior when `responses` list is exhausted (§7.6.3).
    /// Values: repeat_last (default), cycle, error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_exhausted: Option<String>,

    /// Advanced failure injection controls (§7.6.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<MockBehavior>,

    /// JSON Schema for request contract validation (§7.6.6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_schema: Option<serde_yaml::Value>,

    /// JSON Schema for response contract validation (§7.6.6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_yaml::Value>,
}

/// Mock response definition (§7.6.2).
///
/// Supports HTTP (status/headers/body), Kafka (message), and
/// Database (rows) response shapes. All fields support eDSL
/// template interpolation including `$request.*` and `$mock.call_count`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MockResponse {
    /// HTTP status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,

    /// HTTP response headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,

    /// Response body — supports templates (§7.6.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_yaml::Value>,

    /// Simulated processing latency. E.g., "200ms", "${$random.int(50,200)}ms".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency: Option<String>,

    /// Simulated delay before response (for Kafka observe mocks). E.g., "500ms".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,

    /// Kafka mock: synthetic message content (§7.6.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_yaml::Value>,

    /// Database mock: synthetic result rows (§7.6.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<serde_yaml::Value>>,
}

/// Mock failure injection behavior (§7.6.4).
///
/// Controls probabilistic error injection, latency simulation,
/// and timeout simulation for resilience testing.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MockBehavior {
    /// Fraction of calls that return error_response (0.0–1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_rate: Option<f64>,

    /// Response returned on injected errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_response: Option<MockResponse>,

    /// Latency simulation: fixed string or distribution object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency: Option<MockLatency>,

    /// Fraction of calls that simulate a complete timeout (0.0–1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_rate: Option<f64>,
}

/// Mock latency configuration (§7.6.4).
///
/// Can be a fixed duration string or a statistical distribution.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum MockLatency {
    /// Fixed duration string: "200ms".
    Fixed(String),
    /// Statistical distribution: normal, uniform, etc.
    Distribution(MockLatencyDistribution),
}

/// Mock latency distribution parameters (§7.6.4).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MockLatencyDistribution {
    /// Distribution type: normal, uniform.
    pub distribution: String,
    /// Mean value (for normal distribution). E.g., "200ms".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean: Option<String>,
    /// Standard deviation (for normal distribution). E.g., "50ms".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stddev: Option<String>,
    /// Minimum value (for uniform distribution). E.g., "100ms".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    /// Maximum value (for uniform distribution). E.g., "500ms".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
}

// =============================================================================
// Module Definition (§11.1) — For imported module files
// =============================================================================

/// A standalone module file (§11.1).
/// These files define `module` instead of `test`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModuleDefinition {
    /// Module identifier.
    pub module: String,

    /// Expected input parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,

    /// Output variables produced by this module.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<String>>,

    /// The module's scenario steps.
    pub scenario: Vec<Step>,
}

impl ModuleDefinition {
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        let module: ModuleDefinition = serde_yaml::from_str(yaml)
            .with_context(|| "Failed to parse module YAML")?;
        Ok(module)
    }
}
