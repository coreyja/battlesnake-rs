#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;

extern crate rand;

use rocket::http::Status;
use std::collections::HashSet;

mod amphibious_arthur;
mod bombastic_bob;
mod constant_carter;
mod devious_devin;
mod opentelemetry_rocket;

#[derive(Serialize)]
pub struct AboutMe {
    apiversion: String,
    author: Option<String>,
    color: Option<String>,
    head: Option<String>,
    tail: Option<String>,
    version: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Ruleset {
    name: String,
    version: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Game {
    id: String,
    ruleset: Option<Ruleset>,
    timeout: u64,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Hash, Clone)]
pub struct Coordinate {
    x: i64,
    y: i64,
}

#[derive(Clone, PartialEq, Eq, Hash, Copy, Debug)]
pub enum Direction {
    UP,
    RIGHT,
    DOWN,
    LEFT,
}

impl Direction {
    fn value(&self) -> String {
        match self {
            Direction::UP => "up",
            Direction::RIGHT => "right",
            Direction::LEFT => "left",
            Direction::DOWN => "down",
        }
        .to_owned()
    }
}

const ALL_DIRECTIONS: [Direction; 4] = [
    Direction::UP,
    Direction::RIGHT,
    Direction::DOWN,
    Direction::LEFT,
];

impl Coordinate {
    fn valid(&self, board: &Board) -> bool {
        self.x >= 0 && self.x < board.width.into() && self.y >= 0 && self.y < board.height.into()
    }

    fn move_in(&self, direction: &Direction) -> Self {
        let mut x = self.x;
        let mut y = self.y;

        match direction {
            Direction::UP => {
                y += 1;
            }
            Direction::DOWN => {
                y -= 1;
            }
            Direction::LEFT => {
                x -= 1;
            }
            Direction::RIGHT => {
                x += 1;
            }
        };

        Self { x, y }
    }

    fn possbile_moves(&self, board: &Board) -> HashSet<(Direction, Coordinate)> {
        ALL_DIRECTIONS
            .iter()
            .cloned()
            .map(|dir| (dir, self.move_in(&dir)))
            .filter(|(_, coor)| coor.valid(board))
            .collect()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Battlesnake {
    id: String,
    name: String,
    health: u8,
    body: Vec<Coordinate>,
    latency: serde_json::Value,
    head: Coordinate,
    length: u64,
    shout: Option<String>,
    squad: Option<String>,
}

impl Battlesnake {
    fn possbile_moves(&self, board: &Board) -> HashSet<(Direction, Coordinate)> {
        self.head.possbile_moves(board)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Board {
    height: u32,
    width: u32,
    food: Vec<Coordinate>,
    hazards: Vec<Coordinate>,
    snakes: Vec<Battlesnake>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GameState {
    game: Game,
    turn: u64,
    board: Board,
    you: Battlesnake,
}

impl GameState {
    fn move_to(&self, coor: &Coordinate, snake_id: &str) -> Self {
        let mut clonned = self.clone();
        let you: &mut Battlesnake = clonned
            .board
            .snakes
            .iter_mut()
            .find(|s| s.id == snake_id)
            .expect("We didn't find that snake");

        you.body.insert(0, coor.clone());

        if you.health > 0 {
            you.health -= 1;
        }

        if let Some(pos) = clonned.board.food.iter().position(|x| x == coor) {
            clonned.board.food.remove(pos);
            you.health = 100;
        } else {
            you.body.pop();
        }

        clonned
    }
}

#[derive(Serialize, Debug)]
pub struct MoveOutput {
    r#move: String,
    shout: Option<String>,
}

use async_executors::TokioTpBuilder;
use opentelemetry_honeycomb::HoneycombApiKey;
use std::sync::Arc;

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

    let f = opentelemetry_rocket::OpenTelemetryFairing {
        tracer: x.map(|x| x.1),
    };

    rocket::ignite()
        .attach(f)
        .mount(
            "/amphibious-arthur",
            routes![
                amphibious_arthur::me,
                amphibious_arthur::start,
                amphibious_arthur::api_move,
                amphibious_arthur::end,
            ],
        )
        .mount(
            "/bombastic-bob",
            routes![
                bombastic_bob::me,
                bombastic_bob::start,
                bombastic_bob::api_move,
                bombastic_bob::end,
            ],
        )
        .mount(
            "/constant-carter",
            routes![
                constant_carter::me,
                constant_carter::start,
                constant_carter::api_move,
                constant_carter::end,
            ],
        )
        .mount(
            "/devious-devin",
            routes![
                devious_devin::me,
                devious_devin::start,
                devious_devin::api_move,
                devious_devin::end,
            ],
        )
        .launch();
}
