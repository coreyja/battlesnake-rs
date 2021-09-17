#[macro_use]
extern crate serde_derive;

use std::{collections::HashSet, convert::TryInto};

pub use battlesnake_game_types::compact_representation::CellBoard4Snakes11x11;
pub use battlesnake_game_types::types::Move;
pub use battlesnake_game_types::wire_representation::Game;

pub mod a_prime;
pub mod amphibious_arthur;
pub mod bombastic_bob;
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

use battlesnake_game_types::{
    compact_representation::{CellBoard, CellIndex, CellNum},
    types::{PositionGettableGame, SnakeIDGettableGame},
    wire_representation::Position,
};

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
        food_coor: Position,
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

pub type BoxedSnake<T> = Box<dyn BattlesnakeAI<T> + Send + Sync>;

pub trait BattlesnakeAI<T> {
    fn start(&self) {}
    fn end(&self, _state: T) {}
    fn make_move(&self, state: T) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>>;
    fn name(&self) -> String;

    fn about(&self) -> AboutMe {
        Default::default()
    }
}

pub trait SnakeTailPushableGame: SnakeIDGettableGame + PositionGettableGame {
    fn push_tail(&mut self, snake_id: &Self::SnakeIDType, pos: Self::NativePositionType);
}

impl SnakeTailPushableGame for Game {
    fn push_tail(&mut self, snake_id: &Self::SnakeIDType, pos: Self::NativePositionType) {
        self.board
            .snakes
            .iter_mut()
            .find(|s| &s.id == snake_id)
            .unwrap()
            .body
            .push_back(pos)
    }
}

pub trait TurnDeterminableGame {
    fn turn(&self) -> u64;
}

impl TurnDeterminableGame for Game {
    fn turn(&self) -> u64 {
        self.turn.try_into().unwrap()
    }
}

pub trait SnakeBodyGettableGame: PositionGettableGame + SnakeIDGettableGame {
    fn get_snake_body_vec(&self, snake_id: &Self::SnakeIDType) -> Vec<Self::NativePositionType>;
}

impl SnakeBodyGettableGame for Game {
    fn get_snake_body_vec(&self, snake_id: &Self::SnakeIDType) -> Vec<Self::NativePositionType> {
        self.board
            .snakes
            .iter()
            .find(|s| &s.id == snake_id)
            .unwrap()
            .body
            .clone()
            .into_iter()
            .collect()
    }
}

pub trait ShoutGettableGame: SnakeIDGettableGame {
    fn get_shout(&self, snake_id: &Self::SnakeIDType) -> Option<String>;
}

impl ShoutGettableGame for Game {
    fn get_shout(&self, snake_id: &Self::SnakeIDType) -> Option<String> {
        self.board
            .snakes
            .iter()
            .find(|s| &s.id == snake_id)
            .unwrap()
            .shout
            .clone()
    }
}

pub trait SizeDeterminableGame {
    fn get_width(&self) -> u32;
    fn get_height(&self) -> u32;
}

impl SizeDeterminableGame for Game {
    fn get_width(&self) -> u32 {
        self.board.width
    }

    fn get_height(&self) -> u32 {
        self.board.height
    }
}

pub trait NeighborDeterminableGame: PositionGettableGame {
    fn neighbors(&self, pos: &Self::NativePositionType) -> Vec<Self::NativePositionType>;

    fn possible_moves(
        &self,
        pos: &Self::NativePositionType,
    ) -> Vec<(Move, Self::NativePositionType)>;
}

impl<T: CellNum, const BOARD_SIZE: usize, const MAX_SNAKES: usize> NeighborDeterminableGame
    for CellBoard<T, BOARD_SIZE, MAX_SNAKES>
{
    fn possible_moves(
        &self,
        pos: &Self::NativePositionType,
    ) -> Vec<(Move, Self::NativePositionType)> {
        let width = ((11 * 11) as f32).sqrt() as u8;

        Move::all()
            .into_iter()
            .map(|mv| {
                let head_pos = pos.into_position(width);
                let new_head = head_pos.add_vec(mv.to_vector());
                let ci = CellIndex::new(new_head, width);

                (mv, new_head, ci)
            })
            .filter(|(_mv, new_head, _)| !self.off_board(*new_head, width))
            .map(|(mv, _, ci)| (mv, ci))
            .collect()
    }

    fn neighbors(&self, pos: &Self::NativePositionType) -> std::vec::Vec<Self::NativePositionType> {
        let width = ((11 * 11) as f32).sqrt() as u8;

        Move::all()
            .into_iter()
            .map(|mv| {
                let head_pos = pos.into_position(width);
                let new_head = head_pos.add_vec(mv.to_vector());
                let ci = CellIndex::new(new_head, width);

                (new_head, ci)
            })
            .filter(|(new_head, _)| !self.off_board(*new_head, width))
            .map(|(_, ci)| ci)
            .collect()
    }
}

impl NeighborDeterminableGame for Game {
    fn neighbors(&self, pos: &Self::NativePositionType) -> Vec<Self::NativePositionType> {
        Move::all()
            .into_iter()
            .map(|mv| pos.add_vec(mv.to_vector()))
            .filter(|new_head| !self.off_board(*new_head))
            .collect()
    }

    fn possible_moves(
        &self,
        pos: &Self::NativePositionType,
    ) -> Vec<(Move, Self::NativePositionType)> {
        Move::all()
            .into_iter()
            .map(|mv| (mv, pos.add_vec(mv.to_vector())))
            .filter(|(_mv, new_head)| !self.off_board(*new_head))
            .collect()
    }
}
