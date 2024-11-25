use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};
use std::time::Instant;

pub async fn track_timing(req: Request<Body>, next: Next) -> Response<Body> {
    let start = Instant::now();
    let route = req.uri().path().to_owned();

    let response = next.run(req).await;

    let elapsed = start.elapsed();
    tracing::info!("🌐 {} - {:?}", route, elapsed);

    response
}

#[allow(dead_code)]
pub trait TimingLayer {
    fn with_timing(self) -> Self;
}

impl TimingLayer for axum::Router {
    fn with_timing(self) -> Self {
        self.layer(axum::middleware::from_fn(track_timing))
    }
}
