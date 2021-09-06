#[macro_use]
extern crate serde_derive;

use std::{collections::HashSet, convert::TryInto};

pub use battlesnake_game_types::types::Move;
pub use battlesnake_game_types::wire_representation::Game;

pub mod amphibious_arthur;
pub mod bombastic_bob;
pub mod compact_a_prime;
pub mod constant_carter;
pub mod devious_devin;
pub mod eremetic_eric;
pub mod famished_frank;
pub mod gigantic_george;

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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct Coordinate {
    x: i64,
    y: i64,
}

impl Coordinate {
    fn dist_from(&self, other: &Self) -> i64 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    fn to_usize(self) -> (usize, usize) {
        (self.x.try_into().unwrap(), self.y.try_into().unwrap())
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Copy, Debug, Serialize)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    fn value(&self) -> String {
        match self {
            Direction::Up => "up",
            Direction::Right => "right",
            Direction::Left => "left",
            Direction::Down => "down",
        }
        .to_owned()
    }
}

const ALL_DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Right,
    Direction::Down,
    Direction::Left,
];

impl<'a> Coordinate {
    fn valid(&self, board: &Board) -> bool {
        self.x >= 0 && self.x < board.width.into() && self.y >= 0 && self.y < board.height.into()
    }

    fn move_in(&self, direction: &Direction) -> Self {
        let mut x = self.x;
        let mut y = self.y;

        match direction {
            Direction::Up => {
                y += 1;
            }
            Direction::Down => {
                y -= 1;
            }
            Direction::Left => {
                x -= 1;
            }
            Direction::Right => {
                x += 1;
            }
        };

        Self { x, y }
    }

    fn possible_moves(
        &self,
        board: &'a Board,
    ) -> Box<dyn Iterator<Item = (Direction, Coordinate)> + 'a> {
        let cloned = *self;
        Box::new(
            ALL_DIRECTIONS
                .iter()
                .cloned()
                .map(move |dir| (dir, cloned.move_in(&dir)))
                .filter(move |(_, coor)| coor.valid(board)),
        )
    }

    fn neighbors(&self, board: &'a Board) -> Box<dyn Iterator<Item = Coordinate> + 'a> {
        Box::new(self.possible_moves(board).into_iter().map(|x| x.1))
    }
}

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq)]
pub struct Battlesnake {
    id: String,
    name: String,
    health: i16,
    body: Vec<Coordinate>,
    latency: serde_json::Value,
    head: Coordinate,
    length: u64,
    shout: Option<String>,
    squad: Option<String>,
}

use battlesnake_game_types::{
    types::SnakeIDGettableGame,
    wire_representation::{Board, Position},
};
use rand::seq::SliceRandom;

impl Battlesnake {
    fn random_possible_move(&self, board: &Board) -> Option<(Direction, Coordinate)> {
        let body_set: HashSet<&Coordinate> = self.body.iter().collect();
        let possible_moves = self
            .head
            .possible_moves(board)
            .filter(|(_dir, coor)| !body_set.contains(coor))
            .collect::<Vec<_>>();

        possible_moves.choose(&mut rand::thread_rng()).cloned()
    }

    fn tail(&self) -> Coordinate {
        self.body[self.body.len() - 1]
    }
}

pub enum MoveResult {
    MovedTail(i32, Position), //old_health, tail_was
}

pub struct SnakeMove<T> {
    pub snake_id: T,
    pub move_result: MoveResult,
}

pub enum NatureMove {
    AteFood {
        snake_id: String,
        old_health: i32,
        food_coor: Coordinate,
        food_pos: usize,
    },
}

trait MoveableGame: SnakeIDGettableGame {
    fn move_to(
        &mut self,
        coor: &Position,
        snake_id: &Self::SnakeIDType,
    ) -> SnakeMove<Self::SnakeIDType>;
}

impl MoveableGame for Game {
    fn move_to(
        &mut self,
        coor: &Position,
        snake_id: &Self::SnakeIDType,
    ) -> SnakeMove<Self::SnakeIDType> {
        let to_move = self
            .board
            .snakes
            .iter_mut()
            .find(|s| &s.id == snake_id)
            .unwrap();
        to_move.body.insert(0, *coor);

        let old_health = to_move.health;
        to_move.health -= 1;

        let move_result = MoveResult::MovedTail(old_health, to_move.body.pop_back().unwrap());

        if self.board.hazards.contains(coor) {
            to_move.health -= 15;
        }

        let snake_id = snake_id.to_owned();
        SnakeMove {
            snake_id,
            move_result,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct MoveOutput {
    r#move: String,
    shout: Option<String>,
}

pub type BoxedSnake = Box<dyn BattlesnakeAI + Send + Sync>;
pub trait BattlesnakeAI {
    fn start(&self) {}
    fn end(&self, _state: Game) {}
    fn make_move(
        &self,
        state: Game,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>>;
    fn name(&self) -> String;

    fn about(&self) -> AboutMe {
        Default::default()
    }
}

#[cfg(test)]
mod tests {
    use battlesnake_game_types::wire_representation::Board;

    use super::*;

    #[test]
    fn test_possible_moves() {
        let coor = Coordinate { x: 1, y: 1 };
        let board = Board {
            height: 11,
            width: 11,
            food: vec![],
            hazards: vec![],
            snakes: vec![],
        };

        let x: Vec<(Direction, Coordinate)> = coor.possible_moves(&board).collect();
        assert_eq!(
            x,
            vec![
                (Direction::Up, Coordinate { x: 1, y: 2 }),
                (Direction::Right, Coordinate { x: 2, y: 1 }),
                (Direction::Down, Coordinate { x: 1, y: 0 }),
                (Direction::Left, Coordinate { x: 0, y: 1 }),
            ]
        );
    }
}
