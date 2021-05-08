#[macro_use]
extern crate serde_derive;

extern crate rand;

use std::collections::HashSet;

pub mod amphibious_arthur;
pub mod bombastic_bob;
pub mod constant_carter;
pub mod devious_devin;
pub mod opentelemetry_rocket;

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

use std::sync::Arc;

pub type BoxedSnake = Box<dyn BattlesnakeAI + Send + Sync>;
pub trait BattlesnakeAI {
    fn start(&self) {}
    fn end(&self) {}
    fn make_move(&self, state: GameState) -> Result<MoveOutput, Box<dyn std::error::Error>>;
    fn name(&self) -> String;

    fn about(&self) -> AboutMe {
        Default::default()
    }
}