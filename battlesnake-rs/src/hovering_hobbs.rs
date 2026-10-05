use crate::a_prime::APrimeCalculable;
use crate::flood_fill::spread_from_head::{Scores, SpreadFromHead};
use crate::flood_fill::spread_from_head_tail_aware::SpreadFromHeadTailAware;
use crate::*;

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
/// 95, up from 85 once the score counted a meal anywhere in the search window (see below). Measured
/// on four snakes under production rules, both sides searching to the same budget of leaf
/// evaluations, against 85 with the length term on:
///
/// | budget | win rate vs 85 | z |
/// | -- | --: | --: |
/// | 8k leaves | 64.3% (101/157) | +3.59 |
/// | 60k leaves (~production) | 52.4% (77/147) | +0.58 |
/// | both | 58.6% (178/304) | +2.98 |
///
/// The gain shrinks with budget, and at production depth alone it is not significant. What does
/// show at 60k is growth: food one step away is taken 85.8% of the time against 79.0%, and the
/// final length is 22.0 against 20.2. 100 ties with 95: 61.6% and 54.6% against 85, and 52.7%
/// head-to-head against 95 at 8k (n.s.).
///
/// The rival count matters because growth is worth more the more snakes there are to out-length.
/// At fixed depth, 85 crowded / 60 in a duel beat a flat 60 **72.0%** of the time (z=+10.52, 574
/// decisive games of 720, A/A control exactly 50.0%), mostly by cutting head-to-head deaths from 213
/// to 84. A doubled food weight on top of it is worth only 53.0% (n.s.), so the threshold is the
/// knob that matters.
///
/// A high threshold used to make Hobbs skip food, because of how food mode scored a meal. Food mode
/// scores `-dist_to_food`, and eating destroys the food you were next to, so a leaf that ate early
/// in the window -- and is back under the threshold by the leaf -- scored *worse* than one still
/// hovering beside the food. A meal on round `k` of a `D`-round window only counted if
/// `D - k <= 100 - threshold`, so once the search reached `102 - threshold` rounds it kept
/// scheduling the meal for the horizon and never took it. At 100 that starts at two rounds: an
/// always-hungry Hobbs starved beside food 59 times in 180 games, which is why 100 used to lose every
/// decisive game. [`ScoreParams::root_length`] makes every meal in the window count at any depth,
/// and the live score always supplies it.
///
/// This is a *relative* result from controlled self-play, and deliberately not justified by the
/// leaderboard. The ranked games that preceded it were played while the handler was panicking on
/// last-snake-standing boards (fixed separately), so they say nothing about how this policy plays.
pub const STANDARD_LOW_HEALTH_CROWDED: i64 = 95;

/// Weight on the length term, in thousandths of the territory ratio's own scale.
///
/// The territory ratio `my_space / total_space` lives in `[0, 1]`, so a weight of 20 makes each
/// square of length advantage over the longest living rival worth 0.02 of ratio -- about the
/// difference between claiming 40% and 42% of the board.
///
/// Ships at 160: one square of length is worth 0.16 of ratio. Sibling moves usually differ by about
/// 0.05, so in practice this orders leaves by capped length difference first and territory second,
/// and only a territory gap above 0.16 per square (being sealed in, or sealing a rival) overrules
/// it.
///
/// Measured in self-play under production rules against the same snake with the term off, both
/// sides searching to the same budget of leaf evaluations -- iterative deepening that plays the
/// deepest completed round, which is the production time limit with leaves in place of wall time,
/// so the term is charged for any depth it costs:
///
/// | board | budget | games | win rate | z |
/// | -- | -- | --: | --: | --: |
/// | 4 snakes | 1k leaves | 180 | 73.6% | +5.75 |
/// | 4 snakes | 8k leaves | 180 | 78.9% | +6.88 |
/// | 4 snakes | 60k leaves (~production) | 120 | 66.7% | +3.11 |
/// | duel | 1k leaves | 600 | 63.8% | +6.20 |
/// | duel | 8k leaves | 60 | 62.7% | +1.82 |
///
/// It costs no search: compared at the same number of living snakes, both sides complete the same
/// number of rounds on the same budget (2.26 vs 2.25 with four alive at 8k). It does not replace
/// [`STANDARD_LOW_HEALTH_CROWDED`] either -- keeping that threshold on top of the term beats a flat
/// 60 with the term, 58.0% (z=+1.96).
///
/// 0 disables the term.
pub const STANDARD_LENGTH_WEIGHT_MILLI: i64 = 160;

/// Magnitude at which the length difference stops counting, in squares.
///
/// Being 10 longer than every rival is not materially safer than being 3 longer, and an uncapped
/// term would keep paying a runaway leader to eat into bad space. A cap of 1 throws most of the
/// value away: at weight 160, cap 3 beats cap 1 71.5% (z=+5.41; four snakes, fixed depth, both on a
/// flat 60 threshold), so being two or three longer than the longest rival is what pays, not merely
/// not falling behind.
pub const STANDARD_LENGTH_CAP: i64 = 3;

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
    /// Our length at the root of the search, when the caller knows it.
    ///
    /// A leaf longer than this has eaten inside the search window, and keeps the territory score
    /// whatever its health. Without it, the health threshold is read from the leaf's health alone,
    /// so a meal only counts if it was recent enough to lift the leaf back over the threshold. See
    /// [`STANDARD_LOW_HEALTH_CROWDED`] for why that makes a high threshold skip food.
    ///
    /// `None` in [`ScoreParams::STANDARD`] because it belongs to one search, not to the snake; the
    /// live route fills it in on every move.
    pub root_length: Option<i64>,
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
        root_length: None,
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
    let my_length = node.get_length_i64(me);

    // Length only grows by eating, so a leaf longer than the root ate somewhere in the window.
    let ate_in_window = params.root_length.is_some_and(|root| my_length > root);

    if !ate_in_window && node.get_health_i64(me) < params.low_health_for(&rivals) {
        let dist = node
            .shortest_distance(
                &node.get_head_as_native_position(me),
                &node.get_all_food_as_native_positions(),
                None,
            )
            .map(|x| -x);
        return Score::LowOnHealth(dist, my_ratio);
    }

    Score::FloodFill(my_ratio + params.length_term(my_length, &rivals))
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

/// The leaf score Hobbs plays: [`standard_score`] with a tail-aware flood fill, where squares your
/// own tail is about to vacate count as reachable instead of as wall (see
/// [`crate::flood_fill::spread_from_head_tail_aware`]).
///
/// `root_length` is our length at the root of this search. It is required because the shipped
/// crowded threshold is only safe with it: without it a meal early in the window scores as hungry,
/// and the search keeps putting the meal off to the horizon. See [`ScoreParams::root_length`].
pub fn standard_score_tail_aware<BoardType, CellType, const MAX_SNAKES: usize>(
    node: &BoardType,
    root_length: i64,
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
    standard_score_tail_aware_with_params::<_, CellType, MAX_SNAKES>(
        node,
        ScoreParams {
            root_length: Some(root_length),
            ..ScoreParams::STANDARD
        },
    )
}

pub struct Factory;

impl Factory {
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
        standard_score_with_params, Score, ScoreParams, STANDARD_LOW_HEALTH_DUEL,
    };
    use battlesnake_game_types::compact_representation::StandardCellBoard4Snakes11x11;
    use battlesnake_game_types::types::{LengthGettableGame, Move};
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
            let root_length = board.get_length_i64(board.you_id());
            let at_root = ScoreParams {
                root_length: Some(root_length),
                ..ScoreParams::STANDARD
            };
            assert_eq!(
                standard_score_tail_aware_with_params::<_, u8, 4>(&board, at_root),
                standard_score_tail_aware::<_, u8, 4>(&board, root_length)
            );
        }
    }

    /// The length term has to move the score by exactly its weight times the capped length
    /// difference, measured from the same score with the term switched off.
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

            let without_term = ScoreParams {
                length_weight_milli: 0,
                ..ScoreParams::STANDARD
            };
            let Score::FloodFill(base) =
                standard_score_tail_aware_with_params::<_, u8, 4>(&board, without_term)
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

    /// The length term ships on, so the entry point the live route calls has to carry it -- not
    /// just the parameterized one a sweep calls.
    #[test]
    fn the_shipped_score_carries_the_length_term() {
        // We are 9 shorter than the longest rival here, so the term is at its negative cap.
        let game = serde_json::from_str::<Game>(include_str!("../fixtures/a-prime-food-maze.json"))
            .unwrap();
        let id_map = build_snake_id_map(&game);
        let board = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();

        let root_length = board.get_length_i64(board.you_id());
        let without_term = ScoreParams {
            length_weight_milli: 0,
            root_length: Some(root_length),
            ..ScoreParams::STANDARD
        };
        let (Score::FloodFill(shipped), Score::FloodFill(off)) = (
            standard_score_tail_aware::<_, u8, 4>(&board, root_length),
            standard_score_tail_aware_with_params::<_, u8, 4>(&board, without_term),
        ) else {
            panic!("fixture is above every shipped health threshold, so it should flood-fill");
        };

        let expected = -(ScoreParams::STANDARD.length_cap as f64)
            * ScoreParams::STANDARD.length_weight_milli as f64
            / 1000.0;
        assert!(expected < 0.0, "the length term no longer ships on");
        assert!(
            (f64::from(shipped) - f64::from(off) - expected).abs() < 1e-12,
            "the live entry point moved the score by {}, expected {expected}",
            f64::from(shipped) - f64::from(off)
        );
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

    /// A leaf longer than the root has eaten inside the window, so `root_length` keeps it on the
    /// territory score even below the threshold, and a leaf that has not eaten still seeks food.
    #[test]
    fn a_leaf_that_ate_inside_the_window_scores_as_fed() {
        let game =
            serde_json::from_str::<Game>(include_str!("../fixtures/start_of_game.json")).unwrap();
        let id_map = build_snake_id_map(&game);
        let board = StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();
        let length = board.get_length_i64(board.you_id());

        let always_eat = ScoreParams {
            low_health_duel: 101,
            low_health_crowded: 101,
            ..ScoreParams::STANDARD
        };
        for (root_length, fed) in [
            (None, false),
            (Some(length), false),
            (Some(length - 1), true),
        ] {
            let score = standard_score_tail_aware_with_params::<_, u8, 4>(
                &board,
                ScoreParams {
                    root_length,
                    ..always_eat
                },
            );
            assert_eq!(
                matches!(score, Score::FloodFill(_)),
                fed,
                "root length {root_length:?} vs leaf length {length}: {score:?}"
            );
        }
    }

    /// Our head at (5,5) with food one step Right, on a crowded board with every rival in a far
    /// corner, so the meal is free.
    fn food_next_to_head(health: i64) -> StandardCellBoard4Snakes11x11 {
        let snake = |id: &str, body: [(i32, i32); 3], health: i64| {
            serde_json::json!({
                "id": id, "name": id, "health": health, "latency": null, "length": 3,
                "head": {"x": body[0].0, "y": body[0].1},
                "body": body.iter().map(|(x, y)| serde_json::json!({"x": x, "y": y})).collect::<Vec<_>>(),
            })
        };
        let you = snake("you", [(5, 5), (5, 4), (5, 3)], health);
        let game: Game = serde_json::from_value(serde_json::json!({
            "game": {"id": "food-next-to-head", "ruleset": {"name": "standard", "version": "v1"}, "timeout": 500},
            "turn": 60,
            "you": you,
            "board": {
                "height": 11, "width": 11, "hazards": [],
                "food": [{"x": 6, "y": 5}, {"x": 10, "y": 0}],
                "snakes": [
                    you,
                    snake("r1", [(0, 10), (0, 9), (0, 8)], 100),
                    snake("r2", [(10, 10), (10, 9), (10, 8)], 100),
                ],
            },
        }))
        .unwrap();
        let id_map = build_snake_id_map(&game);
        StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap()
    }

    fn best_move_at_depth(
        board: StandardCellBoard4Snakes11x11,
        params: ScoreParams,
        depth: usize,
    ) -> Option<Move> {
        let me = *board.you_id();
        let score = move |b: &StandardCellBoard4Snakes11x11| {
            standard_score_tail_aware_with_params::<_, u8, 4>(b, params)
        };
        let game_info = serde_json::from_value(serde_json::json!({
            "id": "food-next-to-head", "ruleset": {"name": "standard", "version": "v1"}, "timeout": 500
        }))
        .unwrap();
        ParanoidMinimaxSnake::new(board, game_info, 60, score, "hover", Default::default())
            .deepend_minimax_to_turn(depth)
            .your_best_move(&me)
    }

    /// The horizon effect the root length exists to fix. Food mode scores `-dist_to_food`, and
    /// eating destroys the food you were next to, so a leaf that ate early in the window -- and is
    /// back under the threshold by the leaf -- scores *worse* than one still hovering beside the
    /// food. With a threshold of 100 only a meal on the very last round counts, so from two rounds
    /// on the search keeps scheduling the meal for the horizon and never takes it.
    ///
    /// Knowing the root length makes every meal in the window count, and the search eats.
    #[test]
    fn a_meal_early_in_the_window_counts_once_the_root_length_is_known() {
        let always_eat_crowded = ScoreParams {
            low_health_crowded: 100,
            ..ScoreParams::STANDARD
        };
        let board = food_next_to_head(99);
        let root_length = board.get_length_i64(board.you_id());
        let fed = ScoreParams {
            root_length: Some(root_length),
            ..always_eat_crowded
        };

        assert_ne!(
            best_move_at_depth(board, always_eat_crowded, 2),
            Some(Move::Right),
            "without the root length a 2-round search should hover beside the food"
        );
        for depth in 1..=3 {
            assert_eq!(
                best_move_at_depth(board, fed, depth),
                Some(Move::Right),
                "with the root length a {depth}-round search should eat"
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
        let root_length = game.get_length_i64(game.you_id());
        let score = move |board: &WrappedCellBoard4Snakes11x11| {
            standard_score_tail_aware::<_, _, 4>(board, root_length)
        };
        let hobbs = ParanoidMinimaxSnake::new(game, game_info, turn, score, name, options);

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
