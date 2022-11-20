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
    types::{compact_representation::WrappedCellBoard4Snakes11x11, types::YouDeterminableGame},
    ParanoidMinimaxSnake,
};
use battlesnake_rs::{
    all_factories, build_snake_id_map,
    hovering_hobbs::{standard_score, Factory, Score},
    improbable_irene::{Arena, ImprobableIrene},
    BoxedFactory, Game, MoveOutput, StandardCellBoard4Snakes11x11,
};

use tokio::task::JoinHandle;

use tower_http::trace::TraceLayer;
use tracing::{span, Instrument};
use tracing_honeycomb::{libhoney, new_blackhole_telemetry_layer, new_honeycomb_telemetry_layer};
use tracing_subscriber::layer::Layer;
use tracing_subscriber::{prelude::*, registry::Registry};
use tracing_tree::HierarchicalLayer;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

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

#[derive(Debug)]
#[allow(dead_code)]
struct AppState {
    pub hobbs_last_move_return: HashMap<String, MinMaxReturn<WrappedCellBoard4Snakes11x11, Score>>,
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
            .with(
                HierarchicalLayer::default()
                    .with_writer(std::io::stdout)
                    .with_indent_lines(true)
                    .with_indent_amount(2)
                    .with_thread_names(true)
                    .with_thread_ids(true)
                    .with_verbose_exit(true)
                    .with_verbose_entry(true)
                    .with_targets(true),
            )
            .with(telemetry_layer)
            .with(env_filter)
            .try_init()
            .expect("Failed to initialize tracing");
    };

    let state = AppState {
        hobbs_last_move_return: HashMap::new(),
    };
    let state = Mutex::new(state);
    let state = Arc::new(state);

    let app = Router::new()
        .route("/", get(root))
        .route("/hovering-hobbs", post(route_hobbs_info))
        .route("/hovering-hobbs/start", post(route_hobbs_start))
        .route("/hovering-hobbs/move", post(route_hobbs_move))
        .route("/hovering-hobbs/end", post(route_hobbs_end))
        .route("/:snake_name", get(route_info))
        .route("/:snake_name/start", post(route_start))
        .route("/:snake_name/move", post(route_move))
        .route("/improbable-irene/graph", post(route_graph))
        .route("/:snake_name/end", post(route_end))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

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
    let snake = factory.create_from_wire_game(game);

    let output = spawn_blocking_with_tracing(move || {
        snake
            .make_move()
            .expect("TODO: We need to work on our error handling")
    })
    .await
    .unwrap();

    Json(output)
}

async fn route_graph(Json(game): Json<Game>) -> impl IntoResponse {
    let game_info = game.game.clone();
    let id_map = build_snake_id_map(&game);

    assert_ne!(
        game_info.ruleset.name, "wrapped",
        "Graphing does not currently support wrapped games"
    );
    let game = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

    let snake = ImprobableIrene::new(game, game_info);

    let root = span!(tracing::Level::INFO, "graph_move");
    let output = spawn_blocking_with_tracing(move || {
        let mut arena = Arena::new();
        snake
            .graph_move(&mut arena)
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
    let snake = factory.create_from_wire_game(game);

    snake.end();

    StatusCode::NO_CONTENT
}

async fn route_hobbs_info() -> impl IntoResponse {
    Json(Factory {}.about())
}
async fn route_hobbs_start() -> impl IntoResponse {
    // TODO: I need to build the id_map here and store it
    StatusCode::NO_CONTENT
}
async fn route_hobbs_end() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

async fn route_hobbs_move(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(game): Json<Game>,
) -> impl IntoResponse {
    let game_info = game.game.clone();
    let game_id = game_info.id.to_string();
    let turn = game.turn;

    let name = "hovering-hobbs";

    let options: SnakeOptions = SnakeOptions {
        network_latency_padding: Duration::from_millis(50),
        move_ordering: MoveOrdering::BestFirst,
    };

    {
        let state = state.lock().unwrap();

        let last_return = state.hobbs_last_move_return.get(&game_id);

        if let Some(r) = last_return {
            dbg!("We found a last return");
            dbg!(r);
        } else {
            dbg!("What this the first turn of the game? No last return found");
        }
    }

    let id_map = build_snake_id_map(&game);
    let game: WrappedCellBoard4Snakes11x11 =
        WrappedCellBoard4Snakes11x11::convert_from_game(game, &id_map)
            .expect("TODO: We need to work on our error handling");
    let my_id = game.you_id();
    let snake = ParanoidMinimaxSnake::new(game, game_info, turn, &standard_score, name, options);

    let (_depth, scored) = spawn_blocking_with_tracing(move || snake.choose_move_inner())
        .await
        .unwrap();

    let scored_options = scored.first_options_for_snake(my_id).unwrap();
    let output = scored_options.first().unwrap().0;

    {
        let mut state = state.lock().unwrap();

        state.hobbs_last_move_return.insert(game_id, scored);
    }

    let output: MoveOutput = MoveOutput {
        r#move: format!("{output}"),
        shout: None,
    };

    Json(output)
}
