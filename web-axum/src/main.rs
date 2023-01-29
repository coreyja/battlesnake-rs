#![feature(let_chains)]
#![deny(warnings)]

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use battlesnake_minimax::{
    paranoid::{move_ordering::MoveOrdering, MinMaxReturn, SnakeOptions},
    types::types::YouDeterminableGame,
    ParanoidMinimaxSnake,
};
use battlesnake_rs::{
    all_factories, build_snake_id_map,
    hovering_hobbs::{standard_score, Factory, Score},
    improbable_irene::{Arena, ImprobableIrene},
    BoxedFactory, Game, MoveOutput, SnakeId, StandardCellBoard4Snakes11x11,
};
use color_eyre::{
    eyre::{eyre, Result},
    Report,
};

use opentelemetry_otlp::WithExportConfig;
use parking_lot::Mutex;
use sentry_tower::NewSentryLayer;
use serde_json::json;
use tokio::task::{JoinError, JoinHandle};

use tower_http::{
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{span, Instrument, Level};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::Layer;
use tracing_subscriber::{prelude::*, registry::Registry};
use tracing_tree::HierarchicalLayer;

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

struct ExtractSnakeFactory(BoxedFactory);

#[async_trait]
impl<State: Send + Sync> FromRequestParts<State> for ExtractSnakeFactory {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        req: &mut axum::http::request::Parts,
        state: &State,
    ) -> Result<Self, Self::Rejection> {
        let Path(snake_name) = Path::<String>::from_request_parts(req, state)
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
async fn main() -> Result<()> {
    color_eyre::install()?;

    let git_commit = std::option_env!("CIRCLE_SHA1");
    let release_name = if let Some(git_commit) = git_commit {
        git_commit.into()
    } else {
        sentry::release_name!().unwrap_or_else(|| "dev".into())
    };

    let _guard = if let Ok(sentry_dsn) = std::env::var("SENTRY_DSN") {
        println!("Sentry enabled");

        Some(sentry::init((
            sentry_dsn,
            sentry::ClientOptions {
                traces_sample_rate: 0.0,
                release: Some(release_name),
                ..Default::default()
            },
        )))
    } else {
        None
    };

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var(
            "RUST_LOG",
            "info,battlesnake-minimax=info,battlesnake-rs=info",
        );
    }
    let logging: Box<dyn Layer<Registry> + Send + Sync> = if std::env::var("JSON_LOGS").is_ok() {
        Box::new(tracing_subscriber::fmt::layer().json())
    } else {
        Box::new(tracing_subscriber::fmt::layer())
    };
    let env_filter = tracing_subscriber::EnvFilter::from_default_env();

    let opentelemetry_layer = if let Ok(honeycomb_key) = std::env::var("HONEYCOMB_API_KEY") {
        let mut map = HashMap::<String, String>::new();
        map.insert("x-honeycomb-team".to_string(), honeycomb_key);
        map.insert("x-honeycomb-dataset".to_string(), "web-axum".to_string());

        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .http()
                    .with_endpoint("https://api.honeycomb.io/v1/traces")
                    .with_timeout(Duration::from_secs(3))
                    .with_headers(map),
            )
            .install_batch(opentelemetry::runtime::Tokio)?;

        let opentelemetry_layer = OpenTelemetryLayer::new(tracer);

        Some(opentelemetry_layer)
    } else {
        None
    };

    let heirarchical = if opentelemetry_layer.is_none() {
        let heirarchical = HierarchicalLayer::default()
            .with_writer(std::io::stdout)
            .with_indent_lines(true)
            .with_indent_amount(2)
            .with_thread_names(true)
            .with_thread_ids(true)
            .with_verbose_exit(true)
            .with_verbose_entry(true)
            .with_targets(true);

        Some(heirarchical)
    } else {
        None
    };

    Registry::default()
        .with(logging)
        .with(heirarchical)
        .with(opentelemetry_layer)
        .with(env_filter)
        .with(sentry_tracing::layer())
        .try_init()?;

    let state = AppState {
        game_states: HashMap::new(),
    };
    let state = Mutex::new(state);
    let state = Arc::new(state);

    let app = Router::new()
        .route("/", get(root))
        .route("/hovering-hobbs", get(route_hobbs_info))
        .route("/hovering-hobbs/start", post(route_hobbs_start))
        .route("/hovering-hobbs/move", post(route_hobbs_move))
        .route("/hovering-hobbs/end", post(route_hobbs_end))
        .route("/:snake_name", get(route_info))
        .route("/:snake_name/start", post(route_start))
        .route("/:snake_name/move", post(route_move))
        .route("/improbable-irene/graph", post(route_graph))
        .route("/:snake_name/end", post(route_end))
        .layer(sentry_tower::SentryHttpLayer::with_transaction())
        .layer(NewSentryLayer::new_from_top())
        .layer(
            TraceLayer::new_for_http()
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .with_state(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()?;

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

struct HttpError(color_eyre::eyre::Report);

impl From<Report> for HttpError {
    fn from(value: Report) -> Self {
        Self(value)
    }
}

impl From<JoinError> for HttpError {
    fn from(value: JoinError) -> Self {
        Self(eyre!(value).wrap_err("Join Error"))
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Things Broke"})),
        )
            .into_response()
    }
}

type HttpResponse<T> = Result<T, HttpError>;
type JsonResponse<JsonType> = HttpResponse<Json<JsonType>>;

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
) -> JsonResponse<MoveOutput> {
    let snake = factory.create_from_wire_game(game);

    let output = spawn_blocking_with_tracing(move || snake.make_move()).await??;

    Ok(Json(output))
}

async fn route_graph(Json(game): Json<Game>) -> JsonResponse<MoveOutput> {
    let game_info = game.game.clone();
    let id_map = build_snake_id_map(&game);
    let turn = game.turn;

    assert_ne!(
        game_info.ruleset.name, "wrapped",
        "Graphing does not currently support wrapped games"
    );
    let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

    let snake = ImprobableIrene::new(game, game_info, turn);

    let root = span!(tracing::Level::INFO, "graph_move");
    let output = spawn_blocking_with_tracing(move || {
        let mut arena = Arena::new();
        snake
            .graph_move(&mut arena)
            .expect("TODO: We need to work on our error handling")
    })
    .instrument(root)
    .await?;

    Ok(Json(output))
}

async fn route_start() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}
async fn route_end(
    ExtractSnakeFactory(factory): ExtractSnakeFactory,
    Json(game): Json<Game>,
) -> impl IntoResponse {
    let snake = factory.create_from_wire_game(game);

    snake.end();

    StatusCode::NO_CONTENT
}

mod hobbs;
use hobbs::*;
