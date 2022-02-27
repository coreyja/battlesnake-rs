#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use tracing::Subscriber;
use tracing_subscriber::layer::SubscriberExt;

use rocket::http::Status;

use battlesnake_rs::{all_factories, AboutMe, BoxedFactory, Game, MoveOutput};

use rocket::State;

use rocket_contrib::json::Json;

#[post("/<_snake>/start")]
fn api_start(_snake: String) -> Status {
    Status::NoContent
}

#[post("/<snake>/end", data = "<game_state>")]
fn api_end(
    snake: String,
    factories: State<Vec<BoxedFactory>>,
    game_state: Json<Game>,
) -> Option<Status> {
    let snake_ai = factories
        .iter()
        .find(|s| s.name() == snake)?
        .from_wire_game(game_state.into_inner());
    snake_ai.end();

    Some(Status::NoContent)
}

#[post("/<snake>/move", data = "<game_state>")]
fn api_move(
    snake: String,
    factories: State<Vec<BoxedFactory>>,
    game_state: Json<Game>,
) -> Option<Json<MoveOutput>> {
    let snake_ai = factories
        .iter()
        .find(|s| s.name() == snake)?
        .from_wire_game(game_state.into_inner());
    let m = snake_ai.make_move().ok()?;

    Some(Json(m))
}

#[get("/<snake>")]
fn api_about(snake: String, factories: State<Vec<BoxedFactory>>) -> Option<Json<AboutMe>> {
    let factory = factories.iter().find(|s| s.name() == snake)?;
    Some(Json(factory.about()))
}

fn main() {
    let subscriber: Box<dyn Subscriber + Send + Sync> = if std::env::var("JSON_LOGS").is_ok() {
        Box::new(
            tracing_subscriber::registry::Registry::default()
                .with(tracing_subscriber::filter::LevelFilter::DEBUG)
                .with(tracing_subscriber::fmt::Layer::default().json()),
        )
    } else {
        Box::new(
            tracing_subscriber::registry::Registry::default()
                .with(tracing_subscriber::filter::LevelFilter::DEBUG)
                .with(tracing_subscriber::fmt::Layer::default()),
        )
    };
    // let layer = if let Ok(_) = std::env::var("JSON_LOGS") {
    //     tracing_subscriber::fmt::Layer::default().json()
    // } else {
    //     tracing_subscriber::fmt::Layer::default()
    // };

    tracing::subscriber::set_global_default(subscriber).expect("setting global default failed");

    let cors = rocket_cors::CorsOptions::default().to_cors().unwrap();

    rocket::ignite()
        .manage(all_factories())
        .attach(cors)
        .mount("/", routes![api_start, api_end, api_move, api_about])
        .launch();
}
