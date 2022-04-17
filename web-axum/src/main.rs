#![deny(warnings)]

use axum::{response::IntoResponse, routing::get, Json, Router};
use battlesnake_rs::all_factories;

use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/constant-carter", get(constant_carter_info));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn constant_carter_info() -> impl IntoResponse {
    let factories = all_factories();
    let carter_factory = factories
        .iter()
        .find(|f| f.name() == "constant-carter")
        .unwrap();
    let carter_info = carter_factory.about();

    Json(carter_info)
}
