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
    let scores = Scores {
        food: 20,
        hazard: 1,
        empty: 5,
    };
    let square_counts = node.squares_per_snake_with_scores(5, scores);

    let me = node.you_id();
    let my_space: f64 = square_counts[me.as_usize()] as f64;
    let total_space: f64 = square_counts.iter().sum::<u16>() as f64;
    let my_ratio = N64::from(my_space / total_space);

    if node.get_health_i64(me) < 60 {
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
    let length_diff_multiplier: f64 = 0.05 * capped_diff as f64;

    Score::FloodFill(my_ratio * length_diff_multiplier)
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

impl Factory {
    pub fn create_from_wire_game(&self, game: Game) -> BoxedSnake {
        let game_info = game.game.clone();
        let turn = game.turn;

        let name = "hovering-hobbs";

        let options: SnakeOptions = SnakeOptions {
            network_latency_padding: Duration::from_millis(120),
            move_ordering: MoveOrdering::BestFirst,
        };

        if game.is_arcade_maze_map() {
            build_from_best_cell_board!(game, game_info, turn, arcade_maze_score, name, options)
        } else {
            build_from_best_cell_board!(game, game_info, turn, standard_score, name, options)
        }
    }

    pub fn about(&self) -> AboutMe {
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
mod tests {
    use types::{
        compact_representation::WrappedCellBoard4Snakes11x11,
        types::{build_snake_id_map, SnakeIDGettableGame, YouDeterminableGame},
        wire_representation::Game,
    };

    use crate::hovering_hobbs::standard_score;
    use battlesnake_minimax::ParanoidMinimaxSnake;

    #[test]
    #[ignore]
    fn test_095b30fa_f2c7_4826_ac93_90b4dde6b785_turn_5() {
        let fixture = include_str!("../../fixtures/095b30fa-f2c7-4826-ac93-90b4dde6b785_5.json");
        let next_move = move_for_wrapped_fixture(fixture);

        // Right allows us to tailchase,
        // but left gets us into a spot where the 'best' minimax
        // outcome is a tie.
        assert_eq!(next_move, "right");
    }

    #[test]
    fn test_095b30fa_f2c7_4826_ac93_90b4dde6b785_turn_6() {
        let fixture = include_str!("../../fixtures/095b30fa-f2c7-4826-ac93-90b4dde6b785_6.json");
        let next_move = move_for_wrapped_fixture(fixture);

        // Down looks like a tie at best,
        // but left is a lose for sure so down is a tad better
        // Theory this didn't end as a tie for me, cause I think
        // about the score in terms of end states
        assert_eq!(next_move, "down");
    }

    #[test]
    #[ignore]
    fn test_6d9cd0b1_6829_4430_926c_562918397774_turn_101() {
        let fixture = include_str!("../../fixtures/6d9cd0b1-6829-4430-926c-562918397774_101.json");

        let next_move = move_for_wrapped_fixture(fixture);

        let allowed_moves = vec!["left", "right"];

        // Down leads into certain death
        // But left or right allow a tial chase possibility
        // The scores here indicate that down is just getting a better flood score
        // But thats weird cause visually it looks like I should control a LOT less of the board
        // TODO: Figure out what board state we are scoring with `Down` here. And see if there is
        // a scoring bug that is giving it a higher score than we want
        assert!(
            allowed_moves.contains(&next_move.as_str()),
            "{next_move} not in {allowed_moves:?}"
        );
    }

    fn move_for_wrapped_fixture(fixture: &str) -> String {
        let game = serde_json::from_str::<Game>(fixture).unwrap();
        let id_map = build_snake_id_map(&game);

        // game.game.timeout = 5000;

        let game_info = game.game.clone();
        let turn = game.turn;
        let name = "hovering-hobbs";
        let options = Default::default();
        let game = WrappedCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();
        let hobbs =
            ParanoidMinimaxSnake::new(game, game_info, turn, &standard_score, name, options);

        let my_id = game.you_id();
        let mut sorted_ids = game.get_snake_ids();
        sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

        let (depth, scored) = hobbs.deepened_minimax_until_timelimit(sorted_ids, None);
        let scored_options = scored.first_options_for_snake(my_id).unwrap();
        let scores = scored_options
            .iter()
            .map(|(m, r)| (m, r.score()))
            .collect::<Vec<_>>();

        dbg!(depth, &scores);

        scored_options.first().unwrap().0.to_string()
    }
}
