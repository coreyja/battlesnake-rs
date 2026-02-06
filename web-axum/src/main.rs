#![deny(warnings)]

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
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
use serde::Deserialize;

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
        // SAFETY: Called at startup before any threads are spawned
        unsafe {
            std::env::set_var(
                "RUST_LOG",
                "info,battlesnake-minimax=info,battlesnake-rs=info",
            );
        }
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
            Json(json!({ "error": "Things Broke", "details": self.0.to_string()})),
        )
            .into_response()
    }
}

type HttpResponse<T> = Result<T, HttpError>;
type JsonResponse<JsonType> = HttpResponse<Json<JsonType>>;

struct SnakeInfo {
    name: &'static str,
    slug: &'static str,
    color: &'static str,
    head: &'static str,
    tail: &'static str,
    strategy: &'static str,
    description: &'static str,
}

fn snake_catalog() -> Vec<SnakeInfo> {
    vec![
        SnakeInfo {
            name: "Amphibious Arthur",
            slug: "amphibious-arthur",
            color: "#AA66CC",
            head: "trans-rights-scarf",
            tail: "swirl",
            strategy: "Recursive Simulation",
            description: "Simulates opponent sprawl and scores positions recursively. Keeps health around 80 and looks several moves ahead.",
        },
        SnakeInfo {
            name: "Bombastic Bob",
            slug: "bombastic-bob",
            color: "#AA66CC",
            head: "trans-rights-scarf",
            tail: "default",
            strategy: "Random Reasonable",
            description: "Picks a random valid move that won't immediately kill him. Simple, unpredictable, surprisingly effective.",
        },
        SnakeInfo {
            name: "Constant Carter",
            slug: "constant-carter",
            color: "#AA66CC",
            head: "trans-rights-scarf",
            tail: "default",
            strategy: "Always Right",
            description: "Always moves right. The ultimate baseline snake \u{2014} used for latency testing and benchmarking.",
        },
        SnakeInfo {
            name: "Devious Devin",
            slug: "devious-devin",
            color: "#99cc00",
            head: "trans-rights-scarf",
            tail: "rbc-necktie",
            strategy: "Paranoid Minimax",
            description: "Assumes all opponents play optimally against him. Hunts food when short, stalks opponents when long.",
        },
        SnakeInfo {
            name: "Eremetic Eric",
            slug: "eremetic-eric",
            color: "#FF4444",
            head: "trans-rights-scarf",
            tail: "default",
            strategy: "Tail Chaser",
            description: "The hermit snake. Chases his own tail in tight loops, only breaking formation to eat when starving.",
        },
        SnakeInfo {
            name: "Famished Frank",
            slug: "famished-frank",
            color: "#FFBB33",
            head: "trans-rights-scarf",
            tail: "default",
            strategy: "Grow & Corner",
            description: "Eats until enormous, then retreats to corners. Uses A* pathfinding to navigate around hazards.",
        },
        SnakeInfo {
            name: "Gigantic George",
            slug: "gigantic-george",
            color: "#FFBB33",
            head: "trans-rights-scarf",
            tail: "default",
            strategy: "Hamiltonian Path",
            description: "Attempts to fill the entire board with a Hamiltonian path. Encodes the full route in shout messages.",
        },
        SnakeInfo {
            name: "Jump Flooding",
            slug: "jump-flooding",
            color: "#efae09",
            head: "trans-rights-scarf",
            tail: "default",
            strategy: "Area Control",
            description: "Minimax with jump flooding algorithm for space evaluation. Maximizes the ratio of squares under its control.",
        },
        SnakeInfo {
            name: "Hovering Hobbs",
            slug: "hovering-hobbs",
            color: "#da8a1a",
            head: "beach-puffin-special",
            tail: "beach-puffin-special",
            strategy: "Minimax + Flood Fill",
            description: "Advanced paranoid minimax with flood fill scoring. Adapts strategy for arcade maze vs standard maps.",
        },
        SnakeInfo {
            name: "Improbable Irene",
            slug: "improbable-irene",
            color: "#5a25a8",
            head: "hydra",
            tail: "mystic-moon",
            strategy: "Monte Carlo Tree Search",
            description: "Uses MCTS with UCB1-normal selection. Simulates thousands of random games to find the strongest move.",
        },
    ]
}

async fn root() -> Html<String> {
    let snakes = snake_catalog();

    let markup = maud::html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "coreyja's battlesnakes" }
                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
                link href="https://fonts.googleapis.com/css2?family=Anybody:wght@400;700;900&family=IBM+Plex+Mono:wght@400;500&display=swap" rel="stylesheet";
                script src="https://cdn.tailwindcss.com" {}
                script {
                    (maud::PreEscaped(r#"
                    tailwind.config = {
                        theme: {
                            extend: {
                                fontFamily: {
                                    display: ['Anybody', 'sans-serif'],
                                    mono: ['IBM Plex Mono', 'monospace'],
                                },
                            }
                        }
                    }
                    "#))
                }
                style {
                    (maud::PreEscaped(r#"
                    @keyframes grain {
                        0%, 100% { transform: translate(0, 0); }
                        10% { transform: translate(-5%, -10%); }
                        30% { transform: translate(3%, -15%); }
                        50% { transform: translate(12%, 9%); }
                        70% { transform: translate(9%, 4%); }
                        90% { transform: translate(-1%, 7%); }
                    }
                    .grain::before {
                        content: '';
                        position: fixed;
                        top: -50%;
                        left: -50%;
                        right: -50%;
                        bottom: -50%;
                        width: 200%;
                        height: 200%;
                        background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)' opacity='0.04'/%3E%3C/svg%3E");
                        animation: grain 8s steps(10) infinite;
                        pointer-events: none;
                        z-index: 100;
                    }
                    .snake-card {
                        transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1), box-shadow 0.3s ease;
                    }
                    .snake-card:hover {
                        transform: translateY(-6px) scale(1.02);
                    }
                    @keyframes slither-in {
                        from {
                            opacity: 0;
                            transform: translateY(30px);
                        }
                        to {
                            opacity: 1;
                            transform: translateY(0);
                        }
                    }
                    .slither-in {
                        animation: slither-in 0.6s cubic-bezier(0.16, 1, 0.3, 1) both;
                    }
                    "#))
                }
            }
            body class="bg-zinc-950 text-zinc-100 font-mono min-h-screen grain antialiased" {
                // Ambient gradient background
                div class="fixed inset-0 pointer-events-none" style="background: radial-gradient(ellipse 80% 50% at 50% -20%, rgba(90, 37, 168, 0.15), transparent);" {}

                div class="relative z-10" {
                    // Header
                    header class="pt-16 pb-12 px-6" {
                        div class="max-w-5xl mx-auto" {
                            div class="flex items-end gap-4 mb-6" {
                                div class="text-5xl select-none" style="filter: drop-shadow(0 0 20px rgba(90, 37, 168, 0.5));" {
                                    "~"
                                }
                                h1 class="font-display font-black text-5xl md:text-7xl tracking-tight leading-none" {
                                    span class="text-zinc-400" { "coreyja" }
                                    span class="text-zinc-600" { "'" }
                                    span class="text-zinc-400" { "s" }
                                    br;
                                    span class="bg-gradient-to-r from-violet-400 via-fuchsia-400 to-amber-400 bg-clip-text text-transparent" {
                                        "battlesnakes"
                                    }
                                }
                            }
                            p class="text-zinc-500 text-lg max-w-xl leading-relaxed font-mono" {
                                "A collection of Battlesnake AIs written in Rust. "
                                "From simple baselines to Monte Carlo tree search, "
                                "each snake brings a different strategy to the arena."
                            }
                        }
                    }

                    // Divider with snake count
                    div class="px-6 mb-12" {
                        div class="max-w-5xl mx-auto flex items-center gap-4" {
                            div class="h-px flex-1 bg-gradient-to-r from-zinc-800 to-transparent" {}
                            span class="text-xs text-zinc-600 tracking-widest uppercase font-mono" {
                                (snakes.len()) " snakes deployed"
                            }
                            div class="h-px flex-1 bg-gradient-to-l from-zinc-800 to-transparent" {}
                        }
                    }

                    // Snake grid
                    main class="px-6 pb-24" {
                        div class="max-w-5xl mx-auto grid grid-cols-1 md:grid-cols-2 gap-5" {
                            @for (i, snake) in snakes.iter().enumerate() {
                                @let delay = format!("animation-delay: {}ms", i * 80);
                                div class="snake-card slither-in group relative rounded-2xl border border-zinc-800/80 bg-zinc-900/60 backdrop-blur-sm overflow-hidden"
                                    style=(delay) {

                                    // Color accent bar
                                    div class="h-1 w-full" style=(format!("background: linear-gradient(90deg, {}, transparent 80%);", snake.color)) {}

                                    // Hover glow effect
                                    div class="absolute inset-0 rounded-2xl opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none"
                                        style=(format!("background: radial-gradient(ellipse at top left, {}15, transparent 70%);", snake.color)) {}

                                    div class="p-6" {
                                        // Name and color swatch
                                        div class="flex items-start justify-between mb-4" {
                                            div {
                                                h2 class="font-display font-bold text-xl text-zinc-100 group-hover:text-white transition-colors" {
                                                    (snake.name)
                                                }
                                                div class="flex items-center gap-2 mt-1.5" {
                                                    span class="text-xs px-2.5 py-1 rounded-full border font-mono tracking-wide"
                                                        style=(format!("color: {}; border-color: {}40; background: {}10;", snake.color, snake.color, snake.color)) {
                                                        (snake.strategy)
                                                    }
                                                }
                                            }
                                            div class="w-8 h-8 rounded-lg shadow-lg ring-1 ring-white/10"
                                                style=(format!("background: {}; box-shadow: 0 0 20px {}40;", snake.color, snake.color)) {}
                                        }

                                        // Description
                                        p class="text-sm text-zinc-400 leading-relaxed mb-5" {
                                            (snake.description)
                                        }

                                        // Metadata footer
                                        div class="flex items-center justify-between text-xs text-zinc-600 font-mono" {
                                            div class="flex items-center gap-3" {
                                                span class="flex items-center gap-1.5" {
                                                    span class="text-zinc-500" { "head" }
                                                    span class="text-zinc-400" { (snake.head) }
                                                }
                                                @if snake.tail != "default" {
                                                    span class="text-zinc-700" { "|" }
                                                    span class="flex items-center gap-1.5" {
                                                        span class="text-zinc-500" { "tail" }
                                                        span class="text-zinc-400" { (snake.tail) }
                                                    }
                                                }
                                            }
                                            span class="text-zinc-700" {
                                                "/" (snake.slug)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Footer
                    footer class="px-6 pb-16" {
                        div class="max-w-5xl mx-auto" {
                            div class="h-px bg-zinc-800/50 mb-8" {}
                            div class="flex flex-col md:flex-row items-center justify-between gap-4 text-xs text-zinc-600 font-mono" {
                                span { "built with rust + axum + maud" }
                                a href="https://github.com/coreyja/battlesnake-rs" class="hover:text-zinc-400 transition-colors" {
                                    "github.com/coreyja/battlesnake-rs"
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    Html(markup.into_string())
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

#[derive(Debug, Deserialize)]
struct MoveQueryParams {
    sleep_ms: Option<u64>,
}

async fn route_move(
    ExtractSnakeFactory(factory): ExtractSnakeFactory,
    Query(params): Query<MoveQueryParams>,
    Json(game): Json<Game>,
) -> JsonResponse<MoveOutput> {
    if let Some(sleep_ms) = params.sleep_ms {
        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
    }

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
