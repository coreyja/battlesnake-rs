use std::time::Duration;

use crate::a_prime::APrimeCalculable;
use crate::flood_fill::spread_from_head::{Scores, SpreadFromHead};
use crate::flood_fill::spread_from_head_arcade_maze::SpreadFromHeadArcadeMaze;
use crate::flood_fill::spread_from_head_tail_aware::SpreadFromHeadTailAware;
use crate::*;

use battlesnake_minimax::{
    paranoid::{move_ordering::MoveOrdering, SnakeOptions},
    ParanoidMinimaxSnake,
};
use decorum::N64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Score {
    LowOnHealth(Option<i32>, N64),
    FloodFill(N64),
}

/// Weights for the flood-fill leaf score: food is worth four empty squares, hazards a fifth of
/// one.
const STANDARD_SCORES: Scores = Scores {
    food: 20,
    hazard: 1,
    empty: 5,
};

/// How many spread cycles the leaf score looks ahead.
///
/// The fill stops early once its frontier dies, so this is a cap rather than a fixed cost: on a
/// cramped board a larger budget costs nothing extra, and on an 11x11 board the frontier is dead by
/// ~12 cycles, so any larger value is the same computation.
///
/// 12 rather than the original 5: measured head-to-head under production rules, a 12-cycle
/// tail-aware fill beats the 5-cycle one 90.0% of the time (z=+22.4) *after* being charged a full
/// deepening round of lost search on every position. See DEV-1507.
pub const STANDARD_CYCLES: usize = 12;

/// Below this health the leaf score stops valuing territory and heads for the nearest food.
///
/// This is Hobbs' whole growth policy: above the threshold food is worth only its fill weight, so
/// Hobbs hovers and controls space rather than growing, and eats exactly often enough to stay
/// alive. Measured in real Arena games, that is about one food every 40 turns.
pub const STANDARD_LOW_HEALTH: i64 = 60;

/// Every knob the flood-fill leaf score has. Hobbs ships [`ScoreParams::STANDARD`].
///
/// This exists so a sweep can vary one knob without a second copy of the scoring logic; see
/// `byte-scratch/hobbs-tail-aware-ab`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreParams {
    /// Cap on spread cycles. See [`STANDARD_CYCLES`].
    pub cycles: usize,
    /// Per-square weights for the fill. See [`STANDARD_SCORES`].
    pub scores: Scores,
    /// Health below which the score switches to food-seeking. See [`STANDARD_LOW_HEALTH`].
    pub low_health: i64,
}

impl ScoreParams {
    /// What Hobbs ships.
    pub const STANDARD: Self = Self {
        cycles: STANDARD_CYCLES,
        scores: STANDARD_SCORES,
        low_health: STANDARD_LOW_HEALTH,
    };
}

impl Default for ScoreParams {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// Turn a per-snake territory count into a [`Score`], shared by every flood-fill variant so the
/// variants differ only in how the territory was measured.
pub fn score_from_square_counts<BoardType, const MAX_SNAKES: usize>(
    node: &BoardType,
    square_counts: [u16; MAX_SNAKES],
    low_health: i64,
) -> Score
where
    BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + YouDeterminableGame
        + APrimeCalculable
        + HeadGettableGame
        + HealthGettableGame
        + FoodGettableGame,
{
    let me = node.you_id();
    let my_space: f64 = square_counts[me.as_usize()] as f64;
    let total_space: f64 = square_counts.iter().sum::<u16>() as f64;
    let my_ratio = N64::from(my_space / total_space);

    if node.get_health_i64(me) < low_health {
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

/// [`standard_score`] with every knob supplied by the caller.
pub fn standard_score_with_params<BoardType, CellType, const MAX_SNAKES: usize>(
    node: &BoardType,
    params: ScoreParams,
) -> Score
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
    score_from_square_counts(
        node,
        node.squares_per_snake_with_scores(params.cycles, params.scores),
        params.low_health,
    )
}

/// [`standard_score_tail_aware`] with every knob supplied by the caller.
pub fn standard_score_tail_aware_with_params<BoardType, CellType, const MAX_SNAKES: usize>(
    node: &BoardType,
    params: ScoreParams,
) -> Score
where
    BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + YouDeterminableGame
        + SpreadFromHeadTailAware<CellType, MAX_SNAKES>
        + APrimeCalculable
        + HeadGettableGame
        + HazardQueryableGame
        + HealthGettableGame
        + LengthGettableGame
        + FoodGettableGame
        + MaxSnakes<MAX_SNAKES>,
{
    score_from_square_counts(
        node,
        node.squares_per_snake_with_scores_tail_aware(params.cycles, params.scores),
        params.low_health,
    )
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
    standard_score_with_params::<_, CellType, MAX_SNAKES>(node, ScoreParams::STANDARD)
}

/// [`standard_score`] with a tail-aware flood fill: squares your own tail is about to vacate count
/// as reachable instead of as wall. See [`crate::flood_fill::spread_from_head_tail_aware`].
pub fn standard_score_tail_aware<BoardType, CellType, const MAX_SNAKES: usize>(
    node: &BoardType,
) -> Score
where
    BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + YouDeterminableGame
        + SpreadFromHeadTailAware<CellType, MAX_SNAKES>
        + APrimeCalculable
        + HeadGettableGame
        + HazardQueryableGame
        + HealthGettableGame
        + LengthGettableGame
        + FoodGettableGame
        + MaxSnakes<MAX_SNAKES>,
{
    standard_score_tail_aware_with_params::<_, CellType, MAX_SNAKES>(node, ScoreParams::STANDARD)
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
            use battlesnake_game_types::compact_representation::wrapped::*;

            build_from_best_cell_board_inner!(game, game_info, turn, $score_function, name, options)
        } else {
            use battlesnake_game_types::compact_representation::standard::*;

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
            build_from_best_cell_board!(
                game,
                game_info,
                turn,
                standard_score_tail_aware,
                name,
                options
            )
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
    use battlesnake_game_types::{
        compact_representation::WrappedCellBoard4Snakes11x11,
        types::{build_snake_id_map, SnakeIDGettableGame, YouDeterminableGame},
        wire_representation::Game,
    };

    use crate::hovering_hobbs::{
        standard_score, standard_score_tail_aware, standard_score_tail_aware_with_params,
        standard_score_with_params, Score, ScoreParams,
    };
    use battlesnake_game_types::compact_representation::StandardCellBoard4Snakes11x11;
    use battlesnake_minimax::ParanoidMinimaxSnake;

    /// The parameterized entry points exist so a sweep can vary one knob; at
    /// [`ScoreParams::STANDARD`] they have to be the shipped score exactly, or every baseline in a
    /// sweep is quietly measuring something else.
    #[test]
    fn the_standard_params_reproduce_the_shipped_scores() {
        for fixture in [
            include_str!("../fixtures/start_of_game.json"),
            include_str!("../fixtures/a-prime-food-maze.json"),
            include_str!("../fixtures/check_board_doubled_up.json"),
        ] {
            let game = serde_json::from_str::<Game>(fixture).unwrap();
            let id_map = build_snake_id_map(&game);
            let board = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            assert_eq!(
                standard_score_with_params::<_, u8, 4>(&board, ScoreParams::STANDARD),
                standard_score::<_, u8, 4>(&board)
            );
            assert_eq!(
                standard_score_tail_aware_with_params::<_, u8, 4>(&board, ScoreParams::STANDARD),
                standard_score_tail_aware::<_, u8, 4>(&board)
            );
        }
    }

    /// `low_health` is the whole growth policy, so a changed threshold has to actually change the
    /// branch the score takes -- not just sit in the struct unread.
    #[test]
    fn the_low_health_threshold_picks_the_branch() {
        let game =
            serde_json::from_str::<Game>(include_str!("../fixtures/start_of_game.json")).unwrap();
        let id_map = build_snake_id_map(&game);
        let board = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        // A freshly spawned snake is on 100 health, so the shipped threshold of 60 hovers...
        assert!(matches!(
            standard_score_tail_aware_with_params::<_, u8, 4>(&board, ScoreParams::STANDARD),
            Score::FloodFill(_)
        ));

        // ...and a threshold above full health always seeks food.
        let always_eat = ScoreParams {
            low_health: 101,
            ..ScoreParams::STANDARD
        };
        assert!(matches!(
            standard_score_tail_aware_with_params::<_, u8, 4>(&board, always_eat),
            Score::LowOnHealth(_, _)
        ));
    }

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
        let hobbs = ParanoidMinimaxSnake::new(
            game,
            game_info,
            turn,
            &standard_score_tail_aware,
            name,
            options,
        );

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
