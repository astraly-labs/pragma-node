use axum::body::Body;
use axum::http::Response;
use axum::response::IntoResponse;
use axum::Server;
use axum::{routing::get, Router};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;

use crate::config::Config;

pub async fn run_metrics_server(config: &Config) {
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/metrics", get(metrics_handler));

    let host = config.server_host();
    let port = config.metrics_port();

    let address = format!("{}:{}", host, port);
    let socket_addr: SocketAddr = address.parse().unwrap();

    tracing::info!("🖨  Metrics available at http://{}/metrics", socket_addr);
    Server::bind(&socket_addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root_handler() -> impl IntoResponse {
    let html_content = "<a href=\"/metrics\">/metrics</a>";
    Response::builder()
        .header("Content-Type", "text/html")
        .body(Body::from(html_content))
        .unwrap()
}

async fn metrics_handler() -> impl IntoResponse {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    encoder.encode(&metric_families, &mut buffer).unwrap();

    Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap()
}
