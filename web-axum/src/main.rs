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

use tokio::{task::JoinHandle, time::Instant};

use tracing::{span, Instrument};
use tracing_honeycomb::{
    libhoney, new_blackhole_telemetry_layer, new_honeycomb_telemetry_layer,
    register_dist_tracing_root, TraceId,
};
use tracing_subscriber::layer::Layer;
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

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info,libhoney=warn");
    }
    let logging: Box<dyn Layer<Registry> + Send + Sync> = if std::env::var("JSON_LOGS").is_ok() {
        Box::new(tracing_subscriber::fmt::layer().json())
    } else {
        Box::new(tracing_subscriber::fmt::layer())
    };
    let env_filter = tracing_subscriber::EnvFilter::from_default_env();

    let honeycomb_api_key = std::env::var("HONEYCOMB_API_KEY");
    if let Ok(honeycomb_api_key) = honeycomb_api_key {
        let honeycomb_config = libhoney::Config {
            options: libhoney::client::Options {
                api_key: honeycomb_api_key,
                dataset: "battlesnakes".to_string(),
                ..libhoney::client::Options::default()
            },
            transmission_options: libhoney::transmission::Options::default(),
        };

        let telemetry_layer = new_honeycomb_telemetry_layer("web-axum", honeycomb_config);
        println!("Sending traces to Honeycomb");

        Registry::default()
            .with(logging)
            .with(telemetry_layer)
            .with(env_filter)
            .try_init()
            .expect("Failed to initialize tracing");
    } else {
        let telemetry_layer = new_blackhole_telemetry_layer();
        Registry::default()
            .with(logging)
            .with(telemetry_layer)
            .with(env_filter)
            .try_init()
            .expect("Failed to initialize tracing");
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/:snake_name", get(route_info))
        .route("/:snake_name/start", post(route_start))
        .route("/:snake_name/move", post(route_move))
        .route("/:snake_name/end", post(route_end))
        .layer(axum::middleware::from_fn(log_request));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
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

fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}

async fn route_move(
    ExtractSnakeFactory(factory): ExtractSnakeFactory,
    Json(game): Json<Game>,
) -> impl IntoResponse {
    let snake = factory.from_wire_game(game);

    let root = span!(tracing::Level::INFO, "make_move");
    let output = spawn_blocking_with_tracing(move || {
        snake
            .make_move()
            .expect("TODO: We need to work on our error handling")
    })
    .instrument(root)
    .await
    .unwrap();

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

#[tracing::instrument(
  level = "info",
  skip_all,
  fields(
    http.uri =? req.uri(),
    http.path =? req.uri().path(),
    http.method =? req.method(),
    factory_name,
    request_duration,
  ),
)]
async fn log_request(
    req: Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    register_dist_tracing_root(TraceId::new(), None).unwrap();
    let current_span = tracing::Span::current();

    let mut req_parts = RequestParts::new(req);
    let factory: Option<ExtractSnakeFactory> = req_parts
        .extract()
        .await
        .expect("This has an infallible error type so this unwrap is always safe");

    if let Some(f) = factory {
        let factory_name = f.0.name();
        current_span.record("factory_name", &format!("{:?}", &factory_name).as_str());
    }

    let req = req_parts
        .try_into_request()
        .map_err(|_err| (StatusCode::BAD_REQUEST, "Couldn't parse request"))?;

    let start = Instant::now();

    let root = span!(tracing::Level::INFO, "axum request");
    let res = next.run(req).instrument(root).await;

    let duration = start.elapsed();

    tracing::info!(?duration, "Request processed");
    current_span.record("request_duration", &format!("{:?}", &duration).as_str());

    Ok(res)
}
