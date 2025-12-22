use observability::process_query;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tracing::Level;
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
    let _ = provider.shutdown();

    Ok(())
}
