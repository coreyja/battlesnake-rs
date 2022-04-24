#![deny(warnings)]

use axum::{
    async_trait,
    body::Body,
    extract::{FromRequest, Path, RequestParts},
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use battlesnake_rs::{all_factories, BoxedFactory, Game};
use tokio::time::Instant;

use tracing::span;
use tracing_subscriber::{prelude::*, registry::Registry};

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

    // Install a new OpenTelemetry trace pipeline
    use opentelemetry::sdk::export::trace::stdout;
    let tracer = stdout::new_pipeline().install_simple();
    // Create a tracing layer with the configured tracer
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default().with(telemetry).try_init().unwrap();

    let app = Router::new()
        .route("/", get(root))
        .route("/:snake_name", get(route_info))
        .route("/:snake_name/start", post(route_start))
        .route("/:snake_name/move", post(route_move))
        .route("/:snake_name/end", post(route_end))
        .layer(axum::middleware::from_fn(log_request));

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

async fn route_info(ExtractSnakeFactory(factory): ExtractSnakeFactory) -> impl IntoResponse {
    let carter_info = factory.about();

    Json(carter_info)
}

async fn route_move(
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

// Logging Goals:
// - We want to see a Log Line for each request including
//     - Full URL

async fn log_request(
    req: Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let mut req_parts = RequestParts::new(req);
    let factory: Option<ExtractSnakeFactory> = req_parts
        .extract()
        .await
        .expect("This has an infallible error type so this unwrap is always safe");

    {
        let factory_name = factory.as_ref().map(|f| f.0.name());
        let url = req_parts.uri();

        tracing::info!(?factory_name, ?url, "Request received");
    }

    let root = span!(tracing::Level::INFO, "axum request");
    let enter = root.enter();
    let start = Instant::now();

    let req = req_parts
        .try_into_request()
        .map_err(|_err| (StatusCode::BAD_REQUEST, "Couldn't parse request"))?;

    let res = next.run(req).await;

    let duration = start.elapsed();
    drop(enter);

    tracing::info!(?duration, "Request processed");

    Ok(res)
}
