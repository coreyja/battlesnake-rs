#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use tracing_subscriber::EnvFilter;

use battlesnake_rs::gigantic_george::GiganticGeorge;
use rocket::http::Status;

use battlesnake_rs::constant_carter::ConstantCarter;
use battlesnake_rs::devious_devin::DeviousDevin;
use battlesnake_rs::{amphibious_arthur::AmphibiousArthur, famished_frank::FamishedFrank};
use battlesnake_rs::{bombastic_bob::BombasticBob, eremetic_eric::EremeticEric};
use battlesnake_rs::{AboutMe, BoxedSnake, GameState, MoveOutput};

use rocket::State;

use rocket_contrib::json::Json;

#[post("/<_snake>/start")]
fn api_start(_snake: String) -> Status {
    Status::NoContent
}

#[post("/<snake>/end", data = "<game_state>")]
fn api_end(
    snake: String,
    snakes: State<Vec<BoxedSnake>>,
    game_state: Json<GameState>,
) -> Option<Status> {
    let snake_ai = snakes.iter().find(|s| s.name() == snake)?;
    snake_ai.end(game_state.into_inner());

    Some(Status::NoContent)
}

#[post("/<snake>/move", data = "<game_state>")]
fn api_move(
    snake: String,
    snakes: State<Vec<BoxedSnake>>,
    game_state: Json<GameState>,
) -> Option<Json<MoveOutput>> {
    let snake_ai = snakes.iter().find(|s| s.name() == snake)?;
    let m = snake_ai.make_move(game_state.into_inner()).ok()?;

    Some(Json(m))
}

#[get("/<snake>")]
fn api_about(snake: String, snakes: State<Vec<BoxedSnake>>) -> Option<Json<AboutMe>> {
    let snake_ai = snakes.iter().find(|s| s.name() == snake)?;
    Some(Json(snake_ai.about()))
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "battlesnake_rs=info")
    }

    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let snakes: Vec<BoxedSnake> = vec![
        Box::new(AmphibiousArthur {}),
        Box::new(BombasticBob {}),
        Box::new(ConstantCarter {}),
        Box::new(DeviousDevin {}),
        Box::new(EremeticEric {}),
        Box::new(FamishedFrank {}),
        Box::new(GiganticGeorge {}),
    ];

    let cors = rocket_cors::CorsOptions::default().to_cors().unwrap();

    rocket::ignite()
        .manage(snakes)
        .attach(cors)
        .mount("/", routes![api_start, api_end, api_move, api_about])
        .launch();
}
