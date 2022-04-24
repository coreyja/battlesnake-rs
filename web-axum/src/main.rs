#![deny(warnings)]

use axum::{
    async_trait,
    extract::{FromRequest, Path, RequestParts},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use battlesnake_rs::{all_factories, BoxedFactory, Game};

use std::net::SocketAddr;

struct ExtractSnakeFactory(BoxedFactory);

#[async_trait]
impl<B> FromRequest<B> for ExtractSnakeFactory
where
    B: Send,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Path(snake_name) = Path::<String>::from_request(req)
            .await
            .map_err(|_err| (StatusCode::NOT_FOUND, "Couldn't extract snake name"))?;

        let factories = all_factories();
        let factory = factories
            .into_iter()
            .find(|f| f.name() == snake_name)
            .ok_or((StatusCode::NOT_FOUND, "No factory found"))?;

        Ok(Self(factory))
    }
}

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/:snake_name", get(route_info))
        .route("/:snake_name/start", post(route_start))
        .route("/:snake_name/move", post(constant_carter_move))
        .route("/:snake_name/end", post(route_end));

    // NOTE: Axum routes must start with a leading slash.
    // This is NOT checked at compile time
    //
    // .route("/constant-carter", get(constant_carter_info))
    // .route("/constant-carter/start", post(constant_carter_start))
    // .route("/constant-carter/move", post(constant_carter_move))
    // .route("/constant-carter/end", post(constant_carter_end));

    // for factory in factories {
    //     let wrapped = WrappedFactory::new(factory);

    //     app = app.route(
    //         &format!("/{}", wrapped.name()),
    //         get(wrapped.make_info_route()),
    //     );
    //     app = app.route(
    //         &format!("/{}/start", wrapped.name()),
    //         post(constant_carter_start),
    //     );
    //     app = app.route(
    //         &format!("/{}/move", wrapped.name()),
    //         post(constant_carter_move),
    //     );
    //     app = app.route(
    //         &format!("/{}/end", wrapped.name()),
    //         post(constant_carter_end),
    //     );
    // }

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

// fn handle_anyhow_error(err: anyhow::Error) -> (StatusCode, String) {
//     (
//         StatusCode::INTERNAL_SERVER_ERROR,
//         format!("Something went wrong: {}", err),
//     )
// }

async fn route_info(ExtractSnakeFactory(factory): ExtractSnakeFactory) -> impl IntoResponse {
    let carter_info = factory.about();

    Json(carter_info)
}

async fn constant_carter_move(
    ExtractSnakeFactory(factory): ExtractSnakeFactory,
    Json(game): Json<Game>,
) -> impl IntoResponse {
    let snake = factory.from_wire_game(game);

    let output = snake
        .make_move()
        .expect("TODO: We need to work on our error handling");

    Json(output)
}

async fn route_start() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}
async fn route_end(
    ExtractSnakeFactory(factory): ExtractSnakeFactory,
    Json(game): Json<Game>,
) -> impl IntoResponse {
    let snake = factory.from_wire_game(game);

    snake.end();

    StatusCode::NO_CONTENT
}
