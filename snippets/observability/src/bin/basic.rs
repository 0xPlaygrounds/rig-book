use observability::process_query;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rig=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let response = process_query("Hello world!").await.unwrap();

    println!("Response: {response}");

    Ok(())
}
