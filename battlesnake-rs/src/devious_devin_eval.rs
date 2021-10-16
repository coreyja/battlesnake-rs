use crate::a_prime::APrimeCalculable;
use crate::devious_devin_mutable::{score, MinMaxReturn, ScoreEndState};
use crate::*;

use battlesnake_game_types::types::*;

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, info_span};

pub struct DeviousDevin<T> {
    game: T,
}

#[derive(Debug)]
pub struct Instruments;
impl SimulatorInstruments for Instruments {
    fn observe_simulation(&self, _: std::time::Duration) {}
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
        + APrimeCalculable
        + MoveableGame
        + FoodGettableGame
        + Send
        + 'static,
{
    fn make_move(&self) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let my_id = self.game.you_id();
        let mut sorted_ids = self.game.get_snake_ids();
        sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

        let mut players: Vec<_> = sorted_ids.into_iter().map(Player::Snake).collect();
        players.push(Player::Nature);

        let best_option =
            info_span!("deepened_minmax").in_scope(|| deepened_minimax(self.game.clone(), players));

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
        + MoveableGame
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
    node: &mut T,
    players: &[Player<T>],
    depth: usize,
    alpha: ScoreEndState,
    beta: ScoreEndState,
    max_depth: usize,
    previous_return: Option<MinMaxReturn<T>>,
) -> MinMaxReturn<T>
where
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + LengthGettableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + MoveableGame
        + HeadGettableGame
        + SimulableGame<Instruments>
        + Clone
        + APrimeCalculable
        + FoodGettableGame,
{
    let mut alpha = alpha;
    let mut beta = beta;

    let new_depth = depth.try_into().unwrap();
    if let Some(s) = wrapped_score(node, new_depth, max_depth as i64, players.len() as i64) {
        return MinMaxReturn::Leaf { score: s };
    }

    let player = &players[depth % players.len()];
    match player {
        Player::Snake(snake_id) => {
            let mut options: Vec<(Move, MinMaxReturn<T>)> = vec![];

            let is_maximizing = snake_id == node.you_id();

            // let possible_moves: Vec<_> = node
            //     .board
            //     .snakes
            //     .iter()
            //     .find(|s| s.id == snake.id)
            //     .expect("We didn't find that snake")
            //     .body[0]
            //     .possible_moves(&node.board)
            //     .collect();
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

            for ((dir, coor), previous_return) in possible_zipped.into_iter() {
                let last_move = node.move_to(&coor, &snake_id);
                let next_move_return = minimax(
                    node,
                    players,
                    depth + 1,
                    alpha,
                    beta,
                    max_depth,
                    previous_return,
                );
                let value = *next_move_return.score();
                node.reverse_move(last_move);
                options.push((dir, next_move_return));

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
            let chosen_score = *options[0].1.score();

            MinMaxReturn::Node {
                options,
                is_maximizing,
                moving_snake_id: snake_id.clone(),
                score: chosen_score,
            }
        }
        Player::Nature => {
            let nature_moves = node.nature_move();
            let return_value = minimax(
                node,
                players,
                depth + 1,
                alpha,
                beta,
                max_depth,
                previous_return,
            );
            for m in nature_moves.into_iter() {
                node.reverse_nature(m)
            }

            MinMaxReturn::Nature {
                score: *return_value.score(),
                next: Box::new(return_value),
            }
        }
    }
}

fn deepened_minimax<T>(node: T, players: Vec<Player<T>>) -> MinMaxReturn<T>
where
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + LengthGettableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + HeadGettableGame
        + MoveableGame
        + SimulableGame<Instruments>
        + Clone
        + Send
        + 'static
        + APrimeCalculable
        + FoodGettableGame,
{
    const RUNAWAY_DEPTH_LIMIT: usize = 2_000;

    let started_at = Instant::now();
    let me_id = node.you_id().clone();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut current_depth = players.len();
        let mut current_return = None;
        let mut node = node;
        let players = players;
        loop {
            let next = minimax(
                &mut node,
                &players,
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

            current_depth += players.len();
        }
    });

    let mut current = None;

    while started_at.elapsed() < Duration::new(0, 400_000_000) {
        if let Ok((depth, result)) = rx.try_recv() {
            info!(depth, current_score = ?result.score(), current_direction = ?result.direction_for(&me_id), "Just finished depth");

            current = Some((depth, result));

            if depth > RUNAWAY_DEPTH_LIMIT {
                break;
            };
        }
    }

    info!(score = ?current.as_ref().map(|x| x.1.score()), depth = ?current.as_ref().map(|(d, _)| d), "Finished deepened_minimax");
    current
        .map(|(_depth, result)| result)
        .unwrap_or(MinMaxReturn::Leaf {
            score: WORT_POSSIBLE_SCORE_STATE,
        })
}

pub fn minmax_bench_entry<T>(mut game_state: T, max_turns: usize) -> MinMaxReturn<T>
where
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + LengthGettableGame
        + MoveableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + HeadGettableGame
        + SimulableGame<Instruments>
        + Clone
        + APrimeCalculable
        + FoodGettableGame,
{
    let my_id = game_state.you_id();
    let mut sorted_ids = game_state.get_snake_ids();
    sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

    let mut players: Vec<_> = sorted_ids.into_iter().map(Player::Snake).collect();
    players.push(Player::Nature);

    minimax(
        &mut game_state,
        &players,
        0,
        WORT_POSSIBLE_SCORE_STATE,
        BEST_POSSIBLE_SCORE_STATE,
        max_turns * players.len(),
        None,
    )
}

pub fn minmax_deepened_bench_entry<T>(mut game_state: T, max_turns: usize) -> MinMaxReturn<T>
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
        + MoveableGame
        + FoodGettableGame,
{
    let my_id = game_state.you_id();
    let mut sorted_ids = game_state.get_snake_ids();
    sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

    let mut players: Vec<_> = sorted_ids.into_iter().map(Player::Snake).collect();
    players.push(Player::Nature);
    let players = players;

    let max_depth = max_turns * players.len();
    let mut current_depth = players.len();
    let mut current_return = None;
    while current_depth <= max_depth {
        current_return = Some(minimax(
            &mut game_state,
            &players,
            0,
            WORT_POSSIBLE_SCORE_STATE,
            BEST_POSSIBLE_SCORE_STATE,
            current_depth,
            current_return,
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
        // let id_map = build_snake_id_map(&game);
        // let game = CellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap();
        Box::new(DeviousDevin { game })
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
