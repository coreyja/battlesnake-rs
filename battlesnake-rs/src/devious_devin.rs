use super::*;

pub struct DeviousDevin {}

use battlesnake_game_types::compact_representation::CellBoard;
use battlesnake_game_types::types::{
    build_snake_id_map, FoodGettableGame, HeadGettableGame, HealthGettableGame, LengthGettableGame,
    Move, SimulableGame, SimulatorInstruments, SnakeIDGettableGame, VictorDeterminableGame,
    YouDeterminableGame,
};
use battlesnake_game_types::wire_representation::Game;
use itertools::Itertools;
use std::clone::Clone;
use tracing::info;

use crate::a_prime::APrimeCalculable;

#[derive(Serialize)]
pub struct MoveOption {
    moves: Vec<SnakeMove>,
    score: ScoreEndState,
    dir: Move,
}

#[derive(Serialize)]
pub struct EvaluateOutput {
    options: Vec<MoveOption>,
}

impl<
        T: SnakeIDGettableGame
            + YouDeterminableGame
            + PositionGettableGame
            + HeadGettableGame
            + LengthGettableGame
            + HealthGettableGame
            + VictorDeterminableGame
            + HeadGettableGame
            + SimulableGame<Instruments>
            + Clone
            + APrimeCalculable
            + FoodGettableGame
            + Send
            + 'static,
    > BattlesnakeAI<T> for DeviousDevin
{
    fn make_move(
        &self,
        game_state: T,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let best_option = deepened_minimax(game_state);
        let dir = best_option.my_best_move();

        Ok(MoveOutput {
            r#move: format!("{}", dir),
            shout: None,
        })
    }

    fn name(&self) -> String {
        "devious-devin".to_owned()
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            apiversion: "1".to_owned(),
            author: Some("coreyja".to_owned()),
            color: Some("#99cc00".to_owned()),
            head: Some("snail".to_owned()),
            tail: Some("rbc-necktie".to_owned()),
            version: None,
        }
    }
}

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

const BEST_POSSIBLE_SCORE_STATE: ScoreEndState = ScoreEndState::Win(i64::MAX);
const WORT_POSSIBLE_SCORE_STATE: ScoreEndState = ScoreEndState::Lose(i64::MIN);

fn score<
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
    let mut snake_ids = node.get_snake_ids().into_iter();
    snake_ids.next();
    let me_id = node.you_id();

    let opponents: Vec<T::SnakeIDType> = snake_ids.collect();

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

    let foods: Vec<_> = node.get_all_food_as_native_positions();
    if max_opponent_length >= my_length || my_health < 20 {
        let negative_closest_food_distance =
            node.shortest_distance(&my_head, &foods, None).map(|x| -x);

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

#[derive(Clone, Debug, Serialize)]
struct SnakeMove {
    snake_name: String,
    snake_id: String,
    dir: Move,
    move_to: Position,
}

#[derive(Debug, Clone, Serialize)]
pub enum MinMaxReturn<T: SnakeIDGettableGame + Clone + Debug> {
    MinLayer {
        options: Vec<(Vec<(T::SnakeIDType, Move)>, MinMaxReturn<T>)>,
        score: ScoreEndState,
    },
    MaxLayer {
        options: Vec<(Move, MinMaxReturn<T>)>,
        score: ScoreEndState,
    },
    Leaf {
        score: ScoreEndState,
    },
}

impl<T: SnakeIDGettableGame + Clone + Debug> MinMaxReturn<T> {
    fn score(&self) -> &ScoreEndState {
        match self {
            MinMaxReturn::MinLayer { score, .. } => score,
            MinMaxReturn::MaxLayer { score, .. } => score,
            MinMaxReturn::Leaf { score } => score,
        }
    }

    fn my_best_move(&self) -> Move {
        match self {
            MinMaxReturn::Leaf { .. } => {
                unreachable!("We shouldn't ever get a leaf at the top level {:?}", &self)
            }
            MinMaxReturn::MinLayer { .. } => {
                unreachable!(
                    "We shouldn't ever get a min layer at the top level {:?}",
                    &self
                )
            }
            MinMaxReturn::MaxLayer { options, .. } => options.first().unwrap().0,
        }
    }
}

type AllPossibleStatesHashedByMyMove<T> = HashMap<
    (<T as SnakeIDGettableGame>::SnakeIDType, Move),
    Vec<(Vec<(<T as SnakeIDGettableGame>::SnakeIDType, Move)>, T)>,
>;

type AllPossibleStatesGroupedByMyMove<T> = Vec<(
    (<T as SnakeIDGettableGame>::SnakeIDType, Move),
    Vec<(Vec<(<T as SnakeIDGettableGame>::SnakeIDType, Move)>, T)>,
    Option<MinMaxReturn<T>>,
)>;

fn ordered_moves<
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + LengthGettableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + HeadGettableGame
        + SimulableGame<Instruments>
        + Clone
        + APrimeCalculable
        + FoodGettableGame,
>(
    mut grouped: AllPossibleStatesHashedByMyMove<T>,
    previous_return: Option<MinMaxReturn<T>>,
    you_id: T::SnakeIDType,
) -> AllPossibleStatesGroupedByMyMove<T> {
    match previous_return {
        None | Some(MinMaxReturn::Leaf { .. }) => grouped
            .into_iter()
            .map(|(m, others)| (m, others, None))
            .collect(),
        Some(MinMaxReturn::MaxLayer { options, .. }) => {
            let mut sorted_moves = options
                .into_iter()
                .map(|(m, prev)| (m, Some(prev)))
                .collect_vec();
            let mut missing_options = grouped
                .keys()
                .map(|(_, m)| m)
                .filter(|m| !sorted_moves.iter().map(|x| x.0).contains(m))
                .cloned()
                .map(|m| (m, None))
                .collect_vec();
            sorted_moves.append(&mut missing_options);

            sorted_moves
                .into_iter()
                .map(|(m, prev)| {
                    let key = (you_id.clone(), m);
                    (key.clone(), grouped.remove(&key).unwrap(), prev)
                })
                .collect_vec()
        }
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct Instruments;
impl SimulatorInstruments for Instruments {
    fn observe_simulation(&self, _: std::time::Duration) {}
}

use std::collections::HashMap;

fn group_pairs<A, B, C, I>(v: I) -> HashMap<A, Vec<(B, C)>>
where
    A: Eq + std::hash::Hash,
    I: IntoIterator<Item = (A, B, C)>,
{
    v.into_iter().fold(HashMap::new(), |mut acc, (a, b, c)| {
        acc.entry(a).or_default().push((b, c));
        acc
    })
}

fn minimax_min<
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + LengthGettableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + HeadGettableGame
        + SimulableGame<Instruments>
        + Clone
        + APrimeCalculable
        + FoodGettableGame,
>(
    other_moves_and_baords: Vec<(Vec<(T::SnakeIDType, Move)>, T)>,
    depth: usize,
    mut alpha: ScoreEndState,
    mut beta: ScoreEndState,
    max_depth: usize,
    previous_return: Option<MinMaxReturn<T>>,
) -> MinMaxReturn<T> {
    let mut prev_options = match previous_return {
        Some(MinMaxReturn::MinLayer { options, .. }) => options,
        None => vec![],
        _ => unreachable!("We got a bad previous return in a min node"),
    };

    let mut options: Vec<(Vec<(T::SnakeIDType, Move)>, MinMaxReturn<T>)> = vec![];
    let is_maximizing = false;

    let mut other_moves_and_baords = other_moves_and_baords
        .into_iter()
        .map(|(other_moves, board)| {
            let i = prev_options.iter().position(|(ms, _)| ms == &other_moves);
            let prev = i.map(|i| prev_options.remove(i).1);

            (i, prev, other_moves, board)
        })
        .collect_vec();

    other_moves_and_baords.sort_by_key(|(i, _prev, _other_moves, _board)| i.unwrap_or(usize::MAX));

    for (_, prev, moves, board) in other_moves_and_baords.into_iter() {
        let next_move_return = minimax(&board, depth + 1, alpha, beta, max_depth, prev);

        let value = *next_move_return.score();
        options.push((moves, next_move_return));

        if is_maximizing {
            alpha = std::cmp::max(alpha, value);
        } else {
            beta = std::cmp::min(beta, value);
        }
        if beta <= alpha {
            break;
        }
    }

    options.sort_by_cached_key(|(_, value)| *value.score());

    if is_maximizing {
        options.reverse();
    }

    let chosen_score = *options
        .first()
        .map(|x| x.1.score())
        .unwrap_or(&ScoreEndState::Lose(0));

    MinMaxReturn::MinLayer {
        options,
        score: chosen_score,
    }
}

type SnakeMoves<T> = Vec<(<T as SnakeIDGettableGame>::SnakeIDType, Move)>;

pub fn minmax_bench_entry<T>(game_state: T, max_turns: usize) -> MinMaxReturn<T>
where
    T: YouDeterminableGame
        + VictorDeterminableGame
        + SnakeIDGettableGame
        + FoodGettableGame
        + APrimeCalculable
        + HealthGettableGame
        + LengthGettableGame
        + HeadGettableGame
        + std::clone::Clone
        + SimulableGame<Instruments>,
{
    let max_depth = max_turns * 2;
    minimax(
        &game_state,
        0,
        devious_devin::WORT_POSSIBLE_SCORE_STATE,
        devious_devin::BEST_POSSIBLE_SCORE_STATE,
        max_depth,
        None,
    )
}

pub fn minmax_deepened_bench_entry<T>(game_state: T, max_turns: usize) -> MinMaxReturn<T>
where
    T: YouDeterminableGame
        + VictorDeterminableGame
        + SnakeIDGettableGame
        + FoodGettableGame
        + APrimeCalculable
        + HealthGettableGame
        + LengthGettableGame
        + HeadGettableGame
        + std::clone::Clone
        + SimulableGame<Instruments>,
{
    let max_depth = max_turns * 2;
    let mut current_depth = 2;
    let mut current_return = None;
    while current_depth <= max_depth {
        current_return = Some(minimax(
            &game_state,
            0,
            WORT_POSSIBLE_SCORE_STATE,
            BEST_POSSIBLE_SCORE_STATE,
            current_depth,
            current_return,
        ));

        current_depth += 2;
    }

    current_return.unwrap()
}

fn minimax<T>(
    node: &T,
    depth: usize,
    mut alpha: ScoreEndState,
    mut beta: ScoreEndState,
    max_depth: usize,
    previous_return: Option<MinMaxReturn<T>>,
) -> MinMaxReturn<T>
where
    T: YouDeterminableGame
        + VictorDeterminableGame
        + SnakeIDGettableGame
        + FoodGettableGame
        + APrimeCalculable
        + HealthGettableGame
        + LengthGettableGame
        + HeadGettableGame
        + std::clone::Clone
        + SimulableGame<Instruments>,
{
    if let Some(MinMaxReturn::MinLayer { .. }) = previous_return {
        unreachable!(
            "We got a bad previous return in a max node {:?}",
            previous_return
        );
    };

    let you_id = node.you_id();

    if node.is_over() {
        let score = match node.get_winner() {
            Some(s) => {
                if s == *you_id {
                    ScoreEndState::Win(-(depth as i64))
                } else {
                    ScoreEndState::Lose(depth as i64)
                }
            }
            None => ScoreEndState::Tie(depth as i64),
        };

        return MinMaxReturn::Leaf { score };
    }

    if depth >= max_depth {
        let score = score(node);
        return MinMaxReturn::Leaf { score };
    }

    let all_possible_next_moves = node.simulate(&Instruments, node.get_snake_ids());
    let leafs = all_possible_next_moves
        .into_iter()
        .map(|(moves, new_board)| {
            let (you_moves, other_moves): (SnakeMoves<T>, SnakeMoves<T>) =
                moves.into_iter().partition(|(id, _)| id == you_id);
            let you_move = you_moves
                .first()
                .expect("There should be a move by you in each move set");

            (you_move.clone(), other_moves, new_board)
        });
    let grouped: AllPossibleStatesHashedByMyMove<T> = group_pairs(leafs);

    let mut options: Vec<(Move, MinMaxReturn<T>)> = vec![];
    let is_maximizing = true;

    for (you_move, other_moves_and_baords, previous_return) in
        ordered_moves(grouped, previous_return, you_id.clone()).into_iter()
    {
        let next_move_return = minimax_min(
            other_moves_and_baords,
            depth + 1,
            alpha,
            beta,
            max_depth,
            previous_return,
        );
        let value = *next_move_return.score();
        options.push((you_move.1, next_move_return));

        if is_maximizing {
            alpha = std::cmp::max(alpha, value);
        } else {
            beta = std::cmp::min(beta, value);
        }
        if beta <= alpha {
            break;
        }
    }

    options.sort_by_cached_key(|(_, value)| *value.score());

    if is_maximizing {
        options.reverse();
    }
    // let chosen_score = *options[0].1.score();
    let chosen_score = *options
        .first()
        .map(|x| x.1.score())
        .unwrap_or(&ScoreEndState::Lose(0));

    MinMaxReturn::MaxLayer {
        options,
        score: chosen_score,
    }
}

use std::fmt::Debug;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn deepened_minimax<
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + LengthGettableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + HeadGettableGame
        + SimulableGame<Instruments>
        + Clone
        + APrimeCalculable
        + FoodGettableGame,
>(
    node: T,
) -> MinMaxReturn<T>
where
    T: SnakeIDGettableGame
        + std::clone::Clone
        + YouDeterminableGame
        + SimulableGame<Instruments>
        + VictorDeterminableGame
        + Send
        + 'static,
{
    const RUNAWAY_DEPTH_LIMIT: usize = 100;

    let started_at = Instant::now();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut current_depth = 2;
        let mut current_return = None;
        loop {
            current_return = Some(minimax(
                &node,
                0,
                WORT_POSSIBLE_SCORE_STATE,
                BEST_POSSIBLE_SCORE_STATE,
                current_depth,
                current_return,
            ));

            if tx.send((current_depth, current_return.clone())).is_err() {
                return;
            }
            current_depth += 2;
        }
    });

    let mut current = None;

    while started_at.elapsed() < Duration::new(0, 400_000_000) {
        if let Ok((depth, result)) = rx.try_recv() {
            current = result;
            info!(depth, current_score = ?current.as_ref().map(|x| x.score()), current_direction = ?current.as_ref().map(|x| x.my_best_move()), "Just finished depth");

            if depth > RUNAWAY_DEPTH_LIMIT {
                break;
            };
        }
    }

    current.expect("We weren't able to do even a single layer of minmax")
}

#[cfg(test)]
mod tests {}
