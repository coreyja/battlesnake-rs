use crate::devious_devin_mutable::Instruments;
use crate::*;

use battlesnake_game_types::types::*;
use battlesnake_game_types::wire_representation::NestedGame;

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, info_span, Instrument};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Copy)]
pub enum WrappedScore<ScoreType>
where
    ScoreType: PartialOrd + Ord + Debug + Clone + Copy,
{
    Lose(i64),
    Tie(i64),
    Scored(ScoreType),
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

#[derive(Clone)]
pub struct EvalMinimaxSnake<T: 'static, ScoreType: 'static> {
    game: T,
    game_info: NestedGame,
    turn: i32,
    score_function: &'static (dyn Fn(&T) -> ScoreType + Sync + Send),
    name: &'static str,
}

#[derive(Debug, Clone)]
pub enum MinMaxReturn<
    T: SnakeIDGettableGame + Clone + Debug,
    ScoreType: Clone + Debug + PartialOrd + Ord + Copy,
> {
    Node {
        is_maximizing: bool,
        options: Vec<(Move, Self)>,
        moving_snake_id: T::SnakeIDType,
        score: WrappedScore<ScoreType>,
    },
    Leaf {
        score: WrappedScore<ScoreType>,
    },
}

impl<T, ScoreType> MinMaxReturn<T, ScoreType>
where
    T: SnakeIDGettableGame + Debug + Clone,
    ScoreType: Clone + Debug + PartialOrd + Ord + Copy,
{
    pub fn score(&self) -> &WrappedScore<ScoreType> {
        match self {
            MinMaxReturn::Node { score, .. } => score,
            MinMaxReturn::Leaf { score } => score,
        }
    }

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

    pub fn to_text_tree(&self) -> Option<String> {
        let tree_node = self.to_text_tree_node("".to_owned())?;
        Some(format!("{}", tree_node))
    }
}

use text_trees::{FormatCharacters, StringTreeNode, TreeFormatting, TreeNode};

impl<T, ScoreType> BattlesnakeAI for EvalMinimaxSnake<T, ScoreType>
where
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HeadGettableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + NeighborDeterminableGame
        + SimulableGame<Instruments>
        + Clone
        + Sync
        + Copy
        + FoodGettableGame
        + Send,
    T::SnakeIDType: Copy + Send + Sync,
    ScoreType: Clone + Debug + PartialOrd + Ord + Send + Sync + Copy,
{
    fn make_move(&self) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let my_id = self.game.you_id().clone();
        let mut sorted_ids = self.game.get_snake_ids();
        sorted_ids.sort_by_key(|snake_id| if snake_id == &my_id { -1 } else { 1 });

        let copy = self.clone();

        let best_option =
            info_span!("deepened_minmax", name = self.name, game_id = %&self.game_info.id, turn = self.turn, ruleset_name = %self.game_info.ruleset.name, ruleset_version = %self.game_info.ruleset.version).in_scope(|| copy.deepened_minimax(sorted_ids));

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

impl<T, ScoreType> EvalMinimaxSnake<T, ScoreType>
where
    T: SnakeIDGettableGame
        + YouDeterminableGame
        + PositionGettableGame
        + HealthGettableGame
        + VictorDeterminableGame
        + HeadGettableGame
        + NeighborDeterminableGame
        + SimulableGame<Instruments>
        + Clone
        + Copy
        + Sync
        + Send
        + Sized,
    T::SnakeIDType: Copy + Send + Sync,
    ScoreType: Clone + Debug + PartialOrd + Ord + Send + Sync + Copy,
{
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

        if pending_moves.len() == node.get_snake_ids().len() {
            node = node.simulate_with_moves(
                &Instruments,
                pending_moves
                    .into_iter()
                    .map(|(sid, m)| (sid, vec![m]))
                    .collect(),
            )[0]
            .1;
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

        if !node.is_alive(snake_id) {
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
        }

        let mut options: Vec<(Move, MinMaxReturn<T, ScoreType>)> = vec![];

        let is_maximizing = snake_id == node.you_id();

        let possible_moves = node.possible_moves(&node.get_head_as_native_position(&snake_id));

        let possible_zipped: Vec<(
            (Move, T::NativePositionType),
            Option<MinMaxReturn<T, ScoreType>>,
        )> = if let Some(MinMaxReturn::Node { mut options, .. }) = previous_return {
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
            moving_snake_id: snake_id.clone(),
            score: chosen_score,
        }
    }

    fn time_limit_ms(&self) -> i64 {
        const NETWORK_LATENCY_PADDING: i64 = 100;
        self.game_info.timeout - NETWORK_LATENCY_PADDING
    }

    fn max_duration(&self) -> Duration {
        Duration::new(0, (self.time_limit_ms() * 1_000_000).try_into().unwrap())
    }

    fn deepened_minimax(self, players: Vec<T::SnakeIDType>) -> MinMaxReturn<T, ScoreType> {
        let node = self.game.clone();
        let you_id = node.you_id();

        const RUNAWAY_DEPTH_LIMIT: usize = 100;

        let started_at = Instant::now();
        let max_duration = self.max_duration();

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut current_depth = players.len();
            let mut current_return = None;
            loop {
                let next = self.minimax(
                    node,
                    &players,
                    0,
                    WrappedScore::<ScoreType>::worst_possible_score(),
                    WrappedScore::<ScoreType>::best_possible_score(),
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
