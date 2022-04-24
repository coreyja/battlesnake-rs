#![deny(
    warnings,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs
)]
//! This crate implements the minimax algorithm for the battlesnake game. You provide a 'scoring'
//! function that turns a given board into anything that implements the `Ord` trait.
//!
//! We lean on the `battlesnake-game-types` crate for the game logic, and in particular for the
//! simulate logic, which is used to generate the next board states.

use battlesnake_game_types::types::*;
use battlesnake_game_types::wire_representation::NestedGame;

use itertools::Itertools;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, info_span};

use std::fmt::Debug;

#[derive(Debug, Clone, Copy)]
/// Any empty struct that implements `SimulatorInstruments` as a no-op which can be used when you don't want
/// to time the simulation
pub struct Instruments {}

impl SimulatorInstruments for Instruments {
    fn observe_simulation(&self, _duration: Duration) {}
}

/// The move output to be returned to the Battlesnake Engine
#[derive(Debug, Clone)]
pub struct MoveOutput {
    /// A stringified move
    pub r#move: String,
    /// An optional shout that will be echoed to you on your next turn
    pub shout: Option<String>,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Copy)]
/// The wrapped score type. This takes into account the score provided by the score function, but
/// wraps it with a Score based on the game state. This allows us to say that wins are better than
/// any score and loses are worse than any score, etc.
pub enum WrappedScore<ScoreType>
where
    ScoreType: PartialOrd + Ord + Debug + Clone + Copy,
{
    /// We lost, the depth is recorded because we prefer surviving longer
    Lose(i64),
    /// We tied, the depth is recorded because we prefer surviving longer
    Tie(i64),
    /// We order this based on the score provided by the score function
    Scored(ScoreType),
    /// We won, the depth is recorded because we prefer winning sooner
    Win(i64),
}

impl<ScoreType> WrappedScore<ScoreType>
where
    ScoreType: PartialOrd + Ord + Debug + Clone + Copy,
{
    fn best_possible_score() -> Self {
        WrappedScore::Win(std::i64::MAX)
    }
    fn worst_possible_score() -> Self {
        WrappedScore::Lose(std::i64::MIN)
    }

    fn terminal_depth(&self) -> Option<i64> {
        match &self {
            Self::Win(d) => Some(-d),
            Self::Tie(d) | Self::Lose(d) => Some(*d),
            _ => None,
        }
    }
}

trait Scoreable<BoardType, ScoreType>: Sync + Send
where
    ScoreType: PartialOrd + Ord + Debug + Clone + Copy,
{
    fn score(&self, node: &BoardType) -> WrappedScore<ScoreType>;
}

use derivative::Derivative;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
/// This is the struct that wraps a game board and a scoring function and can be used to run
/// minimax
///
/// It also outputs traces using the `tracing` crate which can be subcribed to
pub struct EvalMinimaxSnake<T: 'static, ScoreType: 'static, const N_SNAKES: usize>
where
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + HeadGettableGame
        + NeighborDeterminableGame
        + SimulableGame<Instruments, N_SNAKES>
        + Clone
        + Copy
        + Sync
        + Send
        + Sized,
    T::SnakeIDType: Copy + Send + Sync,
    ScoreType: Clone + Debug + PartialOrd + Ord + Send + Sync + Copy,
{
    game: T,
    game_info: NestedGame,
    turn: i32,
    #[derivative(Debug = "ignore")]
    score_function: &'static (dyn Fn(&T) -> ScoreType + Sync + Send),
    name: &'static str,
}

#[derive(Debug, Clone)]
/// This is returned from an iteration of the minimax algorithm
/// It contains all the information we generated about the game tree
pub enum MinMaxReturn<
    T: SnakeIDGettableGame + Clone + Debug,
    ScoreType: Clone + Debug + PartialOrd + Ord + Copy,
> {
    /// This is a non-leaf node in the game tree
    /// We have information about all the options we looked at as well as the chosen score
    Node {
        /// Whether this node was a maximizing node or not
        is_maximizing: bool,
        /// A recursive look at all the moves under us
        options: Vec<(Move, Self)>,
        /// Which snake was moving at this node
        moving_snake_id: T::SnakeIDType,
        /// The chosen score
        score: WrappedScore<ScoreType>,
    },
    /// Represents a leaf node in the game tree
    /// This happens when we reach a terminal state (win/lose/tie)
    /// or when we reach the maximum depth
    Leaf {
        /// The score of the leaf node
        score: WrappedScore<ScoreType>,
    },
}

impl<T, ScoreType> MinMaxReturn<T, ScoreType>
where
    T: SnakeIDGettableGame + Debug + Clone,
    ScoreType: Clone + Debug + PartialOrd + Ord + Copy,
{
    /// Returns the score for this node
    pub fn score(&self) -> &WrappedScore<ScoreType> {
        match self {
            MinMaxReturn::Node { score, .. } => score,
            MinMaxReturn::Leaf { score } => score,
        }
    }

    /// Returns the direction our snake should move to maximize the score
    /// If we are a leaf node, this will return None
    ///
    /// We take advantage of the fact that the moves are sorted by score, so we can just return the
    /// first option where our snake is moving
    pub fn direction_for(&self, snake_id: &T::SnakeIDType) -> Option<Move> {
        match self {
            MinMaxReturn::Leaf { .. } => None,
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

    /// Returns all the moves in the 'route' through the game tree that minimax took
    /// This is useful for debugging as it shows each of the moves we and our opponents made during
    /// the simulation
    pub fn all_moves(&self) -> Vec<(T::SnakeIDType, Move)> {
        match self {
            MinMaxReturn::Leaf { .. } => vec![],
            MinMaxReturn::Node {
                moving_snake_id,
                options,
                ..
            } => {
                if let Some(chosen) = options.first() {
                    let mut tail = chosen.1.all_moves();
                    tail.insert(0, (moving_snake_id.clone(), chosen.0));
                    tail
                } else {
                    vec![]
                }
            }
        }
    }

    fn to_text_tree_node(&self, label: String) -> Option<StringTreeNode> {
        match self {
            MinMaxReturn::Leaf { .. } => None,
            MinMaxReturn::Node {
                moving_snake_id,
                options,
                score,
                ..
            } => {
                let mut node = StringTreeNode::new(format!("{} {:?}", label, score));
                for (m, result) in options {
                    if let Some(next_node) =
                        result.to_text_tree_node(format!("{} {:?}", m, moving_snake_id))
                    {
                        node.push_node(next_node);
                    }
                }

                Some(node)
            }
        }
    }

    /// This returns a visual representation of the game tree that minimax generated
    /// It shows the chosen score, the moving snake and the chosen move at each level
    pub fn to_text_tree(&self) -> Option<String> {
        let tree_node = self.to_text_tree_node("".to_owned())?;
        Some(format!("{}", tree_node))
    }
}

use text_trees::StringTreeNode;

impl<T, ScoreType, const N_SNAKES: usize> EvalMinimaxSnake<T, ScoreType, N_SNAKES>
where
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + HeadGettableGame
        + NeighborDeterminableGame
        + NeckQueryableGame
        + SimulableGame<Instruments, N_SNAKES>
        + Clone
        + Copy
        + Sync
        + Send
        + Sized,
    T::SnakeIDType: Copy + Send + Sync,
    ScoreType: Clone + Debug + PartialOrd + Ord + Send + Sync + Copy,
{
    /// Construct a new `EvalMinimaxSnake`
    pub fn new(
        game: T,
        game_info: NestedGame,
        turn: i32,
        score_function: &'static (dyn Fn(&T) -> ScoreType + Sync + Send),
        name: &'static str,
    ) -> Self {
        Self {
            game,
            game_info,
            turn,
            score_function,
            name,
        }
    }

    /// Pick the next move to make
    pub fn make_move_inner(&self) -> Move {
        let my_id = self.game.you_id();
        let mut sorted_ids = self.game.get_snake_ids();
        sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

        let copy = self.clone();

        let best_option =
            info_span!("deepened_minmax", snake_name = self.name, game_id = %&self.game_info.id, turn = self.turn, ruleset_name = %self.game_info.ruleset.name, ruleset_version = %self.game_info.ruleset.version).in_scope(|| copy.deepened_minimax(sorted_ids));

        best_option.direction_for(my_id).unwrap()
    }

    fn wrapped_score(
        &self,
        node: &T,
        depth: i64,
        max_depth: i64,
        num_players: i64,
    ) -> Option<WrappedScore<ScoreType>> {
        if depth % num_players != 0 {
            return None;
        }

        let you_id = node.you_id();

        if node.is_over() {
            let score = match node.get_winner() {
                Some(s) => {
                    if s == *you_id {
                        WrappedScore::Win(-(depth as i64))
                    } else {
                        WrappedScore::Lose(depth as i64)
                    }
                }
                None => WrappedScore::Tie(depth as i64),
            };

            return Some(score);
        }

        if depth >= max_depth {
            return Some(WrappedScore::Scored((self.score_function)(node)));
        }

        None
    }

    #[allow(clippy::too_many_arguments)]
    fn minimax(
        &self,
        mut node: T,
        players: &[T::SnakeIDType],
        depth: usize,
        alpha: WrappedScore<ScoreType>,
        beta: WrappedScore<ScoreType>,
        max_depth: usize,
        previous_return: Option<MinMaxReturn<T, ScoreType>>,
        mut pending_moves: Vec<(T::SnakeIDType, Move)>,
    ) -> MinMaxReturn<T, ScoreType> {
        let mut alpha = alpha;
        let mut beta = beta;

        let snake_ids = node.get_snake_ids();

        // Remove pending moves for dead snakes
        pending_moves.retain(|(snake_id, _)| snake_ids.contains(snake_id));

        if !snake_ids.is_empty() && pending_moves.len() == snake_ids.len() {
            let mut simulate_result = node.simulate_with_moves(
                &Instruments {},
                pending_moves
                    .into_iter()
                    .map(|(sid, m)| (sid, vec![m]))
                    .collect_vec(),
            );
            let new_node = simulate_result.next().unwrap().1;
            drop(simulate_result);
            node = new_node;
            pending_moves = vec![];
        };

        let new_depth = depth.try_into().unwrap();
        if let Some(s) = self.wrapped_score(
            &node,
            new_depth,
            max_depth.try_into().unwrap(),
            players.len() as i64,
        ) {
            return MinMaxReturn::Leaf { score: s };
        }

        let snake_id = &players[depth % players.len()];

        let mut options: Vec<(Move, MinMaxReturn<T, ScoreType>)> = vec![];

        let is_maximizing = snake_id == node.you_id();

        let possible_moves = if node.get_health_i64(snake_id) == 0 {
            return self.minimax(
                node,
                players,
                depth + 1,
                alpha,
                beta,
                max_depth,
                previous_return,
                pending_moves,
            );
        } else {
            assert!(node.get_health_i64(snake_id) > 0);
            node.possible_moves(&node.get_head_as_native_position(snake_id))
                .filter(|(_, pos)| !node.is_neck(snake_id, pos))
        };

        #[allow(clippy::type_complexity)]
        let possible_zipped: Vec<(
            (Move, T::NativePositionType),
            Option<MinMaxReturn<T, ScoreType>>,
        )> = if let Some(MinMaxReturn::Node { mut options, .. }) = previous_return {
            let mut v: Vec<_> = possible_moves
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
            possible_moves.map(|m| (m, None)).collect()
        };

        for ((dir, _coor), previous_return) in possible_zipped.into_iter() {
            // let last_move = node.move_to(&coor, &snake_id);
            let mut new_pending_moves = pending_moves.clone();
            new_pending_moves.push((*snake_id, dir));
            let next_move_return = self.minimax(
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
            moving_snake_id: *snake_id,
            score: chosen_score,
        }
    }

    fn time_limit_ms(&self) -> i64 {
        const NETWORK_LATENCY_PADDING: i64 = 130;
        self.game_info.timeout - NETWORK_LATENCY_PADDING
    }

    fn max_duration(&self) -> Duration {
        let seconds = self.time_limit_ms() / 1000;
        let millis = self.time_limit_ms() % 1000;
        Duration::new(
            seconds.try_into().unwrap(),
            (millis * 1_000_000).try_into().unwrap(),
        )
    }

    fn deepened_minimax(self, players: Vec<T::SnakeIDType>) -> MinMaxReturn<T, ScoreType> {
        let inner_span = info_span!("deepened_minmax_inner");
        let node = self.game;
        let you_id = node.you_id();

        const RUNAWAY_DEPTH_LIMIT: usize = 100;

        let started_at = Instant::now();
        let max_duration = self.max_duration();

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut current_depth = players.len();
            let mut current_return = None;

            loop {
                let next = info_span!(parent: &inner_span, "single_depth", depth = current_depth)
                    .in_scope(|| {
                        self.minimax(
                            node,
                            &players,
                            0,
                            WrappedScore::<ScoreType>::worst_possible_score(),
                            WrappedScore::<ScoreType>::best_possible_score(),
                            current_depth,
                            current_return,
                            vec![],
                        )
                    });

                if tx.send((current_depth, next.clone())).is_err() {
                    return;
                }

                current_return = Some(next);

                current_depth += players.len();
            }
        });

        let mut current = None;

        while started_at.elapsed() < max_duration {
            if let Ok((depth, result)) = rx.try_recv() {
                let current_score = result.score();
                let terminal_depth = current_score.terminal_depth();
                info!(depth, current_score = ?&current_score, current_direction = ?result.direction_for(you_id), elapsed_ms = ?started_at.elapsed().as_millis(), "Just finished depth");

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
            info!(depth, score = ?result.score(), direction = ?result.direction_for(you_id), all_moves = ?result.all_moves(), elapsed_ms = ?started_at.elapsed().as_millis(), "Finished deepened_minimax");
        }

        current
            .map(|(_depth, result)| result)
            .expect("We weren't able to do even a single layer of minmax")
    }

    /// Benchmark entry point for running a single minimax to the given number of turns
    pub fn single_minimax_bench(&self, max_turns: usize) -> MinMaxReturn<T, ScoreType> {
        let my_id = self.game.you_id();
        let mut sorted_ids = self.game.get_snake_ids();
        sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

        self.minimax(
            self.game,
            &sorted_ids,
            0,
            WrappedScore::<ScoreType>::worst_possible_score(),
            WrappedScore::<ScoreType>::best_possible_score(),
            max_turns * sorted_ids.len(),
            None,
            vec![],
        )
    }

    /// Used to benchmark a deepened minimax. In 'real' play a deepened_minmax is run with a
    /// timeout, but to work better with benchmarking we run this for a certain number of turns.
    pub fn deepend_minimax_bench(&self, max_turns: usize) -> MinMaxReturn<T, ScoreType> {
        let my_id = self.game.you_id();
        let mut sorted_ids = self.game.get_snake_ids();
        sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

        let players = sorted_ids;

        let max_depth = max_turns * players.len();
        let mut current_depth = players.len();
        let mut current_return = None;
        while current_depth <= max_depth {
            current_return = Some(self.minimax(
                self.game,
                &players,
                0,
                WrappedScore::<ScoreType>::worst_possible_score(),
                WrappedScore::<ScoreType>::best_possible_score(),
                current_depth,
                current_return,
                vec![],
            ));

            current_depth += players.len();
        }

        current_return.unwrap()
    }
}

#[cfg(test)]
mod tests {}
