use crate::a_prime::APrimeCalculable;
use crate::devious_devin_mutable::{
    score, Instruments, MinMaxReturn, ScoreEndState, BEST_POSSIBLE_SCORE_STATE,
    WORT_POSSIBLE_SCORE_STATE,
};
use crate::*;

use battlesnake_game_types::compact_representation::{
    BestCellBoard, MoveEvaluatableGame, ToBestCellBoard,
};
use battlesnake_game_types::types::*;
use battlesnake_game_types::wire_representation::NestedGame;

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, info_span};

pub struct DeviousDevin<T> {
    game: T,
    game_info: NestedGame,
    turn: i32,
}

impl<T> BattlesnakeAI for DeviousDevin<T>
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
        + Sync
        + Copy
        + APrimeCalculable
        + FoodGettableGame
        + MoveEvaluatableGame
        + Send
        + 'static,
{
    fn make_move(&self) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let my_id = self.game.you_id();
        let mut sorted_ids = self.game.get_snake_ids();
        sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

        let best_option =
            info_span!("deepened_minmax", game_id = %&self.game_info.id, turn = self.turn, ruleset_name = %self.game_info.ruleset.name, ruleset_version = %self.game_info.ruleset.version).in_scope(|| self.deepened_minimax(sorted_ids));

        Ok(MoveOutput {
            r#move: format!(
                "{}",
                best_option
                    .direction_for(&my_id)
                    .expect("TODO: this needs to be handled")
            ),
            shout: None,
        })
    }
}

fn wrapped_score<T>(node: &T, depth: i64, max_depth: i64, num_players: i64) -> Option<ScoreEndState>
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
        + FoodGettableGame,
{
    if depth % num_players != 0 {
        return None;
    }

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

        return Some(score);
    }

    if depth >= max_depth {
        let score = score(node);
        return Some(score);
    }

    None
}

fn minimax<T>(
    mut node: T,
    players: &[T::SnakeIDType],
    depth: usize,
    alpha: ScoreEndState,
    beta: ScoreEndState,
    max_depth: usize,
    previous_return: Option<MinMaxReturn<T>>,
    mut pending_moves: Vec<(T::SnakeIDType, Move)>,
) -> MinMaxReturn<T>
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
        + Copy
        + MoveEvaluatableGame
        + APrimeCalculable
        + FoodGettableGame,
{
    let mut alpha = alpha;
    let mut beta = beta;

    if pending_moves.len() == node.get_snake_ids().len() {
        node = node.evaluate_moves(&pending_moves);
        pending_moves = vec![];
    };

    let new_depth = depth.try_into().unwrap();
    if let Some(s) = wrapped_score(&node, new_depth, max_depth as i64, players.len() as i64) {
        return MinMaxReturn::Leaf { score: s };
    }

    let snake_id = &players[depth % players.len()];

    if !node.is_alive(snake_id) {
        return minimax(
            node,
            players,
            depth + 1,
            alpha,
            beta,
            max_depth,
            previous_return,
            pending_moves,
        );
    }

    let mut options: Vec<(Move, MinMaxReturn<T>)> = vec![];

    let is_maximizing = snake_id == node.you_id();

    let possible_moves = node.possible_moves(&node.get_head_as_native_position(&snake_id));

    let possible_zipped: Vec<((Move, T::NativePositionType), Option<MinMaxReturn<T>>)> =
        if let Some(MinMaxReturn::Node { mut options, .. }) = previous_return {
            let mut v: Vec<_> = possible_moves
                .into_iter()
                .map(|m| {
                    (
                        m.clone(),
                        options
                            .iter()
                            .position(|x| x.0 == m.0)
                            .map(|x| options.remove(x).1),
                    )
                })
                .collect();
            v.sort_by_cached_key(|(_, r)| r.as_ref().map(|x| *x.score()));
            v.reverse();
            v
        } else {
            possible_moves.into_iter().map(|m| (m, None)).collect()
        };

    for ((dir, _coor), previous_return) in possible_zipped.into_iter() {
        // let last_move = node.move_to(&coor, &snake_id);
        let mut new_pending_moves = pending_moves.clone();
        new_pending_moves.push((snake_id.clone(), dir));
        let next_move_return = minimax(
            node,
            players,
            depth + 1,
            alpha,
            beta,
            max_depth,
            previous_return,
            new_pending_moves,
        );
        let value = *next_move_return.score();
        // node.reverse_move(last_move);
        options.push((dir, next_move_return));

        if is_maximizing {
            alpha = std::cmp::max(alpha, value);
        } else {
            beta = std::cmp::min(beta, value);
        }
        if beta < alpha {
            break;
        }
    }

    options.sort_by_cached_key(|(_, value)| *value.score());

    if is_maximizing {
        options.reverse();
    }
    let chosen_score = *options[0].1.score();

    MinMaxReturn::Node {
        options,
        is_maximizing,
        moving_snake_id: snake_id.clone(),
        score: chosen_score,
    }
}

impl<T> DeviousDevin<T>
where
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + VictorDeterminableGame
        + Send
        + 'static
        + PositionGettableGame
        + HeadGettableGame
        + LengthGettableGame
        + HealthGettableGame
        + Clone
        + Copy
        + APrimeCalculable
        + MoveEvaluatableGame
        + SimulableGame<Instruments>
        + FoodGettableGame,
{
    fn time_limit_ms(&self) -> i64 {
        const NETWORK_LATENCY_PADDING: i64 = 100;
        self.game_info.timeout - NETWORK_LATENCY_PADDING
    }

    fn max_duration(&self) -> Duration {
        Duration::new(0, (self.time_limit_ms() * 1_000_000).try_into().unwrap())
    }

    fn deepened_minimax(&self, players: Vec<T::SnakeIDType>) -> MinMaxReturn<T> {
        // println!("{}", self.game);
        let node = self.game.clone();
        let you_id = node.you_id();

        const RUNAWAY_DEPTH_LIMIT: usize = 100;

        let started_at = Instant::now();

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut current_depth = players.len();
            let mut current_return = None;
            loop {
                let next = minimax(
                    node,
                    &players,
                    0,
                    WORT_POSSIBLE_SCORE_STATE,
                    BEST_POSSIBLE_SCORE_STATE,
                    current_depth,
                    current_return,
                    vec![],
                );

                if tx.send((current_depth, next.clone())).is_err() {
                    return;
                }

                current_return = Some(next);

                current_depth += players.len();
            }
        });

        let mut current = None;

        let max_duration = self.max_duration();

        while started_at.elapsed() < max_duration {
            if let Ok((depth, result)) = rx.try_recv() {
                let current_score = result.score();
                let terminal_depth = current_score.terminal_depth();
                info!(depth, current_score = ?&current_score, current_direction = ?result.direction_for(you_id), "Just finished depth");

                // println!("{}", self.game.evaluate_moves(&result.all_moves()));
                current = Some((depth, result));

                if let Some(terminal_depth) = terminal_depth {
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
            // println!("{}", result.to_text_tree().unwrap());
            info!(depth, score = ?result.score(), direction = ?result.direction_for(you_id), all_moves = ?result.all_moves(), "Finished deepened_minimax");
        }

        current
            .map(|(_depth, result)| result)
            .expect("We weren't able to do even a single layer of minmax")
    }
}

pub fn minmax_bench_entry<T>(game_state: T, max_turns: usize) -> MinMaxReturn<T>
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
        + Copy
        + APrimeCalculable
        + MoveEvaluatableGame
        + FoodGettableGame,
{
    let my_id = game_state.you_id();
    let mut sorted_ids = game_state.get_snake_ids();
    sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

    minimax(
        game_state,
        &sorted_ids,
        0,
        WORT_POSSIBLE_SCORE_STATE,
        BEST_POSSIBLE_SCORE_STATE,
        max_turns * sorted_ids.len(),
        None,
        vec![],
    )
}

pub fn minmax_deepened_bench_entry<T>(game_state: T, max_turns: usize) -> MinMaxReturn<T>
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
        + MoveEvaluatableGame
        + Copy
        + FoodGettableGame,
{
    let my_id = game_state.you_id();
    let mut sorted_ids = game_state.get_snake_ids();
    sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

    let players = sorted_ids;

    let max_depth = max_turns * players.len();
    let mut current_depth = players.len();
    let mut current_return = None;
    while current_depth <= max_depth {
        current_return = Some(minimax(
            game_state,
            &players,
            0,
            WORT_POSSIBLE_SCORE_STATE,
            BEST_POSSIBLE_SCORE_STATE,
            current_depth,
            current_return,
            vec![],
        ));

        current_depth += players.len();
    }

    current_return.unwrap()
}

pub struct DeviousDevinFactory;

impl BattlesnakeFactory for DeviousDevinFactory {
    fn name(&self) -> String {
        "devious-devin".to_owned()
    }

    fn from_wire_game(&self, game: Game) -> BoxedSnake {
        let game_info = game.game.clone();
        let turn = game.turn;

        let best_board = game.to_best_cell_board().unwrap();
        let inner: BoxedSnake = match best_board {
            BestCellBoard::Standard(b) => Box::new(DeviousDevin {
                game_info,
                turn,
                game: *b,
            }),
            BestCellBoard::LargestU8(b) => Box::new(DeviousDevin {
                game_info,
                turn,
                game: *b,
            }),
            BestCellBoard::Large(b) => Box::new(DeviousDevin {
                game_info,
                turn,
                game: *b,
            }),
            BestCellBoard::Silly(b) => Box::new(DeviousDevin {
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
