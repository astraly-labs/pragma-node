use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::prelude::*;

pub fn init_tracing(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let axum_layer = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // axum logs rejections from built-in extractors with the `axum::rejection`
        // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
        "example_tracing_aka_logging=debug,tower_http=debug,axum::rejection=trace".into()
    });
    let axiom_layer = tracing_axiom::builder_with_env(service_name)?
        .with_dataset("pragma-node")?
        .build()?;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .pretty();

    let filter = filter_fn(|metadata| {
        // Filter out hyper logs
        metadata.target() != "hyper" && 
        // You can add more conditions here if needed
        metadata.level() <= &tracing::Level::DEBUG
    });

    tracing_subscriber::registry()
        .with(fmt_layer.with_filter(filter.clone()))
        .with(axiom_layer.with_filter(filter.clone()))
        .with(axum_layer.with_filter(filter))
        .try_init()?;

    Ok(())
}
