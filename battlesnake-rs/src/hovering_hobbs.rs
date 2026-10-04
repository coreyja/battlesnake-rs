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
pub const STANDARD_SCORES: Scores = Scores {
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

/// Below this health the leaf score stops valuing territory and heads for the nearest food, when
/// only one rival is left alive.
///
/// Growth is worth much less in a duel: there is one snake to out-length rather than three, while
/// the cost of being long -- less room, so more collisions -- is unchanged. Measured head-to-head
/// on 2 snakes under production rules, raising this to 85 **loses** 43.0% (z=-3.80), with
/// head-to-head deaths falling 49 -> 19 but body collisions rising 57 -> 124 and self-collisions
/// 212 -> 279. So the duel threshold stays where it was.
pub const STANDARD_LOW_HEALTH_DUEL: i64 = 60;

/// Below this health the leaf score heads for the nearest food, when two or more rivals are alive.
///
/// 85 rather than 60: measured head-to-head on the 4-snake board under production rules at fixed
/// depth, the conditional policy beats a flat 60 **72.0%** of the time (z=+10.52, 574 decisive games
/// of 720, A/A control exactly 50.0%). In those games the flat-60 side finishes near length 15 and
/// the raised-threshold side near 20, and the elimination counts show the trade: head-to-head deaths
/// 213 -> 84, body collisions flat at 306 -> 300, self-collisions 604 -> 513.
///
/// It is also the knob that matters: adding a doubled food weight on top of it is worth only 53.0%
/// (z=+1.62, n.s.), while adding this threshold on top of a doubled food weight is worth 62.4%
/// (z=+6.30).
///
/// Do not raise it further. The fraction of turns on which *every* leaf sits below the threshold --
/// so territory is demoted to a tiebreak behind food distance for the whole search window -- goes as
/// `depth / (100 - threshold)`, so the cost is hyperbolic: 90 scores 34.0%, 95 scores 27.4%, and 100
/// (always food-seeking) loses every decisive game while finishing *shorter* than a flat 60, because
/// it dives for food into losing space.
///
/// This is a *relative* result from controlled self-play, and deliberately not justified by the
/// leaderboard. The ranked games that preceded it were played while the handler was panicking on
/// last-snake-standing boards (fixed separately), so they say nothing about how this policy plays.
pub const STANDARD_LOW_HEALTH_CROWDED: i64 = 85;

/// Weight on the length term, in thousandths of the territory ratio's own scale.
///
/// The territory ratio `my_space / total_space` lives in `[0, 1]`, so a weight of 20 makes each
/// square of length advantage over the longest living rival worth 0.02 of ratio -- about the
/// difference between claiming 40% and 42% of the board.
///
/// 0 disables the term, which is what ships until an A/B says otherwise.
pub const STANDARD_LENGTH_WEIGHT_MILLI: i64 = 0;

/// Magnitude at which the length difference stops counting, in squares.
///
/// Being 10 longer than every rival is not materially safer than being 3 longer, and an uncapped
/// term would keep paying a runaway leader to eat into bad space.
pub const STANDARD_LENGTH_CAP: i64 = 3;

/// Weight on the continuous hunger term, in thousandths of a ratio point per square of distance to
/// the nearest food, at full hunger.
///
/// This is the *other* way to remove the mode switch's cliff, and the one the switch is actually
/// shaped like. The switch replaces the score with `-dist_to_food` once health drops below a
/// threshold, which is why territory vanishes from the comparison; a hunger term keeps territory and
/// food distance in the **same scalar**, so a hungry snake trades room for food at a rate rather
/// than abandoning room entirely.
///
/// It also differs from the length term in the thing that matters at shallow depth: food distance is
/// an A* distance that sees past the search horizon, while length only changes if the snake can
/// actually reach food inside the window.
///
/// 0 disables the term, which is what ships until an A/B says otherwise.
pub const STANDARD_HUNGER_WEIGHT_MILLI: i64 = 0;

/// Health at which the hunger term starts to bite. Hunger ramps linearly from 0 here to 1 at health
/// 0, so there is no threshold to cross.
pub const STANDARD_HUNGER_ONSET: i64 = 100;

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
    /// Health below which the score switches to food-seeking with one rival left.
    /// See [`STANDARD_LOW_HEALTH_DUEL`].
    pub low_health_duel: i64,
    /// Health below which the score switches to food-seeking with two or more rivals left.
    /// See [`STANDARD_LOW_HEALTH_CROWDED`].
    pub low_health_crowded: i64,
    /// Weight on the length term, in thousandths of a ratio point.
    /// See [`STANDARD_LENGTH_WEIGHT_MILLI`].
    pub length_weight_milli: i64,
    /// Length difference at which the length term saturates. See [`STANDARD_LENGTH_CAP`].
    pub length_cap: i64,
    /// Weight on the continuous hunger term, in thousandths of a ratio point per square of distance
    /// at full hunger. See [`STANDARD_HUNGER_WEIGHT_MILLI`].
    pub hunger_weight_milli: i64,
    /// Health at which hunger starts to bite. See [`STANDARD_HUNGER_ONSET`].
    pub hunger_onset: i64,
    /// Weight on a continuous *health* penalty, in thousandths of a ratio point at zero health.
    ///
    /// The mode switch's real mechanism is ordinal: every `LowOnHealth` score sorts below every
    /// `FloodFill`, so the search treats any line that drops below the threshold as worse than any
    /// line that does not, and the way to stay above it is to eat. That is a step function on
    /// health, not a pull toward food. This is the same pressure made continuous: a penalty that
    /// ramps linearly as health falls, with no band to cross.
    ///
    /// 0 disables it.
    pub health_weight_milli: i64,
    /// Health at which the continuous health penalty starts. Shares the shape of
    /// [`STANDARD_HUNGER_ONSET`].
    pub health_onset: i64,
    /// RESEARCH ONLY -- do not ship. Route food distance through the pre-fix A*, so an A/B can play
    /// the broken pathfinder against the fixed one in one process.
    pub pre_fix_food_distance: bool,
    /// RESEARCH ONLY. Weight of a health ramp that *replaces* the mode switch, in thousandths of a
    /// ratio point at full depth.
    ///
    /// The ramp sits below the same threshold the switch would use and falls linearly over
    /// [`ScoreParams::ramp_width`] health points, then stays flat. Width 1 with a weight of at least
    /// 1000 orders every leaf exactly as the switch does with no food distance inside the band;
    /// wider ramps interpolate toward a smooth health penalty, and smaller weights toward no policy.
    /// That separates the two things the switch does at once: being a *step* and being *dominant*.
    ///
    /// 0 disables it, and the mode switch applies as shipped.
    pub ramp_weight_milli: i64,
    /// Width of the health ramp in health points. 0 means the full threshold, so the ramp runs all
    /// the way from the threshold down to zero health.
    pub ramp_width: i64,
}

impl ScoreParams {
    /// What Hobbs ships.
    pub const STANDARD: Self = Self {
        cycles: STANDARD_CYCLES,
        scores: STANDARD_SCORES,
        low_health_duel: STANDARD_LOW_HEALTH_DUEL,
        low_health_crowded: STANDARD_LOW_HEALTH_CROWDED,
        length_weight_milli: STANDARD_LENGTH_WEIGHT_MILLI,
        length_cap: STANDARD_LENGTH_CAP,
        hunger_weight_milli: STANDARD_HUNGER_WEIGHT_MILLI,
        hunger_onset: STANDARD_HUNGER_ONSET,
        health_weight_milli: 0,
        health_onset: 100,
        pre_fix_food_distance: false,
        ramp_weight_milli: 0,
        ramp_width: 1,
    };

    /// The threshold for this board: growth is worth more the more rivals there are to out-length.
    fn low_health_for(self, rivals: &LivingRivals) -> i64 {
        if rivals.count >= 2 {
            self.low_health_crowded
        } else {
            self.low_health_duel
        }
    }

    /// The length term for this board: how far ahead of the longest *living* rival we are, capped,
    /// scaled into the territory ratio's units.
    ///
    /// This is the continuous alternative to the health mode switch. The switch demotes territory to
    /// a tiebreak behind food distance for the whole search window once every leaf is below the
    /// threshold, which is why it has a cliff; a term inside `FloodFill` prices length against
    /// territory at every leaf instead, so there is nothing to cross.
    ///
    /// Returns zero when disabled or when nobody else is alive, so the default params produce
    /// exactly the score they produced before this term existed.
    fn length_term(self, my_length: i64, rivals: &LivingRivals) -> N64 {
        if self.length_weight_milli == 0 {
            return N64::from(0.0);
        }

        let Some(longest_rival) = rivals.longest else {
            return N64::from(0.0);
        };

        let diff = (my_length - longest_rival).clamp(-self.length_cap, self.length_cap);
        N64::from(diff as f64 * self.length_weight_milli as f64 / 1000.0)
    }

    /// The hunger penalty for this board: distance to the nearest food, weighted by how hungry we
    /// are, subtracted from the territory ratio.
    ///
    /// Hunger ramps linearly from 0 at [`ScoreParams::hunger_onset`] health to 1 at 0 health, so a
    /// snake on full health ignores food entirely and a starving one will trade a lot of room for a
    /// square of progress toward it -- with no threshold at which territory stops counting.
    ///
    /// An unreachable food contributes nothing. The flood fill already scores being sealed into a
    /// pocket, and inventing a distance for food we cannot reach would price a move by a route that
    /// does not exist.
    ///
    /// Returns zero when disabled, so the default params produce exactly the score they produced
    /// before this term existed -- and the A* call is skipped, which is the expensive part.
    /// The continuous health penalty: how far below the onset our health is, as a fraction of the
    /// onset, scaled into ratio units. Zero at or above the onset, so a healthy snake is unaffected.
    fn health_penalty(self, health: i64) -> N64 {
        if self.health_weight_milli == 0 {
            return N64::from(0.0);
        }
        let shortfall =
            (self.health_onset - health).max(0) as f64 / self.health_onset.max(1) as f64;
        N64::from(shortfall * self.health_weight_milli as f64 / 1000.0)
    }

    /// The health ramp below `threshold`: how far into the ramp our health is, as a fraction of its
    /// width, scaled into ratio units. Zero at or above the threshold.
    fn health_ramp(self, health: i64, threshold: i64) -> N64 {
        if self.ramp_weight_milli == 0 {
            return N64::from(0.0);
        }
        let width = if self.ramp_width == 0 {
            threshold
        } else {
            self.ramp_width
        }
        .max(1);
        let depth = (threshold - health).clamp(0, width) as f64 / width as f64;
        N64::from(depth * self.ramp_weight_milli as f64 / 1000.0)
    }

    /// Distance to the nearest food from our own head, through whichever A* the params select.
    fn food_distance<BoardType>(self, node: &BoardType) -> Option<i32>
    where
        BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
            + YouDeterminableGame
            + APrimeCalculable
            + HeadGettableGame
            + FoodGettableGame,
    {
        let me = node.you_id();
        let head = node.get_head_as_native_position(me);
        let food = node.get_all_food_as_native_positions();
        if self.pre_fix_food_distance {
            node.shortest_distance_pre_fix(&head, &food)
        } else {
            node.shortest_distance(&head, &food, None)
        }
    }

    fn hunger_penalty<BoardType>(self, node: &BoardType) -> N64
    where
        BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
            + YouDeterminableGame
            + APrimeCalculable
            + HeadGettableGame
            + HealthGettableGame
            + FoodGettableGame,
    {
        if self.hunger_weight_milli == 0 {
            return N64::from(0.0);
        }

        let me = node.you_id();
        let hunger = (self.hunger_onset - node.get_health_i64(me)).max(0) as f64
            / self.hunger_onset.max(1) as f64;
        if hunger <= 0.0 {
            return N64::from(0.0);
        }

        let Some(dist) = self.food_distance(node) else {
            return N64::from(0.0);
        };

        N64::from(hunger * dist as f64 * self.hunger_weight_milli as f64 / 1000.0)
    }
}

/// What the leaf score needs to know about the other snakes.
///
/// Both the health threshold and the length term are functions of the living rivals, and
/// `get_snake_ids` allocates a `Vec` on every call. The leaf score runs millions of times per move,
/// so the list is walked exactly once and both answers come out of the same pass.
struct LivingRivals {
    /// How many rivals are still alive. Picks which health threshold applies.
    count: usize,
    /// The longest living rival, or `None` when we are the last snake standing.
    longest: Option<i64>,
}

impl LivingRivals {
    fn of<BoardType>(node: &BoardType) -> Self
    where
        BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
            + YouDeterminableGame
            + HealthGettableGame
            + LengthGettableGame,
    {
        let me = node.you_id();
        let mut count = 0;
        let mut longest: Option<i64> = None;
        for id in node.get_snake_ids().iter().filter(|id| *id != me) {
            if !node.is_alive(id) {
                continue;
            }
            count += 1;
            let length = node.get_length_i64(id);
            longest = Some(longest.map_or(length, |best: i64| best.max(length)));
        }
        Self { count, longest }
    }
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
    params: ScoreParams,
) -> Score
where
    BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + YouDeterminableGame
        + APrimeCalculable
        + HeadGettableGame
        + HealthGettableGame
        + LengthGettableGame
        + FoodGettableGame,
{
    let me = node.you_id();
    let my_space: f64 = square_counts[me.as_usize()] as f64;
    let total_space: f64 = square_counts.iter().sum::<u16>() as f64;
    let my_ratio = N64::from(my_space / total_space);

    let rivals = LivingRivals::of(node);
    let health = node.get_health_i64(me);
    let threshold = params.low_health_for(&rivals);

    if params.ramp_weight_milli == 0 && health < threshold {
        let dist = params.food_distance(node).map(|x| -x);
        return Score::LowOnHealth(dist, my_ratio);
    }

    Score::FloodFill(
        my_ratio + params.length_term(node.get_length_i64(me), &rivals)
            - params.hunger_penalty(node)
            - params.health_penalty(health)
            - params.health_ramp(health, threshold),
    )
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
        params,
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
        params,
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

    use crate::a_prime::APrimeCalculable;
    use crate::hovering_hobbs::{
        standard_score, standard_score_tail_aware, standard_score_tail_aware_with_params,
        standard_score_with_params, Score, ScoreParams, STANDARD_LOW_HEALTH_DUEL,
    };
    use battlesnake_game_types::compact_representation::StandardCellBoard4Snakes11x11;
    use battlesnake_game_types::types::{FoodGettableGame, HeadGettableGame, LengthGettableGame};
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

    /// The length term has to move the score by exactly its weight times the capped length
    /// difference, and by nothing at all at weight 0 -- which is what ships, and what the test above
    /// relies on.
    #[test]
    fn the_length_term_is_the_weight_times_the_capped_difference() {
        // The diffs these fixtures carry are 0, +1 and -9, so the sign and the cap are both
        // exercised rather than assumed.
        let mut diffs_seen = Vec::new();
        for fixture in [
            include_str!("../fixtures/start_of_game.json"),
            include_str!("../fixtures/check_board_doubled_up.json"),
            include_str!("../fixtures/a-prime-food-maze.json"),
        ] {
            let game = serde_json::from_str::<Game>(fixture).unwrap();
            let id_map = build_snake_id_map(&game);
            let board = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let Score::FloodFill(base) =
                standard_score_tail_aware_with_params::<_, u8, 4>(&board, ScoreParams::STANDARD)
            else {
                panic!("fixture is above every shipped health threshold, so it should flood-fill");
            };

            let me = board.you_id();
            let longest_rival = board
                .get_snake_ids()
                .iter()
                .filter(|id| *id != me)
                .map(|id| board.get_length_i64(id))
                .max()
                .expect("fixture has rivals");
            let raw_diff = board.get_length_i64(me) - longest_rival;
            diffs_seen.push(raw_diff);

            for (weight, cap) in [(20, 3), (20, 1), (5, 3), (100, 3)] {
                let params = ScoreParams {
                    length_weight_milli: weight,
                    length_cap: cap,
                    ..ScoreParams::STANDARD
                };
                let Score::FloodFill(scored) =
                    standard_score_tail_aware_with_params::<_, u8, 4>(&board, params)
                else {
                    panic!("the length term must not change which branch the score takes");
                };

                let expected = raw_diff.clamp(-cap, cap) as f64 * weight as f64 / 1000.0;
                let actual = f64::from(scored) - f64::from(base);
                // Subtracting two ratios reintroduces float error the term itself does not have,
                // so compare within it rather than bit-exactly.
                assert!(
                    (actual - expected).abs() < 1e-12,
                    "weight {weight} cap {cap} on a board where we are {raw_diff} longer: \
                     moved the score by {actual}, expected {expected}"
                );
            }
        }

        // Without this the cap and the negative branch could go untested if a fixture changed.
        assert!(
            diffs_seen.iter().any(|d| *d < -3),
            "no fixture saturates the cap on the negative side: {diffs_seen:?}"
        );
        assert!(
            diffs_seen.contains(&0),
            "no fixture has equal lengths, so the zero case is untested: {diffs_seen:?}"
        );
    }

    /// The hunger term has to be hunger x distance x weight, and has to leave the score in the
    /// `FloodFill` branch -- the whole point is that territory never stops counting.
    ///
    /// Every fixture is on 100 health, so rather than inventing a board this raises `hunger_onset`
    /// above 100 to put a full-health snake partway up the ramp. That is the same trick
    /// `the_low_health_threshold_picks_the_branch` uses from the other side.
    #[test]
    fn the_hunger_term_is_hunger_times_distance_times_weight() {
        for fixture in [
            include_str!("../fixtures/start_of_game.json"),
            include_str!("../fixtures/a-prime-food-maze.json"),
        ] {
            let game = serde_json::from_str::<Game>(fixture).unwrap();
            let id_map = build_snake_id_map(&game);
            let board = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

            let Score::FloodFill(base) =
                standard_score_tail_aware_with_params::<_, u8, 4>(&board, ScoreParams::STANDARD)
            else {
                panic!("fixtures are on 100 health, so the shipped params should flood-fill");
            };

            let me = board.you_id();
            let dist = board
                .shortest_distance(
                    &board.get_head_as_native_position(me),
                    &board.get_all_food_as_native_positions(),
                    None,
                )
                .unwrap_or_else(|| {
                    panic!(
                        "no reachable food: head {:?} food {:?}",
                        board.get_head_as_native_position(me),
                        board.get_all_food_as_native_positions()
                    )
                });

            for (weight, onset) in [(5, 200), (20, 200), (20, 400), (100, 125)] {
                let params = ScoreParams {
                    hunger_weight_milli: weight,
                    hunger_onset: onset,
                    ..ScoreParams::STANDARD
                };
                let Score::FloodFill(scored) =
                    standard_score_tail_aware_with_params::<_, u8, 4>(&board, params)
                else {
                    panic!("the hunger term must not change which branch the score takes");
                };

                let hunger = (onset - 100) as f64 / onset as f64;
                let expected = -hunger * dist as f64 * weight as f64 / 1000.0;
                let actual = f64::from(scored) - f64::from(base);
                assert!(
                    (actual - expected).abs() < 1e-12,
                    "weight {weight} onset {onset}, food {dist} away: moved the score by {actual}, \
                     expected {expected}"
                );
                assert!(
                    expected < 0.0,
                    "a hungry snake has to be penalised for distance, not rewarded"
                );
            }

            // At or above the onset there is no hunger, so the term is exactly nothing -- the
            // property that keeps a full-health snake's play identical.
            let not_hungry = ScoreParams {
                hunger_weight_milli: 100,
                hunger_onset: 100,
                ..ScoreParams::STANDARD
            };
            assert_eq!(
                standard_score_tail_aware_with_params::<_, u8, 4>(&board, not_hungry),
                Score::FloodFill(base)
            );
        }
    }

    /// The thresholds are the whole growth policy, so a changed one has to actually change the
    /// branch the score takes -- not just sit in the struct unread.
    #[test]
    fn the_low_health_threshold_picks_the_branch() {
        let game =
            serde_json::from_str::<Game>(include_str!("../fixtures/start_of_game.json")).unwrap();
        let id_map = build_snake_id_map(&game);
        let board = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        // A freshly spawned snake is on 100 health, so every shipped threshold hovers...
        assert!(matches!(
            standard_score_tail_aware_with_params::<_, u8, 4>(&board, ScoreParams::STANDARD),
            Score::FloodFill(_)
        ));

        // ...and a threshold above full health always seeks food.
        let always_eat = ScoreParams {
            low_health_duel: 101,
            low_health_crowded: 101,
            ..ScoreParams::STANDARD
        };
        assert!(matches!(
            standard_score_tail_aware_with_params::<_, u8, 4>(&board, always_eat),
            Score::LowOnHealth(_, _)
        ));
    }

    /// The ramp replaces the mode switch: a board below the threshold keeps its territory score and
    /// pays the ramp instead of dropping into `LowOnHealth`, and width 1 is a step of the full weight.
    #[test]
    fn the_health_ramp_replaces_the_switch_and_falls_over_its_width() {
        let game =
            serde_json::from_str::<Game>(include_str!("../fixtures/start_of_game.json")).unwrap();
        let id_map = build_snake_id_map(&game);
        let board = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();
        let Score::FloodFill(base) =
            standard_score_tail_aware_with_params::<_, u8, 4>(&board, ScoreParams::STANDARD)
        else {
            panic!("a full-health snake hovers");
        };

        // Every snake is on 100 health. (threshold, width, expected depth into the ramp)
        for (threshold, width, depth) in [
            (101, 1, 1.0),
            (110, 1, 1.0),
            (110, 20, 0.5),
            (110, 0, 10.0 / 110.0),
            (100, 5, 0.0),
        ] {
            let params = ScoreParams {
                low_health_crowded: threshold,
                ramp_weight_milli: 1000,
                ramp_width: width,
                ..ScoreParams::STANDARD
            };
            let Score::FloodFill(scored) =
                standard_score_tail_aware_with_params::<_, u8, 4>(&board, params)
            else {
                panic!("with the ramp on the score never takes the LowOnHealth branch");
            };
            let actual = f64::from(base) - f64::from(scored);
            assert!(
                (actual - depth).abs() < 1e-12,
                "threshold {threshold} width {width}: penalty {actual}, expected {depth}"
            );
        }
    }

    /// The point of two thresholds is that the board picks between them. `start_of_game` has three
    /// snakes, so it must read the *crowded* one and ignore the duel one entirely.
    #[test]
    fn the_rival_count_picks_which_threshold_applies() {
        let game =
            serde_json::from_str::<Game>(include_str!("../fixtures/start_of_game.json")).unwrap();
        let id_map = build_snake_id_map(&game);
        let board = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();
        assert_eq!(board.get_snake_ids().len(), 3, "fixture is a crowded board");

        // Crowded threshold above full health: food-seeking, even though the duel threshold is low.
        let crowded_eats = ScoreParams {
            low_health_duel: 1,
            low_health_crowded: 101,
            ..ScoreParams::STANDARD
        };
        assert!(matches!(
            standard_score_tail_aware_with_params::<_, u8, 4>(&board, crowded_eats),
            Score::LowOnHealth(_, _)
        ));

        // And the other way round: a sky-high *duel* threshold must not reach a 3-snake board.
        let duel_eats = ScoreParams {
            low_health_duel: 101,
            low_health_crowded: 1,
            ..ScoreParams::STANDARD
        };
        assert!(matches!(
            standard_score_tail_aware_with_params::<_, u8, 4>(&board, duel_eats),
            Score::FloodFill(_)
        ));
    }

    /// The claim that justifies a conditional threshold rather than a flat one: with a single rival
    /// the crowded threshold is unreachable, so raising it cannot change how Hobbs plays a duel.
    #[test]
    fn a_duel_never_reads_the_crowded_threshold() {
        let game =
            serde_json::from_str::<Game>(include_str!("../fixtures/check_board_doubled_up.json"))
                .unwrap();
        let id_map = build_snake_id_map(&game);
        let board = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();
        assert_eq!(board.get_snake_ids().len(), 2, "fixture is a duel");

        // Whatever the crowded threshold says, a duel board scores the same as it did before the
        // crowded threshold existed.
        let flat_duel = ScoreParams {
            low_health_crowded: STANDARD_LOW_HEALTH_DUEL,
            ..ScoreParams::STANDARD
        };
        for crowded in [1, 50, 85, 101] {
            let params = ScoreParams {
                low_health_crowded: crowded,
                ..ScoreParams::STANDARD
            };
            assert_eq!(
                standard_score_tail_aware_with_params::<_, u8, 4>(&board, params),
                standard_score_tail_aware_with_params::<_, u8, 4>(&board, flat_duel),
                "crowded threshold {crowded} leaked into a 2-snake board"
            );
        }
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
