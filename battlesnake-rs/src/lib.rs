#[macro_use]
extern crate serde_derive;

use std::{collections::HashSet, convert::TryInto};

pub mod a_prime;
pub mod amphibious_arthur;
pub mod bombastic_bob;
pub mod compact_a_prime;
pub mod constant_carter;
pub mod devious_devin;
pub mod eremetic_eric;
pub mod famished_frank;
pub mod flood_fill;
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

impl<'a> BoardGrid<'a> {
    fn is_full(&self) -> bool {
        for inner in &self.0 {
            for item in inner {
                if item.is_none() {
                    return false;
                }
            }
        }

        true
    }
}

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq)]
pub struct GameState {
    game: Game,
    turn: u64,
    board: Board,
    you: Battlesnake,
}

pub enum MoveResult {
    MovedTail(i16, Coordinate), //old_health, tail_was
}
pub struct Move {
    snake_id: String,
    move_result: MoveResult,
}

pub enum NatureMove {
    AteFood {
        snake_id: String,
        old_health: i16,
        food_coor: Coordinate,
        food_pos: usize,
    },
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

        let move_result = MoveResult::MovedTail(old_health, to_move.body.pop().unwrap());

        if self.board.hazards.contains(coor) {
            to_move.health -= 15;
        }

        let snake_id = snake_id.to_owned();
        Move {
            snake_id,
            move_result,
        }
    }

    fn nature_move(&mut self) -> Vec<NatureMove> {
        let mut moves = vec![];

        for s in self.board.snakes.iter_mut() {
            if let Some(pos) = self.board.food.iter().position(|x| x == &s.body[0]) {
                moves.push(NatureMove::AteFood {
                    snake_id: s.id.clone(),
                    old_health: s.health,
                    food_coor: self.board.food.remove(pos),
                    food_pos: pos,
                });
                s.health = 100;
                s.body.push(*s.body.last().unwrap());
            }
        }

        moves.reverse();
        moves
    }

    fn reverse_nature(&mut self, m: NatureMove) {
        match m {
            NatureMove::AteFood {
                snake_id,
                old_health,
                food_coor,
                food_pos,
            } => {
                let snake = self
                    .board
                    .snakes
                    .iter_mut()
                    .find(|s| s.id == snake_id)
                    .unwrap();
                snake.health = old_health;
                snake.body.pop();
                self.board.food.insert(food_pos, food_coor);
            }
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
