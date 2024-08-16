use tracing_subscriber::prelude::*;

pub fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    let axiom_layer = tracing_axiom::builder_with_env("pragma-node")?.with_dataset("pragma-node")?.build()?;
    let fmt_layer = tracing_subscriber::fmt::layer().pretty();
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(axiom_layer)
        .try_init()?;

    Ok(())
}
