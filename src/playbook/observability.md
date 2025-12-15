# Observability

Effective observability is crucial for understanding and debugging LLM applications in production. Rig provides built-in instrumentation using the `tracing` ecosystem, allowing you to track requests, measure performance, and diagnose issues across your AI workflows.

## Observability in LLM-assisted systems
Observability in AI/LLM-assisted systems is a huge component in being able to ensure that a non-deterministic system can still be considered relatively reliable by making sure that metrics like model drift (and accuracy!), guardrail effectiveness and token usage can be easily tracked as well as error rates. Even moreso than traditional systems, it's important for LLM-assisted systems to be instrumented and observable specifically because of the non-deterministic component.

## What does observability mean in Rig?
Generally speaking, observability is the ability to observe a system.

## Overview

Rig's observability approach is relatively minimal and unopinionated. Internally we use the `tracing` crate to emit logs and spans, which you can use however you want and can use any kind of tracing subscriber (via `tracing-subscriber`) or log facade (like `env-logger`), etc, to emit them.

### Instrumentation Levels

Rig uses the following logging conventions:

- **`INFO` level**: Spans marking the start and end of operations
- **`TRACE` level**: Detailed request/response message logs for debugging

## Basic Setup with `tracing-subscriber`

The simplest way to get started is with `tracing-subscriber`'s formatting layer:

```rust
tracing_subscriber::fmt().init();
```

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rig=trace".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Your Rig application code here
}
```

This configuration:
- Sets the default log level to `info`
- Enables `trace` level logging for `rig` to see detailed request/response data
- Uses environment variables (e.g., `RUST_LOG=trace`) to override filter settings

### Customizing Output Format

For JSON-formatted logs (useful in production), `tracing` provides a JSON formatting layer that will automatically output logs in JSON format. See below for a practical usage example:

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt};

fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rig=trace".into())
        )
        .with(fmt::layer().json())
        .init();

    // Your application code
}
```

This is primarily useful when you have a log drain or place where logs (or traces!) are stored, as it allows you to then query the logs easily using `jq`:

```bash
tail -f <logfile> | jq .
```

## OpenTelemetry Integration

For production deployments, you'll typically want to export traces to an external observability platform. Rig integrates seamlessly with OpenTelemetry, allowing you to use your own OTEL collector and route traces to any compatible backend.

### Dependencies

Add these to your `Cargo.toml`:

```toml
[dependencies]
rig-core = "0.1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-opentelemetry = "0.30"
opentelemetry = { version = "0.31", features = ["trace"] }
opentelemetry_sdk = { version = "0.31", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.31", features = ["tonic", "trace"] }
```

### Exporting to an OTEL Collector

Configure Rig to export traces to your OpenTelemetry collector:

```rust
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure OTLP exporter to send to your collector
    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://localhost:4317"); // Your OTEL collector endpoint

    // Create tracer provider
    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(
            sdktrace::config().with_resource(Resource::new(vec![
                opentelemetry::KeyValue::new("service.name", "my-rig-app"),
            ]))
        )
        .install_batch(runtime::Tokio)?;

    // Get tracer
    let tracer = tracer_provider.tracer("my-rig-app");

    // Set up tracing subscriber with OTEL layer
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rig_core=trace".into())
        )
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    // Your Rig application code here
    // ...

    // Shutdown tracer provider on exit
    opentelemetry::global::shutdown_tracer_provider();
    
    Ok(())
}
```

### OTEL Collector Configuration

Your OTEL collector can then route traces to various backends. Example `otel-collector-config.yaml`:

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:

exporters:
  # Export to Jaeger
  otlp/jaeger:
    endpoint: jaeger:4317
    tls:
      insecure: true
  
  # Export to Grafana Tempo
  otlp/tempo:
    endpoint: tempo:4317
    tls:
      insecure: true
  
  # Export to Honeycomb, Datadog, etc.
  otlp/vendor:
    endpoint: api.vendor.io:443
    headers:
      x-api-key: ${VENDOR_API_KEY}

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [otlp/jaeger, otlp/tempo]
```

## Example: Observing a Rig Agent

Here's a complete example showing how instrumentation appears in a Rig application:

```rust
use rig::{completion::Prompt, providers::openai};
use tracing::{info, instrument};

#[instrument(name = "process_user_query")]
async fn process_query(user_input: &str) -> Result<String, Box<dyn std::error::Error>> {
    info!("Processing user query");
    
    let openai_client = openai::Client::from_env();
    
    let agent = openai_client
        .agent("gpt-4")
        .preamble("You are a helpful assistant.")
        .build();

    // This completion call will emit spans automatically
    let response = agent.prompt(user_input).await?;
    
    info!("Query processed successfully");
    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (see setup examples above)
    tracing_subscriber::fmt::init();
    
    let result = process_query("What is Rust?").await?;
    println!("Response: {}", result);
    
    Ok(())
}
```

When running with `RUST_LOG=trace`, you'll see:

- `INFO` spans for the completion operation lifecycle
- `TRACE` logs showing the actual request/response payloads
- Custom spans from your application code (like `process_user_query`)

## Integration with Specific Providers

While Rig doesn't include pre-built subscriber layers for specific vendors, the otel collector approach allows you to send traces to any observability platform:

### Grafana Cloud / Tempo

```yaml
# In your OTEL collector config
exporters:
  otlp/grafana:
    endpoint: tempo-prod-04-prod-us-east-0.grafana.net:443
    headers:
      authorization: Basic ${GRAFANA_CLOUD_TOKEN}
```

### Honeycomb

```yaml
exporters:
  otlp/honeycomb:
    endpoint: api.honeycomb.io:443
    headers:
      x-honeycomb-team: ${HONEYCOMB_API_KEY}
      x-honeycomb-dataset: my-rig-app
```

### Datadog

```yaml
exporters:
  datadog:
    api:
      key: ${DD_API_KEY}
      site: datadoghq.com
```

### Jaeger (Self-Hosted)

```yaml
exporters:
  otlp/jaeger:
    endpoint: localhost:4317
    tls:
      insecure: true
```

## Best Practices

### 1. Use Appropriate Log Levels in Production

Avoid running `trace` level logging in production as it logs full request/response bodies:

```rust
tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| "info".into()) // Only trace for rig_core when needed
```

### 2. Add Custom Spans for Business Logic

Wrap your domain-specific operations in spans for better observability:

```rust
use tracing::instrument;

#[instrument(skip(agent))]
async fn analyze_sentiment(agent: &Agent, text: &str) -> Result<String, Error> {
    agent.prompt(&format!("Analyze sentiment: {}", text)).await
}
```

### 3. Include Contextual Attributes

Add relevant metadata to your spans:

```rust
use tracing::info_span;

let span = info_span!(
    "user_request",
    user_id = %user_id,
    request_type = "completion"
);

let _enter = span.enter();
// Your code here
```

### 4. Handle Errors with Context

Log errors with appropriate context:

```rust
use tracing::error;

match agent.prompt(input).await {
    Ok(response) => Ok(response),
    Err(e) => {
        error!(error = %e, "Failed to get completion");
        Err(e)
    }
}
```

### 5. Sampling in High-Volume Scenarios

For high-throughput applications, configure sampling in your tracer:

```rust
use opentelemetry_sdk::trace::Sampler;

let tracer_provider = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(otlp_exporter)
    .with_trace_config(
        sdktrace::config()
            .with_sampler(Sampler::TraceIdRatioBased(0.1)) // Sample 10% of traces
            .with_resource(resource)
    )
    .install_batch(runtime::Tokio)?;
```

## Troubleshooting

### Not Seeing Traces

1. Verify your filter configuration includes Rig: `RUST_LOG=info,rig_core=trace`
2. Check that your OTEL collector is reachable
3. Ensure `tracing_subscriber::init()` is called before any Rig operations

### Too Verbose Output

1. Set `rig_core` to `info` instead of `trace` to hide request/response bodies
2. Adjust your environment filter: `RUST_LOG=warn,rig_core=info`

### Missing Spans in External Tools

1. Verify your OTEL collector is correctly configured and running
2. Check collector logs for export errors
3. Ensure your tracer provider is properly shut down on application exit

## Additional Resources

- [tracing documentation](https://docs.rs/tracing/)
- [tracing-subscriber documentation](https://docs.rs/tracing-subscriber/)
- [OpenTelemetry Rust documentation](https://docs.rs/opentelemetry/)
- [OTEL Collector documentation](https://opentelemetry.io/docs/collector/)
