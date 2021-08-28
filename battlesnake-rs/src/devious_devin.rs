use super::*;

pub struct DeviousDevin {}

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
        let my_id = game_state.you.id.clone();
        let mut sorted_snakes = game_state.board.snakes.clone();
        sorted_snakes.sort_by_key(|snake| if snake.id == game_state.you.id { -1 } else { 1 });

        let mut players: Vec<_> = sorted_snakes.into_iter().map(Player::Snake).collect();
        players.push(Player::Nature);

        let best_option = info_span!(
            "deepened_minmax",
            game_id = ?game_state.game.id,
            turn = game_state.turn
        )
        .in_scope(|| deepened_minimax(game_state, players));

        Ok(MoveOutput {
            r#move: best_option
                .direction_for(&my_id)
                .expect("TODO: this needs to be handled")
                .value(),
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
    HitSelfLose(i64),
    /// depth: i64
    RanIntoOtherLose(i64),
    /// depth: i64
    OutOfHealthLose(i64),
    /// depth: i64
    HeadToHeadLose(i64),
    /// difference_in_snake_length: i64, negative_distance_to_nearest_food: Option<i64>, health: u8
    ShorterThanOpponent(i64, Option<i64>, i16),
    /// negative_distance_to_opponent: Option<i64>, difference_in_snake_length: i64, health: u8
    LongerThanOpponent(Option<i64>, i64, i16),
    /// negative_depth: i64
    HitSelfWin(i64),
    /// negative_depth: i64
    RanIntoOtherWin(i64),
    /// negative_depth: i64
    HeadToHeadWin(i64),
}

const BEST_POSSIBLE_SCORE_STATE: ScoreEndState = ScoreEndState::HeadToHeadWin(i64::MAX);
const WORT_POSSIBLE_SCORE_STATE: ScoreEndState = ScoreEndState::HitSelfLose(i64::MIN);

fn score(node: &GameState, depth: i64, max_depth: i64, num_players: i64) -> Option<ScoreEndState> {
    if depth % num_players != 0 {
        return None;
    }

    let me: &Battlesnake = node
        .board
        .snakes
        .iter()
        .find(|s| s.id == node.you.id)
        .unwrap();
    let opponents: Vec<&Battlesnake> = node
        .board
        .snakes
        .iter()
        .filter(|s| s.id != node.you.id)
        .collect();

    let opponent_heads: Vec<Coordinate> = opponents.iter().map(|s| s.body[0]).collect();

    let my_length: i64 = me.body.len().try_into().unwrap();

    if me.body[1..].contains(&me.body[0]) && depth != 0 {
        return Some(ScoreEndState::HitSelfLose(depth));
    }

    if me.health <= 0 {
        return Some(ScoreEndState::OutOfHealthLose(depth));
    }

    if let Some(opponent_end_state) = opponents
        .iter()
        .filter_map(|not_me| {
            if not_me.body[1..].contains(&me.body[0]) {
                return Some(ScoreEndState::RanIntoOtherLose(depth));
            }

            if not_me.body[1..].contains(&not_me.body[0]) && depth != 0 {
                return Some(ScoreEndState::HitSelfWin(-depth));
            }

            if me.body[1..].contains(&not_me.body[0]) {
                return Some(ScoreEndState::RanIntoOtherWin(-depth));
            }

            if me.body[0] == not_me.body[0] {
                if my_length > not_me.body.len().try_into().unwrap() {
                    return Some(ScoreEndState::HeadToHeadWin(-depth));
                } else {
                    return Some(ScoreEndState::HeadToHeadLose(depth));
                }
            }

            None
        })
        .min()
    {
        return Some(opponent_end_state);
    };

    if depth >= max_depth {
        let max_opponent_length: i64 = opponents
            .iter()
            .map(|s| s.body.len().try_into().unwrap())
            .max()
            .unwrap();
        let length_difference = my_length - max_opponent_length;

        if max_opponent_length >= my_length || me.health < 20 {
            let negative_closest_food_distance =
                a_prime::shortest_distance(&node.board, &me.body[0], &node.board.food, None)
                    .map(|x| -x);

            return Some(ScoreEndState::ShorterThanOpponent(
                length_difference,
                negative_closest_food_distance,
                me.health.max(50),
            ));
        }

        let negative_distance_to_opponent =
            a_prime::shortest_distance(&node.board, &me.body[0], &opponent_heads, None)
                .map(|dist| -dist);

        return Some(ScoreEndState::LongerThanOpponent(
            negative_distance_to_opponent,
            length_difference.max(4),
            me.health.max(50),
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

#[derive(Debug, Clone)]
enum MinMaxReturn {
    Node {
        is_maximizing: bool,
        options: Vec<(Direction, MinMaxReturn)>,
        moving_snake_id: String,
        score: ScoreEndState,
    },
    Nature {
        score: ScoreEndState,
        next: Box<MinMaxReturn>,
    },
    Leaf {
        score: ScoreEndState,
    },
}

impl MinMaxReturn {
    fn score(&self) -> &ScoreEndState {
        match self {
            MinMaxReturn::Node { score, .. } => score,
            MinMaxReturn::Nature { score, .. } => score,
            MinMaxReturn::Leaf { score } => score,
        }
    }

    fn direction_for(&self, snake_id: &str) -> Option<Direction> {
        match self {
            MinMaxReturn::Leaf { .. } => None,
            MinMaxReturn::Nature { next, .. } => next.direction_for(snake_id),
            MinMaxReturn::Node {
                moving_snake_id,
                options,
                ..
            } => {
                let chosen = options.first()?;
                if moving_snake_id == snake_id {
                    Some(chosen.0)
                } else {
                    chosen.1.direction_for(snake_id)
                }
            }
        }
    }
}

enum Player {
    Snake(Battlesnake),
    Nature,
}

fn minimax(
    node: &mut GameState,
    players: &[Player],
    depth: usize,
    alpha: ScoreEndState,
    beta: ScoreEndState,
    max_depth: usize,
    previous_return: Option<MinMaxReturn>,
) -> MinMaxReturn {
    let mut alpha = alpha;
    let mut beta = beta;

    let new_depth = depth.try_into().unwrap();
    if let Some(s) = score(node, new_depth, max_depth as i64, players.len() as i64) {
        return MinMaxReturn::Leaf { score: s };
    }

    let player = &players[depth % players.len()];
    match player {
        Player::Snake(snake) => {
            let mut options: Vec<(Direction, MinMaxReturn)> = vec![];

            let is_maximizing = snake.id == node.you.id;

            let possible_moves: Vec<_> = node
                .board
                .snakes
                .iter()
                .find(|s| s.id == snake.id)
                .expect("We didn't find that snake")
                .body[0]
                .possible_moves(&node.board)
                .collect();

            let possible_zipped: Vec<((Direction, Coordinate), Option<MinMaxReturn>)> =
                if let Some(MinMaxReturn::Node { mut options, .. }) = previous_return {
                    let mut v: Vec<_> = possible_moves
                        .into_iter()
                        .map(|m| {
                            (
                                m,
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
                let last_move = node.move_to(&coor, &snake.id);
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
                moving_snake_id: snake.id.to_owned(),
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

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn deepened_minimax(node: GameState, players: Vec<Player>) -> MinMaxReturn {
    const RUNAWAY_DEPTH_LIMIT: usize = 2_000;

    let started_at = Instant::now();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut current_depth = players.len();
        let mut current_return = None;
        let mut node = node;
        let players = players;
        loop {
            current_return = Some(minimax(
                &mut node,
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

    info!(score = ?current.as_ref().map(|x| x.score()), "Finished deepened_minimax");
    current.unwrap_or(MinMaxReturn::Leaf {
        score: WORT_POSSIBLE_SCORE_STATE,
    })
}

#[cfg(test)]
mod tests {}
