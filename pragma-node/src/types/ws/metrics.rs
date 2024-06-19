use lazy_static::lazy_static;
use prometheus::{opts, register_gauge_vec, GaugeVec};
use std::string::ToString;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Labels {
    pub interaction: Interaction,
    pub status: Status,
}

#[derive(strum::Display, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Interaction {
    #[strum(to_string = "New Connection")]
    NewConnection,
    #[strum(to_string = "Close Connection")]
    CloseConnection,
    #[strum(to_string = "Client Message Decoding")]
    ClientMessageDecode,
    #[strum(to_string = "Client Message Processing")]
    ClientMessageProcess,
    #[strum(to_string = "Channel Update")]
    ChannelUpdate,
}

#[derive(strum::Display, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Status {
    #[strum(to_string = "Success")]
    Success,
    #[strum(to_string = "Error")]
    Error,
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
        .with_label_values(&[&interaction.to_string(), &status.to_string()])
        .inc();
}
