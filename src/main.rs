#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;

use rocket::http::Status;
use rocket_contrib::json::Json;
use std::collections::HashSet;

#[derive(Serialize)]
struct AboutMe {
    apiversion: String,
    author: Option<String>,
    color: Option<String>,
    head: Option<String>,
    tail: Option<String>,
    version: Option<String>,
}

#[get("/")]
fn me() -> Json<AboutMe> {
    Json(AboutMe {
        apiversion: "1".to_owned(),
        author: Some("coreyja".to_owned()),
        color: Some("#AA66CC".to_owned()),
        head: None,
        tail: None,
        version: None,
    })
}

#[derive(Deserialize, Debug)]
struct Ruleset {
    name: String,
    version: String,
}

#[derive(Deserialize, Debug)]
struct Game {
    id: String,
    ruleset: Option<Ruleset>,
    timeout: u64,
}

#[derive(Deserialize, Debug)]
struct Coordinate {
    x: u64,
    y: u64,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Direction {
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
    Direction::LEFT,
    Direction::DOWN,
];

impl Coordinate {
    fn is_valid_for_board(&self, board: &Board) -> bool {
        self.x < board.width && self.y < board.height
    }

    fn move_in(&self, direction: &Direction) -> Self {
        match direction {
            Direction::UP => Self {
                x: self.x,
                y: self.y + 1,
            },
            Direction::DOWN => Self {
                x: self.x,
                y: self.y - 1,
            },
            Direction::LEFT => Self {
                x: self.x - 1,
                y: self.y,
            },
            Direction::RIGHT => Self {
                x: self.x + 1,
                y: self.y,
            },
        }
    }

    fn possbile_directions(&self, board: &Board) -> HashSet<Direction> {
        ALL_DIRECTIONS
            .iter()
            .filter(|&dir| self.move_in(&dir).is_valid_for_board(board))
            .cloned()
            .collect()
    }
}

#[derive(Deserialize, Debug)]
struct Battlesnake {
    id: String,
    name: String,
    health: u8,
    body: Vec<Coordinate>,
    latency: u16,
    head: Coordinate,
    length: u64,
    shout: Option<String>,
    squad: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Board {
    height: u64,
    width: u64,
    food: Vec<Coordinate>,
    hazards: Vec<Coordinate>,
    snakes: Vec<Battlesnake>,
}

#[derive(Deserialize, Debug)]
struct GameState {
    game: Game,
    turn: u64,
    board: Board,
    you: Battlesnake,
}

#[post("/start", data = "<game_state>")]
fn start(game_state: Json<GameState>) -> Status {
    println!("{:?}", game_state);
    Status::NoContent
}

#[derive(Serialize, Debug)]
struct MoveOutput {
    r#move: String,
    shout: String,
}

#[post("/move", data = "<game_state>")]
fn api_move(game_state: Json<GameState>) -> Json<MoveOutput> {
    let possible = game_state.you.head.possbile_directions(&game_state.board);
    let next_move = possible
        .iter()
        .next()
        .expect("There isn't anywhere we can move that isn't a wall??");

    Json(MoveOutput {
        r#move: next_move.value(),
        shout: "".to_owned(),
    })
}

fn main() {
    rocket::ignite()
        .mount("/", routes![me, start, api_move])
        .launch();
}
