#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

extern crate rand;

use rocket::http::Status;

use battlesnake_rs::amphibious_arthur::AmphibiousArthur;
use battlesnake_rs::bombastic_bob::BombasticBob;
use battlesnake_rs::constant_carter::ConstantCarter;
use battlesnake_rs::devious_devin::DeviousDevin;
use battlesnake_rs::{AboutMe, BoxedSnake, GameState, MoveOutput};

use rocket::State;

use async_executors::TokioTpBuilder;
use opentelemetry_honeycomb::HoneycombApiKey;
use rocket_contrib::json::Json;
use std::sync::Arc;

#[post("/<snake>/start")]
fn api_start(snake: String) -> Status {
    Status::NoContent
}

#[post("/<snake>/end")]
fn api_end(snake: String) -> Status {
    Status::NoContent
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
    let mut builder = TokioTpBuilder::new();
    builder.tokio_builder().enable_io().enable_time();
    let executor = Arc::new(builder.build().expect("Failed to build Tokio executor"));

    let x = match (
        std::env::var("HONEYCOMB_API_KEY"),
        std::env::var("HONEYCOMB_DATASET"),
    ) {
        (Ok(api_key), Ok(dataset)) => Some(
            opentelemetry_honeycomb::new_pipeline(
                HoneycombApiKey::new(api_key),
                dataset,
                executor.clone(),
                move |fut| executor.block_on(fut),
            )
            .install()
            .unwrap(),
        ),
        _ => None,
    };

    let snakes: Vec<BoxedSnake> = vec![
        Box::new(ConstantCarter {}),
        Box::new(BombasticBob {}),
        Box::new(AmphibiousArthur::new(Arc::new(x.map(|x| x.1)))),
        Box::new(DeviousDevin {}),
    ];

    rocket::ignite()
        .manage(snakes)
        .mount("/", routes![api_start, api_end, api_move, api_about])
        .launch();
}