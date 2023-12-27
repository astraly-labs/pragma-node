use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErrorKind {
    #[error("read config error: {0}")]
    ReadConfig(#[from] std::io::Error),
    #[error("load config error: {0}")]
    LoadConfig(#[from] envy::Error),
}
