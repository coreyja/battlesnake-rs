#![deny(warnings)]

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use battlesnake_rs::{all_factories, Game};

use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/constant-carter", get(constant_carter_info))
        .route("/constant-carter/start", post(constant_carter_start))
        .route("/constant-carter/move", post(constant_carter_move))
        .route("/constant-carter/end", post(constant_carter_end));

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

async fn constant_carter_move(Json(game): Json<Game>) -> impl IntoResponse {
    let factories = all_factories();
    let carter_factory = factories
        .iter()
        .find(|f| f.name() == "constant-carter")
        .unwrap();
    let carter = carter_factory.from_wire_game(game);

    // TODO: Figure out how to return errors nicely here
    // Unwrapping and panicking isn't ideal, we want to return a 500 of some kind
    // when we run into an error
    Json(carter.make_move().unwrap())
}

async fn constant_carter_start() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}
async fn constant_carter_end(Json(game): Json<Game>) -> impl IntoResponse {
    let factories = all_factories();
    let carter_factory = factories
        .iter()
        .find(|f| f.name() == "constant-carter")
        .unwrap();
    let carter = carter_factory.from_wire_game(game);

    carter.end();

    StatusCode::NO_CONTENT
}
