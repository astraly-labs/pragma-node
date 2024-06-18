use lazy_static::lazy_static;

use prometheus::{opts, register_gauge_vec, GaugeVec};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Labels {
    pub interaction: Interaction,
    pub status: Status,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Interaction {
    NewConnection,
    CloseConnection,
    ClientMessage,
    ChannelUpdate,
}

impl Interaction {
    fn as_str(&self) -> &'static str {
        match self {
            Interaction::NewConnection => "New Connection",
            Interaction::CloseConnection => "Close Connection",
            Interaction::ClientMessage => "Client Message",
            Interaction::ChannelUpdate => "Channel Update",
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Status {
    Success,
    Error,
}

impl Status {
    fn as_str(&self) -> &'static str {
        match self {
            Status::Success => "Success",
            Status::Error => "Error",
        }
    }
}

lazy_static! {
    pub static ref WS_INTERACTIONS: GaugeVec = register_gauge_vec!(
        opts!("ws_interactions", "Websocket interactions"),
        &["interaction", "status"]
    )
    .unwrap();
}

pub fn record_ws_interaction(interaction: Interaction, status: Status) {
    WS_INTERACTIONS
        .with_label_values(&[interaction.as_str(), status.as_str()])
        .inc();
}
