use crate::a_prime::APrimeCalculable;
use crate::devious_devin_eval::{score, ScoreEndState};
use crate::devious_devin_mutable::{
    Instruments, BEST_POSSIBLE_SCORE_STATE, WORT_POSSIBLE_SCORE_STATE,
};
use crate::*;

use battlesnake_game_types::compact_representation::{BestCellBoard, ToBestCellBoard};
use battlesnake_game_types::types::*;
use battlesnake_game_types::wire_representation::{Game, NestedGame};
use itertools::Itertools;
use rand::prelude::*;
use std::clone::Clone;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, info_span};

pub struct FullDeviousDevin<T> {
    game: T,
    game_info: NestedGame,
    turn: i32,
}

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

impl<T> BattlesnakeAI for FullDeviousDevin<T>
where
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
{
    fn make_move(&self) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let best_option =
            info_span!("deepened_minmax", game_id = %&self.game_info.id, turn = self.turn, ruleset_name = %self.game_info.ruleset.name, ruleset_version = %self.game_info.ruleset.version)
                .in_scope(|| self.deepened_minimax());

        let dir = best_option.my_best_move();

        Ok(MoveOutput {
            r#move: format!("{}", dir),
            shout: None,
        })
    }
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
    fn score(&self) -> ScoreEndState {
        match self {
            MinMaxReturn::MinLayer { score, .. } => *score,
            MinMaxReturn::MaxLayer { score, .. } => *score,
            MinMaxReturn::Leaf { score } => *score,
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

        let value = next_move_return.score();
        options.push((moves, next_move_return));

        if is_maximizing {
            alpha = std::cmp::max(alpha, value);
        } else {
            beta = std::cmp::min(beta, value);
        }
        if beta < alpha {
            break;
        }
    }

    options.sort_by_cached_key(|(_, value)| value.score());

    if is_maximizing {
        options.reverse();
    }

    let chosen_score = options
        .first()
        .map(|x| x.1.score())
        .unwrap_or(ScoreEndState::Lose(0));

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
        WORT_POSSIBLE_SCORE_STATE,
        BEST_POSSIBLE_SCORE_STATE,
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

    let snake_ids = node.get_snake_ids();

    let snake_moves = if snake_ids.len() > 4 {
        let me_head = node.get_head_as_native_position(you_id);

        let mut snake_distances = snake_ids
            .iter()
            .map(|sid| {
                let oppenent_head = node.get_head_as_native_position(sid);

                (
                    sid,
                    node.shortest_distance(&me_head, &[oppenent_head], None),
                )
            })
            .collect_vec();
        snake_distances.sort_by_key(|(_, dist)| dist.unwrap_or(i32::MAX));

        let closest_snakes = snake_distances[0..3].iter().map(|x| x.0).collect_vec();

        let mut rng = thread_rng();
        snake_ids
            .iter()
            .map(|sid| {
                let ms = if sid == you_id || closest_snakes.contains(&sid) {
                    Move::all()
                } else {
                    vec![
                        node.possible_moves(&node.get_head_as_native_position(sid))
                            .choose(&mut rng)
                            .unwrap()
                            .0,
                    ]
                };

                (sid.clone(), ms)
            })
            .collect_vec()
    } else {
        snake_ids
            .into_iter()
            .map(|sid| (sid, Move::all()))
            .collect_vec()
    };

    let all_possible_next_moves = node.simulate_with_moves(&Instruments, snake_moves);
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
        let value = next_move_return.score();
        options.push((you_move.1, next_move_return));

        if is_maximizing {
            alpha = std::cmp::max(alpha, value);
        } else {
            beta = std::cmp::min(beta, value);
        }
        if beta < alpha {
            break;
        }
    }

    options.sort_by_cached_key(|(_, value)| value.score());

    if is_maximizing {
        options.reverse();
    }

    let chosen_score = options
        .first()
        .map(|x| x.1.score())
        .unwrap_or(ScoreEndState::Lose(0));

    MinMaxReturn::MaxLayer {
        options,
        score: chosen_score,
    }
}

impl<T> FullDeviousDevin<T>
where
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + SimulableGame<Instruments>
        + VictorDeterminableGame
        + Send
        + 'static
        + PositionGettableGame
        + HeadGettableGame
        + LengthGettableGame
        + HealthGettableGame
        + Clone
        + APrimeCalculable
        + FoodGettableGame,
{
    fn time_limit_ms(&self) -> i64 {
        const NETWORK_LATENCY_PADDING: i64 = 100;
        self.game_info.timeout - NETWORK_LATENCY_PADDING
    }

    fn max_duration(&self) -> Duration {
        Duration::new(0, (self.time_limit_ms() * 1_000_000).try_into().unwrap())
    }

    fn deepened_minimax(&self) -> MinMaxReturn<T> {
        let node = self.game.clone();

        const RUNAWAY_DEPTH_LIMIT: usize = 100;

        let started_at = Instant::now();

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut current_depth = 2;
            let mut current_return = None;
            loop {
                let next = minimax(
                    &node,
                    0,
                    WORT_POSSIBLE_SCORE_STATE,
                    BEST_POSSIBLE_SCORE_STATE,
                    current_depth,
                    current_return,
                );

                if tx.send((current_depth, next.clone())).is_err() {
                    return;
                }

                current_return = Some(next);

                current_depth += 2;
            }
        });

        let mut current = None;

        let max_duration = self.max_duration();

        while started_at.elapsed() < max_duration {
            if let Ok((depth, result)) = rx.try_recv() {
                let current_score = result.score();
                info!(depth, current_score = ?&current_score, current_direction = ?result.my_best_move(), "Just finished depth");

                current = Some((depth, result));

                if let Some(terminal_depth) = current_score.terminal_depth() {
                    if depth > (terminal_depth as usize) {
                        info!(depth, "This game is over, no need to keep going");
                        break;
                    }
                }

                if depth > RUNAWAY_DEPTH_LIMIT {
                    break;
                };
            }
        }

        if let Some((depth, result)) = &current {
            info!(depth, score = ?result.score(), direction = ?result.my_best_move(), "Finished deepened_minimax");
        }

        current
            .map(|(_depth, result)| result)
            .expect("We weren't able to do even a single layer of minmax")
    }
}

pub struct FullDeviousDevinFactory;

impl BattlesnakeFactory for FullDeviousDevinFactory {
    fn name(&self) -> String {
        "devious-devin".to_owned()
    }

    fn from_wire_game(&self, game: Game) -> BoxedSnake {
        let game_info = game.game.clone();
        let turn = game.turn;

        let best_board = game.to_best_cell_board().unwrap();
        let inner: BoxedSnake = match best_board {
            BestCellBoard::Tiny(b) => Box::new(FullDeviousDevin {
                game_info,
                turn,
                game: *b,
            }),
            BestCellBoard::Standard(b) => Box::new(FullDeviousDevin {
                game_info,
                turn,
                game: *b,
            }),
            BestCellBoard::LargestU8(b) => Box::new(FullDeviousDevin {
                game_info,
                turn,
                game: *b,
            }),
            BestCellBoard::Large(b) => Box::new(FullDeviousDevin {
                game_info,
                turn,
                game: *b,
            }),
            BestCellBoard::Silly(b) => Box::new(FullDeviousDevin {
                game_info,
                turn,
                game: *b,
            }),
        };

        inner
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

#[cfg(test)]
mod tests {}
