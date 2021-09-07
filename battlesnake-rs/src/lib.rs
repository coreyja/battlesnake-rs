#[macro_use]
extern crate serde_derive;

use std::collections::HashSet;

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

use battlesnake_game_types::{types::SnakeIDGettableGame, wire_representation::Position};

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
