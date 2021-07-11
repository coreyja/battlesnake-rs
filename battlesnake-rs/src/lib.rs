#[macro_use]
extern crate serde_derive;

use std::{collections::HashSet, convert::TryInto};

pub mod a_prime;
pub mod amphibious_arthur;
pub mod bombastic_bob;
pub mod constant_carter;
pub mod devious_devin;
pub mod eremetic_eric;
pub mod famished_frank;
pub mod flood_fill;

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

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq)]
pub struct Ruleset {
    name: String,
    version: String,
}

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq)]
pub struct Game {
    id: serde_json::Value,
    ruleset: Option<Ruleset>,
    timeout: u64,
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

    fn on_wall(&self, board: &Board) -> bool {
        let (width, height): (i64, i64) = (board.width.into(), board.height.into());

        self.x == 0 || self.y == 0 || self.x + 1 == width || self.y + 1 == height
    }

    fn to_usize(&self) -> (usize, usize) {
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

impl Coordinate {
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

    fn possible_moves(&self, board: &Board) -> Vec<(Direction, Coordinate)> {
        ALL_DIRECTIONS
            .iter()
            .cloned()
            .map(|dir| (dir, self.move_in(&dir)))
            .filter(|(_, coor)| coor.valid(board))
            .collect()
    }

    fn neighbors(&self, board: &Board) -> Vec<Coordinate> {
        self.possible_moves(board)
            .into_iter()
            .map(|x| x.1)
            .collect()
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

use rand::seq::SliceRandom;

impl Battlesnake {
    fn possible_moves(&self, board: &Board) -> Vec<(Direction, Coordinate)> {
        self.head.possible_moves(board)
    }

    fn random_possible_move(&self, board: &Board) -> Option<(Direction, Coordinate)> {
        let body_set: HashSet<&Coordinate> = self.body.iter().collect();
        let possible_moves = self
            .head
            .possible_moves(board)
            .iter()
            .filter(|(_dir, coor)| !body_set.contains(coor))
            .cloned()
            .collect::<Vec<_>>();

        possible_moves.choose(&mut rand::thread_rng()).cloned()
    }

    fn tail(&self) -> Coordinate {
        self.body[self.body.len() - 1]
    }
}

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq)]
pub struct Board {
    height: u32,
    width: u32,
    food: Vec<Coordinate>,
    hazards: Vec<Coordinate>,
    snakes: Vec<Battlesnake>,
}

impl Board {
    pub fn empty_coordiates(&self) -> Vec<Coordinate> {
        let filled_coordinates = self.filled_coordinates();
        self.all_coordinates()
            .into_iter()
            .filter(|c| !filled_coordinates.contains(c))
            .collect()
    }

    pub fn all_coordinates(&self) -> Vec<Coordinate> {
        let mut all = vec![];

        for x in 0..self.width {
            for y in 0..self.height {
                all.push(Coordinate {
                    x: x.into(),
                    y: y.into(),
                })
            }
        }

        all
    }

    pub fn filled_coordinates(&self) -> Vec<Coordinate> {
        let mut filled = vec![];

        filled.append(&mut self.food.clone());

        for s in &self.snakes {
            filled.append(&mut s.body.clone());
        }

        filled
    }

    pub fn to_grid(&self) -> BoardGrid {
        let mut grid: Vec<Vec<Option<BoardGridItem>>> =
            vec![vec![None; self.width.try_into().unwrap()]; self.height.try_into().unwrap()];

        for h in &self.hazards {
            let (x, y) = h.to_usize();
            grid[x][y] = Some(BoardGridItem::Hazard);
        }

        for f in &self.food {
            let (x, y) = f.to_usize();
            grid[x][y] = Some(BoardGridItem::Food);
        }

        for s in &self.snakes {
            for b in &s.body {
                let (x, y) = b.to_usize();
                grid[x][y] = Some(BoardGridItem::Snake(&s.id));
            }
        }

        BoardGrid(grid)
    }
}

#[derive(Clone, Copy)]
pub enum BoardGridItem<'snake_id> {
    Snake(&'snake_id str),
    Food,
    Hazard,
}

pub struct BoardGrid<'a>(Vec<Vec<Option<BoardGridItem<'a>>>>);

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq)]
pub struct GameState {
    game: Game,
    turn: u64,
    board: Board,
    you: Battlesnake,
}

pub enum MoveResult {
    AteFood(i16, Coordinate, usize), // old_health, food_coor, food_pos
    MovedTail(i16, Coordinate),      //old_health, tail_was
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
        to_move.body.insert(0, *coor);

        let old_health = to_move.health;
        to_move.health -= 1;

        let move_result = if let Some(pos) = self.board.food.iter().position(|x| x == coor) {
            self.board.food.remove(pos);
            to_move.health = 100;
            MoveResult::AteFood(old_health, *coor, pos)
        } else {
            MoveResult::MovedTail(old_health, to_move.body.pop().unwrap())
        };

        if self.board.hazards.contains(coor) {
            to_move.health -= 15;
        }

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
            MoveResult::AteFood(old_health, food_coor, food_pos) => {
                to_move.health = old_health;
                self.board.food.insert(food_pos, food_coor);
            }
            MoveResult::MovedTail(old_health, tail) => {
                to_move.health = old_health;
                to_move.body.push(tail);
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
    fn end(&self, _state: GameState) {}
    fn make_move(
        &self,
        state: GameState,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>>;
    fn name(&self) -> String;

    fn about(&self) -> AboutMe {
        Default::default()
    }
}

#[cfg(test)]
mod tests {
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

        assert_eq!(
            coor.possible_moves(&board),
            vec![
                (Direction::Up, Coordinate { x: 1, y: 2 }),
                (Direction::Right, Coordinate { x: 2, y: 1 }),
                (Direction::Down, Coordinate { x: 1, y: 0 }),
                (Direction::Left, Coordinate { x: 0, y: 1 }),
            ]
        );
    }
}
