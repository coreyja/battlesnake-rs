#![deny(warnings)]
#![feature(min_specialization)]

#[macro_use]
extern crate serde_derive;

use std::fmt::Debug;

use anyhow::Result;

pub use types::{
    compact_representation::StandardCellBoard4Snakes11x11, types::*, wire_representation::Game,
};

pub mod amphibious_arthur;
pub mod bombastic_bob;
pub mod constant_carter;
pub mod devious_devin_eval;
pub mod eremetic_eric;
pub mod famished_frank;
pub mod gigantic_george;
pub mod hovering_hobbs;
pub mod jump_flooding_snake;

pub mod mcts_snake;

pub mod a_prime;
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

use types::{
    types::{PositionGettableGame, SnakeIDGettableGame, YouDeterminableGame},
    wire_representation::Position,
};

// use crate::{
//     bombastic_bob::BombasticBobFactory, constant_carter::ConstantCarterFactory,
//     eremetic_eric::EremeticEricFactory, famished_frank::FamishedFrankFactory,
//     gigantic_george::GiganticGeorgeFactory, jump_flooding_snake::JumpFloodingSnakeFactory,
// };

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

pub trait MoveableGame: SnakeIDGettableGame + PositionGettableGame {
    fn move_to(
        &mut self,
        coor: &Self::NativePositionType,
        snake_id: &Self::SnakeIDType,
    ) -> SnakeMove<Self::SnakeIDType>;
    fn reverse_move(&mut self, m: SnakeMove<Self::SnakeIDType>);

    fn nature_move(&mut self) -> Vec<NatureMove>;
    fn reverse_nature(&mut self, m: NatureMove);
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
                s.body.push_back(*s.body.back().unwrap());
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
                snake.body.pop_back();
                self.board.food.insert(food_pos, food_coor);
            }
        }
    }

    fn reverse_move(&mut self, m: SnakeMove<Self::SnakeIDType>) {
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
                to_move.body.push_back(tail);
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
pub type BoxedFactory = Box<dyn BattlesnakeFactory + Send + Sync>;

pub trait BattlesnakeAI {
    fn end(&self) {}
    fn make_move(&self) -> Result<MoveOutput>;
}

pub trait BattlesnakeFactory {
    fn name(&self) -> String;
    fn create_from_wire_game(&self, game: Game) -> BoxedSnake;

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

pub use battlesnake_minimax::paranoid::MinimaxSnake;
use battlesnake_minimax::Instruments;

use crate::{
    amphibious_arthur::AmphibiousArthurFactory, bombastic_bob::BombasticBobFactory,
    constant_carter::ConstantCarterFactory, eremetic_eric::EremeticEricFactory,
    famished_frank::FamishedFrankFactory, gigantic_george::GiganticGeorgeFactory,
    jump_flooding_snake::JumpFloodingSnakeFactory, mcts_snake::MctsSnakeFactory,
};

impl<T, ScoreType, const N_SNAKES: usize> BattlesnakeAI for MinimaxSnake<T, ScoreType, N_SNAKES>
where
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + NeighborDeterminableGame
        + NeckQueryableGame
        // + ReasonableMoveDeterminableGame
        + SimulableGame<Instruments, N_SNAKES>
        + Clone
        + Sync
        + Copy
        + FoodGettableGame
        + Send,
    T::SnakeIDType: Copy + Send + Sync,
    ScoreType: Clone + Debug + PartialOrd + Ord + Send + Sync + Copy,
{
    fn make_move(&self) -> Result<MoveOutput> {
        let m: Move = Self::make_move(self);

        Ok(MoveOutput {
            r#move: format!("{}", m),
            shout: None,
        })
    }
}

pub fn all_factories() -> Vec<BoxedFactory> {
    vec![
        Box::new(AmphibiousArthurFactory {}),
        Box::new(BombasticBobFactory {}),
        Box::new(ConstantCarterFactory {}),
        Box::new(devious_devin_eval::Factory {}),
        Box::new(EremeticEricFactory {}),
        Box::new(FamishedFrankFactory {}),
        Box::new(GiganticGeorgeFactory {}),
        Box::new(JumpFloodingSnakeFactory {}),
        Box::new(hovering_hobbs::Factory {}),
        Box::new(MctsSnakeFactory {}),
    ]
}
