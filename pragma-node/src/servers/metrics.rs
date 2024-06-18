use axum::body::Body;
use axum::http::Response;
use axum::response::IntoResponse;
use axum::Server;
use axum::{routing::get, Router};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;

pub async fn run_metrics_server() {
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/metrics", get(metrics_handler));

    let port = String::from("8080").parse::<u16>().unwrap();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("metrics on http://0.0.0.0:8080");
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root_handler() -> impl IntoResponse {
    "<a href=\"/metrics\">/metrics</a>".to_string()
}

async fn metrics_handler() -> impl IntoResponse {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    tracing::info!("Huh: {:?}", metric_families);

    encoder.encode(&metric_families, &mut buffer).unwrap();

    Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap()
}
