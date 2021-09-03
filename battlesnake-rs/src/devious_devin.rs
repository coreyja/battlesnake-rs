use super::*;

pub struct DeviousDevin {}

use battlesnake_game_types::compact_representation::{
    CellBoard, CellBoard4Snakes11x11, CellIndex, CellNum,
};
use battlesnake_game_types::types::{
    build_snake_id_map, Move, SimulableGame, SimulatorInstruments, SnakeIDGettableGame, SnakeId,
    SnakeLocationDeterminableGame, VictorDeterminableGame, YouDeterminableGame,
};
use tracing::{info, info_span};

#[derive(Serialize)]
pub struct MoveOption {
    moves: Vec<SnakeMove>,
    score: ScoreEndState,
    dir: Direction,
}

#[derive(Serialize)]
pub struct EvaluateOutput {
    options: Vec<MoveOption>,
}

impl BattlesnakeAI for DeviousDevin {
    fn make_move(
        &self,
        game_state: GameState,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let json_value: serde_json::Value = serde_json::json!(game_state);
        let game_state: battlesnake_game_types::wire_representation::Game =
            serde_json::from_value::<battlesnake_game_types::wire_representation::Game>(json_value)
                .unwrap();
        let id_map = build_snake_id_map(&game_state);
        let game_state: battlesnake_game_types::compact_representation::CellBoard4Snakes11x11 =
            CellBoard::convert_from_game(game_state, &id_map).unwrap();
        let my_id = game_state.you_id();
        let mut sorted_snakes = game_state.get_snake_ids();
        sorted_snakes.sort_by_key(|snake| if snake == my_id { -1 } else { 1 });

        let mut players: Vec<_> = sorted_snakes.into_iter().map(Player::Snake).collect();

        let best_option = deepened_minimax(game_state, players);
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
enum ScoreEndState {
    /// depth: i64
    Lose(i64),
    /// depth: i64
    Tie(i64),
    /// difference_in_snake_length: u16, negative_distance_to_nearest_food: Option<i32>, health: u8
    ShorterThanOpponent(i64, Option<i32>, u8),
    /// negative_distance_to_opponent: Option<i64>, difference_in_snake_length: i64, health: u8
    LongerThanOpponent(Option<i32>, i64, u8),
    /// depth: i64
    Win(i64),
}

const BEST_POSSIBLE_SCORE_STATE: ScoreEndState = ScoreEndState::Win(i64::MAX);
const WORT_POSSIBLE_SCORE_STATE: ScoreEndState = ScoreEndState::Lose(i64::MIN);

fn score(
    node: &battlesnake_game_types::compact_representation::CellBoard4Snakes11x11,
    depth: i64,
    max_depth: i64,
    num_players: i64,
) -> Option<ScoreEndState> {
    if depth % num_players != 0 {
        return None;
    }

    let mut snake_ids = node.get_snake_ids().into_iter();
    snake_ids.next();
    let me_id = node.you_id();

    if node.is_over() {
        return Some(match node.get_winner() {
            Some(s) => {
                if s == *me_id {
                    ScoreEndState::Win(-depth)
                } else {
                    ScoreEndState::Lose(depth)
                }
            }
            None => ScoreEndState::Tie(depth),
        });
    }

    let opponents: Vec<SnakeId> = snake_ids.collect();

    let opponent_heads: Vec<_> = opponents
        .iter()
        .map(|s| node.get_snake_head_position(s).unwrap())
        .collect();
    let my_head = node.get_snake_head_position(me_id).unwrap();

    let my_length = node.lengths[me_id.0 as usize] as i64;

    if depth >= max_depth {
        let max_opponent_length = *node.lengths.iter().max().unwrap() as i64;
        let length_difference = my_length - max_opponent_length;
        let my_health = node.healths[me_id.0 as usize];

        let foods: Vec<_> = node
            .cells
            .iter()
            .filter(|c| c.is_food())
            .map(|c| c.idx)
            .collect();
        if max_opponent_length >= my_length || my_health < 20 {
            let negative_closest_food_distance =
                compact_a_prime::shortest_distance(&node, &my_head, &foods, None).map(|x| -x);

            return Some(ScoreEndState::ShorterThanOpponent(
                length_difference,
                negative_closest_food_distance,
                my_health.max(50),
            ));
        }

        let negative_distance_to_opponent =
            compact_a_prime::shortest_distance(&node, &my_head, &opponent_heads, None)
                .map(|dist| -dist);

        return Some(ScoreEndState::LongerThanOpponent(
            negative_distance_to_opponent,
            length_difference.max(4),
            my_health.max(50),
        ));
    }

    None
}

use std::convert::TryInto;

#[derive(Clone, Debug, Serialize)]
struct SnakeMove {
    snake_name: String,
    snake_id: String,
    dir: Direction,
    move_to: Coordinate,
}

#[derive(Debug, Clone, Serialize)]
enum MinMaxReturn {
    MinLayer {
        options: Vec<(Vec<(SnakeId, Move)>, MinMaxReturn)>,
        score: ScoreEndState,
    },
    MaxLayer {
        options: Vec<(Move, MinMaxReturn)>,
        score: ScoreEndState,
    },
    Leaf {
        score: ScoreEndState,
    },
}

impl MinMaxReturn {
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
                unreachable!("We shouldn't ever get a leaf at the top level")
            }
            MinMaxReturn::MinLayer { .. } => {
                unreachable!("We shouldn't ever get a min layer at the top level")
            }
            MinMaxReturn::MaxLayer { options, .. } => options.first().unwrap().0,
        }
    }
}

enum Player {
    Snake(SnakeId),
}

#[derive(Debug)]
struct Instruments;
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

fn minimax_min(
    other_moves_and_baords: Vec<(
        Vec<(SnakeId, Move)>,
        battlesnake_game_types::compact_representation::CellBoard4Snakes11x11,
    )>,
    players: &[Player],
    depth: usize,
    mut alpha: ScoreEndState,
    mut beta: ScoreEndState,
    max_depth: usize,
    previous_return: Option<MinMaxReturn>,
) -> MinMaxReturn {
    let mut options: Vec<(Vec<(SnakeId, Move)>, MinMaxReturn)> = vec![];
    let is_maximizing = false;

    for (moves, board) in other_moves_and_baords.into_iter() {
        let next_move_return = minimax(board, players, depth + 1, alpha, beta, max_depth, None);

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

fn minimax(
    node: battlesnake_game_types::compact_representation::CellBoard4Snakes11x11,
    players: &[Player],
    depth: usize,
    mut alpha: ScoreEndState,
    mut beta: ScoreEndState,
    max_depth: usize,
    previous_return: Option<MinMaxReturn>,
) -> MinMaxReturn {
    let new_depth = depth.try_into().unwrap();
    if let Some(s) = score(&node, new_depth, max_depth as i64, players.len() as i64) {
        return MinMaxReturn::Leaf { score: s };
    }

    let you_id = node.you_id();
    let all_possible_next_moves = node.simulate(&Instruments, node.get_snake_ids());
    let leafs = all_possible_next_moves
        .into_iter()
        .map(|(moves, new_board)| {
            let (you_moves, other_moves): (Vec<(SnakeId, Move)>, Vec<(SnakeId, Move)>) =
                moves.iter().partition(|(id, _)| id == you_id);
            let you_move = you_moves.first().unwrap();

            (*you_move, other_moves, new_board)
        });
    let grouped = group_pairs(leafs);

    let mut options: Vec<(Move, MinMaxReturn)> = vec![];
    let is_maximizing = true;

    for (you_move, other_moves_and_baords) in grouped.into_iter() {
        let next_move_return = minimax_min(
            other_moves_and_baords,
            players,
            depth + 1,
            alpha,
            beta,
            max_depth,
            None,
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

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn deepened_minimax(
    node: battlesnake_game_types::compact_representation::CellBoard4Snakes11x11,
    players: Vec<Player>,
) -> MinMaxReturn {
    const RUNAWAY_DEPTH_LIMIT: usize = 2_000;

    let started_at = Instant::now();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut current_depth = players.len();
        let mut current_return = None;
        let players = players;
        loop {
            current_return = Some(minimax(
                node,
                &players,
                0,
                WORT_POSSIBLE_SCORE_STATE,
                BEST_POSSIBLE_SCORE_STATE,
                current_depth,
                current_return,
            ));

            if tx.send((current_depth, current_return.clone())).is_err() {
                return;
            }
            current_depth += players.len();
        }
    });

    let mut current = None;

    while started_at.elapsed() < Duration::new(0, 400_000_000) {
        if let Ok((depth, result)) = rx.try_recv() {
            current = result;
            info!(depth, "Just finished depth");

            if depth > RUNAWAY_DEPTH_LIMIT {
                break;
            };
        }
    }

    current.unwrap_or(MinMaxReturn::Leaf {
        score: WORT_POSSIBLE_SCORE_STATE,
    })
}

#[cfg(test)]
mod tests {}
