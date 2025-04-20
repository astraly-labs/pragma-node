use clap::Parser;
use std::sync::LazyLock;

pub(crate) static CONFIG: LazyLock<Ingestor> = LazyLock::new(load_configuration);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Ingestor {
    /// Number of consumers to run
    #[arg(long, env = "NUM_CONSUMERS", default_value = "10")]
    pub(crate) num_consumers: usize,
}

pub(crate) fn load_configuration() -> Ingestor {
    Ingestor::parse()
}
