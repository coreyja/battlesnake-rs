use crate::a_prime::{APrimeCalculable, ClosestFoodCalculable};
use crate::*;
use battlesnake_minimax::paranoid::MinimaxSnake;

use types::types::*;

pub struct Factory;

#[derive(Serialize, PartialEq, PartialOrd, Ord, Eq, Debug, Copy, Clone)]
pub enum ScoreEndState {
    /// depth: i64
    Lose(i64),
    /// depth: i64
    Tie(i64),
    /// difference_in_snake_length: u16, negative_distance_to_nearest_food: Option<i32>, health: u8
    ShorterThanOpponent(i64, Option<i32>, i64),
    /// negative_distance_to_opponent: Option<i64>, difference_in_snake_length: i64, health: u8
    LongerThanOpponent(Option<i32>, i64, i64),
    /// depth: i64
    Win(i64),
}

impl ScoreEndState {
    pub fn terminal_depth(&self) -> Option<i64> {
        match &self {
            ScoreEndState::Win(d) => Some(-d),
            ScoreEndState::Tie(d) | ScoreEndState::Lose(d) => Some(*d),
            _ => None,
        }
    }
}

pub fn score<
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + LengthGettableGame
        + HealthGettableGame
        + HeadGettableGame
        + APrimeCalculable
        + FoodGettableGame,
>(
    node: &T,
) -> ScoreEndState {
    let me_id = node.you_id();
    let opponents: Vec<T::SnakeIDType> = node
        .get_snake_ids()
        .into_iter()
        .filter(|x| x != me_id)
        .collect();

    let opponent_heads: Vec<_> = opponents
        .iter()
        .map(|s| node.get_head_as_native_position(s))
        .collect();
    let my_head = node.get_head_as_native_position(me_id);

    let my_length = node.get_length_i64(me_id);

    let max_opponent_length = opponents
        .iter()
        .map(|o| node.get_length_i64(o))
        .max()
        .unwrap();
    let length_difference = (my_length as i64) - (max_opponent_length as i64);
    let my_health = node.get_health_i64(me_id);

    if max_opponent_length >= my_length || my_health < 20 {
        let negative_closest_food_distance = node.dist_to_closest_food(&my_head, None).map(|x| -x);

        return ScoreEndState::ShorterThanOpponent(
            length_difference,
            negative_closest_food_distance,
            my_health.max(50),
        );
    }

    let negative_distance_to_opponent = node
        .shortest_distance(&my_head, &opponent_heads, None)
        .map(|dist| -dist);

    ScoreEndState::LongerThanOpponent(
        negative_distance_to_opponent,
        length_difference.max(4),
        my_health.max(50),
    )
}

impl Factory {
    pub fn new() -> Self {
        Self
    }

    pub fn create(&self, game: Game) -> BoxedSnake {
        let game_info = game.game.clone();
        let turn = game.turn;
        let name = "devious-devin";

        if game_info.ruleset.name == "wrapped" {
            use types::compact_representation::wrapped::*;

            match ToBestCellBoard::to_best_cell_board(game).unwrap() {
                BestCellBoard::Tiny(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::SmallExact(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::Standard(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::MediumExact(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::LargestU8(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::LargeExact(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::ArcadeMaze(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::ArcadeMaze8Snake(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::Large(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::Silly(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
            }
        } else {
            use types::compact_representation::standard::*;

            match ToBestCellBoard::to_best_cell_board(game).unwrap() {
                BestCellBoard::Tiny(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::SmallExact(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::Standard(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::MediumExact(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::LargestU8(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::LargeExact(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::ArcadeMaze(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::ArcadeMaze8Snake(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::Large(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
                BestCellBoard::Silly(game) => {
                    Box::new(MinimaxSnake::from_fn(*game, game_info, turn, &score, name))
                }
            }
        }
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}

impl BattlesnakeFactory for Factory {
    fn name(&self) -> String {
        "devious-devin".to_owned()
    }

    fn create_from_wire_game(&self, game: Game) -> BoxedSnake {
        self.create(game)
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            apiversion: "1".to_owned(),
            author: Some("coreyja".to_owned()),
            color: Some("#99cc00".to_owned()),
            head: Some("trans-rights-scarf".to_owned()),
            tail: Some("rbc-necktie".to_owned()),
            version: None,
        }
    }
}

#[cfg(test)]
mod tests {}
