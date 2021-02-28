#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;

use rocket::http::Status;
use rocket_contrib::json::Json;
use std::collections::HashSet;

mod amphibious_arthur;

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
    x: u64,
    y: u64,
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
    height: u64,
    width: u64,
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
    fn move_to(&self, coor: &Coordinate) -> Self {
        let mut clonned = self.clone();

        clonned.you.body.insert(0, coor.clone());
        let removed = clonned.you.body.pop();

        if clonned.you.health > 0 {
            clonned.you.health -= 1;
        }

        if let Some(pos) = clonned.board.food.iter().position(|x| x == coor) {
            clonned.board.food.remove(pos);
            clonned.you.health = 100;

            if let Some(c) = removed {
                clonned.you.body.push(c);
            }
        }

        clonned
    }
}

#[derive(Serialize, Debug)]
pub struct MoveOutput {
    r#move: String,
    shout: Option<String>,
}

fn main() {
    rocket::ignite()
        .mount(
            "/amphibious-arthur",
            routes![
                amphibious_arthur::me,
                amphibious_arthur::start,
                amphibious_arthur::api_move,
                amphibious_arthur::end,
            ],
        )
        .launch();
}
