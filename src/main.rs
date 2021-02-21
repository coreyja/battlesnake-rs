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

#[derive(Deserialize, Debug, Clone)]
struct Ruleset {
    name: String,
    version: String,
}

#[derive(Deserialize, Debug, Clone)]
struct Game {
    id: String,
    ruleset: Option<Ruleset>,
    timeout: u64,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Hash, Clone)]
struct Coordinate {
    x: u64,
    y: u64,
}

#[derive(Clone, PartialEq, Eq, Hash, Copy, Debug)]
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
    fn move_in(&self, direction: &Direction, board: &Board) -> Option<Self> {
        let mut x = self.x;
        let mut y = self.y;

        match direction {
            Direction::UP => {
                if self.y + 1 >= board.height {
                    return None;
                }

                y += 1;
            }
            Direction::DOWN => {
                if self.y == 0 {
                    return None;
                }

                y -= 1;
            }
            Direction::LEFT => {
                if self.x == 0 {
                    return None;
                }

                x -= 1;
            }
            Direction::RIGHT => {
                if self.x + 1 >= board.width {
                    return None;
                }

                x += 1;
            }
        };

        Some(Self { x, y })
    }

    fn possbile_moves(&self, board: &Board) -> HashSet<(Direction, Coordinate)> {
        ALL_DIRECTIONS
            .iter()
            .cloned()
            .filter_map(|dir| self.move_in(&dir, &board).map(|coor| (dir, coor)))
            .collect()
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Battlesnake {
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
struct Board {
    height: u64,
    width: u64,
    food: Vec<Coordinate>,
    hazards: Vec<Coordinate>,
    snakes: Vec<Battlesnake>,
}

#[derive(Deserialize, Debug, Clone)]
struct GameState {
    game: Game,
    turn: u64,
    board: Board,
    you: Battlesnake,
}

impl GameState {
    fn move_in(&self, dir: &Direction, coor: &Coordinate) -> Self {
        let mut clonned = self.clone();

        clonned.you.body.insert(0, coor.clone());
        clonned.you.body.pop();

        clonned
    }
}

#[post("/start")]
fn start() -> Status {
    Status::NoContent
}

#[post("/end")]
fn end() -> Status {
    Status::NoContent
}

#[derive(Serialize, Debug)]
struct MoveOutput {
    r#move: String,
    shout: Option<String>,
}

fn score(game_state: &GameState, dir: &Direction, coor: &Coordinate, times_to_recurse: u8) -> u64 {
    if game_state.you.body.contains(coor) {
        return 0;
    }

    if times_to_recurse == 0 {
        return 1;
    }

    coor.possbile_moves(&game_state.board)
        .iter()
        .map(|(d, c)| score(&game_state.move_in(dir, coor), &d, &c, times_to_recurse - 1))
        .sum()
}

#[post("/move", data = "<game_state>")]
fn api_move(game_state: Json<GameState>) -> Json<MoveOutput> {
    let possible = game_state.you.possbile_moves(&game_state.board);
    let next_move = possible
        .iter()
        .max_by_key(|(dir, coor)| score(&game_state, &dir, &coor, 5));

    let stuck_response: MoveOutput = MoveOutput {
        r#move: Direction::UP.value(),
        shout: Some("Oh NO we are stuck".to_owned()),
    };
    let output = next_move.map_or(stuck_response, |(dir, _coor)| MoveOutput {
        r#move: dir.value(),
        shout: None,
    });
    Json(output)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![me, start, api_move, end])
        .launch();
}
