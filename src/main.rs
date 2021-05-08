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

impl Default for AboutMe {
    fn default() -> Self {
        AboutMe {
            apiversion: "1".to_owned(),
            author: None,
            color: None,
            head: None,
            tail: None,
            version: None,
        }
    }
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

pub enum MoveResult {
    AteFood(u8), // old_health
    MovedTail(Coordinate),
}
pub struct Move {
    snake_id: String,
    move_result: MoveResult,
}

impl GameState {
    fn move_to(&mut self, coor: &Coordinate, snake_id: &str) -> Move {
        let to_move = self
            .board
            .snakes
            .iter_mut()
            .find(|s| s.id == snake_id)
            .unwrap();
        to_move.body.insert(0, coor.clone());

        if to_move.health > 0 {
            to_move.health -= 1;
        }

        let move_result = if let Some(pos) = self.board.food.iter().position(|x| x == coor) {
            self.board.food.remove(pos);
            let old_health = to_move.health;
            to_move.health = 100;
            MoveResult::AteFood(old_health)
        } else {
            MoveResult::MovedTail(to_move.body.pop().unwrap())
        };

        let snake_id = snake_id.to_owned();
        Move {
            snake_id,
            move_result,
        }
    }

    fn reverse_move(&mut self, m: Move) {
        let to_move = self
            .board
            .snakes
            .iter_mut()
            .find(|s| s.id == m.snake_id)
            .unwrap();
        to_move.body.remove(0);

        match m.move_result {
            MoveResult::AteFood(old_health) => {
                to_move.health = old_health;
            }
            MoveResult::MovedTail(tail) => {
                to_move.body.push(tail);
                to_move.health += 1;
            }
        }
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

use amphibious_arthur::AmphibiousArthur;
use bombastic_bob::BombasticBob;
use constant_carter::ConstantCarter;
use rocket::State;
use rocket_contrib::json::Json;

type BoxedSnake = Box<dyn BattlesnakeAI + Send + Sync>;
pub trait BattlesnakeAI {
    fn start(&self) {}
    fn end(&self) {}
    fn make_move(&self, state: GameState) -> Result<MoveOutput, Box<dyn std::error::Error>>;
    fn name(&self) -> String;

    fn about(&self) -> AboutMe {
        Default::default()
    }
}

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

    // let f = opentelemetry_rocket::OpenTelemetryFairing {
    //     tracer: x.map(|x| x.1),
    // };

    let snakes: Vec<BoxedSnake> = vec![
        Box::new(ConstantCarter {}),
        Box::new(BombasticBob {}),
        Box::new(AmphibiousArthur::new(Arc::new(x.map(|x| x.1)))),
    ];

    rocket::ignite()
        .manage(snakes)
        .mount(
            "/devious-devin",
            routes![
                devious_devin::me,
                devious_devin::start,
                devious_devin::api_moved,
                devious_devin::end,
            ],
        )
        .mount("/", routes![api_start, api_end, api_move, api_about])
        .launch();
}
