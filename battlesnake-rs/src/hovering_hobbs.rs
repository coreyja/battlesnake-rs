use std::time::Duration;

use crate::a_prime::APrimeCalculable;
use crate::flood_fill::spread_from_head::{Scores, SpreadFromHead};
use crate::flood_fill::spread_from_head_arcade_maze::SpreadFromHeadArcadeMaze;
use crate::*;

use battlesnake_minimax::{
    paranoid::{move_ordering::MoveOrdering, SnakeOptions},
    ParanoidMinimaxSnake,
};
use decorum::N64;
use types::types::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Score {
    LowOnHealth(Option<i32>, N64),
    FloodFill(N64),
}

pub fn standard_score<BoardType, CellType, const MAX_SNAKES: usize>(node: &BoardType) -> Score
where
    BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + YouDeterminableGame
        + SpreadFromHead<CellType, MAX_SNAKES>
        + APrimeCalculable
        + HeadGettableGame
        + HazardQueryableGame
        + HealthGettableGame
        + LengthGettableGame
        + FoodGettableGame
        + MaxSnakes<MAX_SNAKES>,
{
    let scores = if node.get_hazard_damage().is_positive() {
        Scores {
            food: 5,
            hazard: 1,
            empty: 5,
        }
    } else {
        Scores {
            food: 5,
            hazard: 5,
            empty: 1,
        }
    };
    let square_counts = node.squares_per_snake_with_scores(5, scores);

    let me = node.you_id();
    let my_space: f64 = square_counts[me.as_usize()] as f64;
    let total_space: f64 = square_counts.iter().sum::<u16>() as f64;
    let my_ratio = N64::from(my_space / total_space);

    if node.get_health_i64(me) < 40 {
        let dist = node
            .shortest_distance(
                &node.get_head_as_native_position(me),
                &node.get_all_food_as_native_positions(),
                None,
            )
            .map(|x| -x);
        return Score::LowOnHealth(dist, my_ratio);
    }

    Score::FloodFill(my_ratio)
}

pub fn arcade_maze_score<BoardType, CellType, const MAX_SNAKES: usize>(node: &BoardType) -> Score
where
    BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + YouDeterminableGame
        + SpreadFromHead<CellType, MAX_SNAKES>
        + SpreadFromHeadArcadeMaze<CellType, MAX_SNAKES>
        + APrimeCalculable
        + HeadGettableGame
        + HazardQueryableGame
        + HealthGettableGame
        + LengthGettableGame
        + FoodGettableGame
        + MaxSnakes<MAX_SNAKES>,
{
    let square_counts = node.squares_per_snake_hazard_maze(8);

    let me = node.you_id();
    let my_space: f64 = square_counts[me.as_usize()] as f64;
    let total_space: f64 = square_counts.iter().sum::<u8>() as f64;
    let my_ratio = N64::from(my_space / total_space);

    if node.get_health_i64(me) < 40 {
        let dist = node
            .shortest_distance(
                &node.get_head_as_native_position(me),
                &node.get_all_food_as_native_positions(),
                None,
            )
            .map(|x| -x);
        return Score::LowOnHealth(dist, my_ratio);
    }

    let me_length = node.get_length_i64(me);
    let max_opponent_length = node
        .get_snake_ids()
        .iter()
        .filter(|&x| x != me)
        .map(|&x| node.get_length_i64(&x))
        .max()
        .unwrap();
    let length_diff = me_length - max_opponent_length;
    let capped_diff = length_diff.min(3);
    let length_diff_multipier: f64 = 0.05 * capped_diff as f64;

    Score::FloodFill(my_ratio * length_diff_multipier)
}

pub struct Factory;

#[macro_export]
macro_rules! build_from_best_cell_board {
    ( $wire_game:expr, $game_info:expr, $turn:expr, $score_function:ident, $name:expr, $options:expr ) => {{
        let game = $wire_game;
        let game_info = $game_info;
        let turn = $turn;
        let name = $name;
        let options = $options;

        if game_info.ruleset.name == "wrapped" {
            use types::compact_representation::wrapped::*;

            build_from_best_cell_board_inner!(game, game_info, turn, $score_function, name, options)
        } else {
            use types::compact_representation::standard::*;

            build_from_best_cell_board_inner!(game, game_info, turn, $score_function, name, options)
        }
    }};
}

#[macro_export]
macro_rules! build_from_best_cell_board_inner {
    ( $wire_game:expr, $game_info:expr, $turn:expr, $score_function:ident, $name:expr, $options:expr ) => {{
        {
            let game = $wire_game;
            let game_info = $game_info;
            let turn = $turn;
            let name = $name;
            let options = $options;

            match ToBestCellBoard::to_best_cell_board(game).unwrap() {
                BestCellBoard::Tiny(game) => Box::new(ParanoidMinimaxSnake::new(
                    *game,
                    game_info,
                    turn,
                    &$score_function,
                    name,
                    options,
                )),
                BestCellBoard::SmallExact(game) => Box::new(ParanoidMinimaxSnake::new(
                    *game,
                    game_info,
                    turn,
                    &$score_function,
                    name,
                    options,
                )),
                BestCellBoard::Standard(game) => Box::new(ParanoidMinimaxSnake::new(
                    *game,
                    game_info,
                    turn,
                    &$score_function,
                    name,
                    options,
                )),
                BestCellBoard::MediumExact(game) => Box::new(ParanoidMinimaxSnake::new(
                    *game,
                    game_info,
                    turn,
                    &$score_function,
                    name,
                    options,
                )),
                BestCellBoard::LargestU8(game) => Box::new(ParanoidMinimaxSnake::new(
                    *game,
                    game_info,
                    turn,
                    &$score_function,
                    name,
                    options,
                )),
                BestCellBoard::LargeExact(game) => Box::new(ParanoidMinimaxSnake::new(
                    *game,
                    game_info,
                    turn,
                    &$score_function,
                    name,
                    options,
                )),
                BestCellBoard::ArcadeMaze(game) => Box::new(ParanoidMinimaxSnake::new(
                    *game,
                    game_info,
                    turn,
                    &$score_function,
                    name,
                    options,
                )),
                BestCellBoard::ArcadeMaze8Snake(game) => Box::new(ParanoidMinimaxSnake::new(
                    *game,
                    game_info,
                    turn,
                    &$score_function,
                    name,
                    options,
                )),
                BestCellBoard::Large(game) => Box::new(ParanoidMinimaxSnake::new(
                    *game,
                    game_info,
                    turn,
                    &$score_function,
                    name,
                    options,
                )),
                BestCellBoard::Silly(game) => Box::new(ParanoidMinimaxSnake::new(
                    *game,
                    game_info,
                    turn,
                    &$score_function,
                    name,
                    options,
                )),
            }
        }
    }};
}

impl BattlesnakeFactory for Factory {
    fn name(&self) -> String {
        "hovering-hobbs".to_owned()
    }

    fn create_from_wire_game(&self, game: Game) -> BoxedSnake {
        let game_info = game.game.clone();
        let turn = game.turn;

        let name = "hovering-hobbs";

        let options: SnakeOptions = SnakeOptions {
            network_latency_padding: Duration::from_millis(50),
            move_ordering: MoveOrdering::BestFirst,
        };

        if game.is_arcade_maze_map() {
            build_from_best_cell_board!(game, game_info, turn, arcade_maze_score, name, options)
        } else {
            build_from_best_cell_board!(game, game_info, turn, standard_score, name, options)
        }
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            apiversion: "1".to_owned(),
            author: Some("coreyja".to_owned()),
            color: Some("#da8a1a".to_owned()),
            head: Some("beach-puffin-special".to_owned()),
            tail: Some("beach-puffin-special".to_owned()),
            version: None,
        }
    }
}

#[cfg(test)]
mod tests {}
