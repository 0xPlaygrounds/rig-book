# Observability

Effective observability is crucial for understanding and debugging LLM applications in production. Rig provides built-in instrumentation using the `tracing` ecosystem, allowing you to track requests, measure performance, and diagnose issues across your AI workflows.

The complete codebase for this section is split up into two separate binary examples, with a separate folder for the accompanying OpenTelemetry files:
- [Basic observability example](https://github.com/0xPlaygrounds/rig-book/blob/main/snippets/observability/src/bin/basic.rs)
- [OpenTelemetry based example](https://github.com/0xPlaygrounds/rig-book/blob/main/snippets/observability/src/bin/otel.rs)
- [OpenTelemetry collector YAML and Dockerfile](https://github.com/0xPlaygrounds/rig-book/tree/main/snippets/observability/otel)

## Observability in LLM-assisted systems
Observability in AI/LLM-assisted systems is a huge component in being able to ensure that a non-deterministic system can still be considered relatively reliable by making sure that metrics like model drift (and accuracy!), guardrail effectiveness and token usage can be easily tracked as well as error rates. Even moreso than traditional systems, it's important for LLM-assisted systems to be instrumented and observable specifically because of the non-deterministic component.

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
use rig::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::openai,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{info, instrument};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rig=trace".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    let response = process_query("Hello world!").await.unwrap();

    println!("Response: {response}");

    Ok(())
}

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
```

When running this example, you'll see:

- `INFO` spans for the completion operation lifecycle
- `TRACE` logs showing the actual request/response payloads
- Custom spans from your application code (like `process_user_query`)

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

This is primarily useful when you have a local log drain or place where logs (or traces!) are stored for debugging, as it allows you to then query the logs easily using `jq`:

```bash
tail -f <logfile> | jq .
```

## OpenTelemetry Integration

For production deployments, you'll typically want to export traces to an external observability platform. Rig integrates seamlessly with OpenTelemetry, allowing you to use your own [Otel collector](https://opentelemetry.io/docs/collector/) and route traces to any compatible backend.

This example will utilise the OTel collector as it can easily be used to send your traces/spans and logs anywhere you'd like.

### Dependencies

Add the following dependencies to your `Cargo.toml` - the listed dependencies below (besides `rig-core`) are all required to make this work:

```toml
[dependencies]
rig-core = "0.27.0"
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
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
        .build()?;
    // Create a new OpenTelemetry trace pipeline that prints to stdout
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::builder().with_service_name("rig-service").build())
        .build();
    let tracer = provider.tracer("example");

    // Create a tracing layer with the configured tracer
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let filter_layer = tracing_subscriber::filter::EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    // add a `fmt` layer that prettifies the logs/spans that get outputted to `stdout`
    let fmt_layer = tracing_subscriber::fmt::layer().pretty();

    // Create a multi-layer subscriber that filters for given traces,
    // prettifies the logs/spans and then sends them to the OTel collector.
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    let response = process_query("Hello world!").await.unwrap();

    println!("Response: {response}");

    // Shutdown tracer provider on exit
    opentelemetry::global::shutdown_tracer_provider();
    
    Ok(())
}
```

This should output all traces to your program's stdout, *as well as* sending it to your OTel collector which will then transform the spans as required and send them along to Langfuse.

### OTEL Collector Configuration

Your OTEL collector can then route traces to various backends. Below is an example YAML file which you might use - for the purposes of this example we'll be using [Langfuse](https://langfuse.com/) as they are a relatively well known service provider for LLM-related observability that can additionally be self-hosted:

```yaml
receivers:
  otlp:
    protocols:
      http:
        endpoint: 0.0.0.0:4318

processors:
  transform:
    trace_statements:
      - context: span
        statements:
          # Rename span if it's "invoke_agent" and has an agent attribute
          # Theoretically this can be left out, 
          - set(name, attributes["gen_ai.agent.name"]) where name == "invoke_agent" and attributes["gen_ai.agent.name"] != nil

exporters:
  debug:
    verbosity: detailed
  otlphttp/langfuse:
    endpoint: "https://cloud.langfuse.com/api/public/otel"
    headers:
      # Langfuse uses basic auth, in the form of username:password.
      # In this case your username is your Langfuse public key and the password is your Langfuse secret key.
      Authorization: "Basic ${AUTH_STRING}"

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [transform]
      exporters: [otlphttp/langfuse, debug]
```

To actually use this file, you would want to write your own Dockerfile that pulls the OTel collector image:

```bash
# Start from the official OpenTelemetry Collector Contrib image
FROM otel/opentelemetry-collector-contrib:0.135.0

# Copy your local config into the container
# Replace `config.yaml` with your actual filename if different
COPY ./config.yaml /etc/otelcol-contrib/config.yaml
```

You can then build this image with `docker build -t <some-tag-name> -f <dockerfile-filename> .` where `<some-tag-name>` is whatever name you want to give to the image.

## Integration with Specific Providers

While Rig doesn't include pre-built subscriber layers for specific vendors, the otel collector approach allows you to send traces to any observability platform. This is arguably a much more flexible way to set up telemetry, although it does have some set-up required.

If you're interested in tracing subscriber layer integrations for specific integrations, please open a feature request issue on our [GitHub repo!](https://github.com/0xPlaygrounds/rig/issues)


## Additional Resources

- [tracing documentation](https://docs.rs/tracing/)
- [tracing-subscriber documentation](https://docs.rs/tracing-subscriber/)
- [OpenTelemetry Rust documentation](https://docs.rs/opentelemetry/)
- [OTEL Collector documentation](https://opentelemetry.io/docs/collector/)
