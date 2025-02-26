use std::env;

pub mod docker;
pub mod local;

// Main port of the API
pub const SERVER_PORT: u16 = 3000;

// Port where we expose pragma-node metrics
const METRICS_PORT: u16 = 8080;

// Port used by both databases in their container
const DB_PORT: u16 = 5432;

#[derive(Debug, Clone, PartialEq)]
pub enum PragmaNodeMode {
    Docker,
    Local,
}

impl Default for PragmaNodeMode {
    fn default() -> Self {
        match env::var("PRAGMA_NODE_MODE").as_deref() {
            Ok("local") => PragmaNodeMode::Local,
            _ => PragmaNodeMode::Docker,
        }
    }
}
