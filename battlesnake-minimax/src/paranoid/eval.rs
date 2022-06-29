use std::{
    borrow::Cow,
    fmt::Debug,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use battlesnake_game_types::{
    types::{
        HeadGettableGame, HealthGettableGame, Move, NeckQueryableGame, NeighborDeterminableGame,
        PositionGettableGame, SimulableGame, SimulatorInstruments, SnakeIDGettableGame,
        VictorDeterminableGame, YouDeterminableGame,
    },
    wire_representation::NestedGame,
};
use derivative::Derivative;
use itertools::Itertools;
use tracing::{info, info_span, warn};

use crate::Instruments;

use super::{MinMaxReturn, Scorable, WrappedScore};

#[derive(Derivative, Clone)]
#[derivative(Debug)]
/// This is the struct that wraps a game board and a scoring function and can be used to run
/// minimax
///
/// It also outputs traces using the [tracing] crate.
pub struct MinimaxSnake<T: 'static, ScoreType: 'static, const N_SNAKES: usize> {
    game: T,
    game_info: NestedGame,
    turn: i32,
    #[derivative(Debug = "ignore")]
    score_function: &'static (dyn Fn(&T) -> ScoreType + Sync + Send),
    name: &'static str,
    options: SnakeOptions,
}

#[derive(Debug, Clone, Copy)]
/// Optional properties that can be defined for an [MinimaxSnake]
///
/// The defaults (as implemented by [Default]) are as follows:
/// ```
/// use std::time::Duration;
/// use battlesnake_minimax::paranoid::SnakeOptions;
///
/// let defaults: SnakeOptions = Default::default();
///
/// assert_eq!(defaults.network_latency_padding, Duration::from_millis(100));
/// ```
pub struct SnakeOptions {
    /// How long should we 'reserve' for Network Latency
    ///
    /// This is used in conjunction with the timeout for the game to determine how much time we can
    /// spend calculating the next move in our Deepened Minimax
    ///
    /// Defaults to 100 milliseconds
    pub network_latency_padding: Duration,
}

impl Default for SnakeOptions {
    fn default() -> Self {
        Self {
            network_latency_padding: Duration::from_millis(100),
        }
    }
}

#[derive(Debug, Copy, Clone)]
/// This type is used to represent that the main thread
/// told the worker thread to stop running so we returned
/// out of the current context
pub struct AbortedEarly;

impl<GameType, ScoreType, const N_SNAKES: usize> Scorable<GameType, ScoreType>
    for MinimaxSnake<GameType, ScoreType, N_SNAKES>
where
    ScoreType: Debug + PartialOrd + Ord + Copy,
    GameType: YouDeterminableGame + VictorDeterminableGame,
{
    fn score(&self, node: &GameType) -> ScoreType {
        (self.score_function)(node)
    }
}

impl SimulatorInstruments for Instruments {
    fn observe_simulation(&self, _duration: Duration) {}
}

impl<T, ScoreType, const N_SNAKES: usize> MinimaxSnake<T, ScoreType, N_SNAKES>
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
        + Sync
        + Send
        + Sized,
    T::SnakeIDType: Clone + Send + Sync,
    ScoreType: Clone + Debug + PartialOrd + Ord + Send + Sync + Copy,
{
    /// Construct a new `MinimaxSnake`
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use battlesnake_minimax::paranoid::{MinMaxReturn, MinimaxSnake, SnakeOptions};
    /// use battlesnake_game_types::{types::build_snake_id_map, compact_representation::StandardCellBoard4Snakes11x11, wire_representation::Game};
    ///
    /// // This fixture data matches what we expect to come from the Battlesnake Game Server
    /// let game_state_from_server = include_str!("../../../battlesnake-rs/fixtures/start_of_game.json");
    ///
    /// // First we take the JSON from the game server and construct a `Game` struct which
    /// // represents the 'wire' representation of the game state
    /// let wire_game: Game = serde_json::from_str(game_state_from_server).unwrap();
    ///
    /// // The 'compact' representation of the game state doesn't include the game_info but we use
    /// // it for some of our tracing so we want to clone it before we create the compact representation
    /// let game_info = wire_game.game.clone();
    ///
    /// let snake_id_map = build_snake_id_map(&wire_game);
    /// let compact_game = StandardCellBoard4Snakes11x11::convert_from_game(wire_game, &snake_id_map).unwrap();
    ///
    /// // This is the scoring function that we will use to evaluate the game states
    /// // Here it just returns a constant but would ideally contain some logic to decide which
    /// // states are better than others
    /// fn score_function(board: &StandardCellBoard4Snakes11x11) -> i32 { 4 }
    ///
    /// let minimax_snake = MinimaxSnake::new(
    ///    compact_game,
    ///    game_info,
    ///    0,
    ///    &score_function,
    ///    "minimax_snake",
    /// );
    /// ```
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
            options: Default::default(),
        }
    }

    /// Construct a new `MinimaxSnake` providing an optional set of [SnakeOptions]
    ///
    /// [SnakeOptions] implements [Default] so you can override specific options and rely on
    /// defaults for the rest.
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use battlesnake_minimax::paranoid::{MinMaxReturn, MinimaxSnake, SnakeOptions};
    /// use battlesnake_game_types::{types::build_snake_id_map, compact_representation::StandardCellBoard4Snakes11x11, wire_representation::Game};
    ///
    /// // This fixture data matches what we expect to come from the Battlesnake Game Server
    /// let game_state_from_server = include_str!("../../../battlesnake-rs/fixtures/start_of_game.json");
    ///
    /// // First we take the JSON from the game server and construct a `Game` struct which
    /// // represents the 'wire' representation of the game state
    /// let wire_game: Game = serde_json::from_str(game_state_from_server).unwrap();
    ///
    /// // The 'compact' representation of the game state doesn't include the game_info but we use
    /// // it for some of our tracing so we want to clone it before we create the compact representation
    /// let game_info = wire_game.game.clone();
    ///
    /// let snake_id_map = build_snake_id_map(&wire_game);
    /// let compact_game = StandardCellBoard4Snakes11x11::convert_from_game(wire_game, &snake_id_map).unwrap();
    ///
    /// // This is the scoring function that we will use to evaluate the game states
    /// // Here it just returns a constant but would ideally contain some logic to decide which
    /// // states are better than others
    /// fn score_function(board: &StandardCellBoard4Snakes11x11) -> i32 { 4 }
    ///
    ///
    /// // Optional settings for the snake
    /// let snake_options = SnakeOptions {
    ///   network_latency_padding: Duration::from_millis(100),
    ///   ..Default::default()
    /// };
    ///
    /// let minimax_snake = MinimaxSnake::new_with_options(
    ///    compact_game,
    ///    game_info,
    ///    0,
    ///    &score_function,
    ///    "minimax_snake",
    ///    snake_options,
    /// );
    /// ```
    pub fn new_with_options(
        game: T,
        game_info: NestedGame,
        turn: i32,
        score_function: &'static (dyn Fn(&T) -> ScoreType + Sync + Send),
        name: &'static str,
        options: SnakeOptions,
    ) -> Self {
        Self {
            game,
            game_info,
            turn,
            score_function,
            name,
            options,
        }
    }

    /// Pick the next move to make
    ///
    /// This uses [MinimaxSnake::deepened_minimax_until_timelimit()] to run the Minimax algorihm until we run out of time, and
    /// return the chosen move. For more information on the inner working see the docs for
    /// [MinimaxSnake::deepened_minimax_until_timelimit()]
    pub fn make_move(&self) -> Move {
        let my_id = self.game.you_id();
        let mut sorted_ids = self.game.get_snake_ids();
        sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

        let copy = self.clone();

        let (scored, explored) = info_span!(
          "deepened_minmax_with_exploration",
          snake_name = self.name,
          game_id = %&self.game_info.id,
          turn = self.turn,
          ruleset_name = %self.game_info.ruleset.name,
          ruleset_version = %self.game_info.ruleset.version,
        )
        .in_scope(|| copy.deepened_minimax_until_timelimit_with_exploration_thread(sorted_ids));

        match (scored, explored) {
            (Some((scored_depth, scored_return)), Some((explored_depth, explored_return))) => {
                info!(scored_depth, explored_depth, "finished make_move",);

                let scored_options = scored_return.first_options_for_snake(my_id).unwrap();

                // True in this map means the move is safe
                let move_safety_map =
                    Move::all().map(|m| !explored_return.your_move_is_death(my_id, m));

                if move_safety_map.iter().all(|safe| !safe) {
                    warn!("No safe moves found, choosing the best move anyways");

                    return scored_options.first().unwrap().0;
                }

                for (m, _) in scored_options {
                    if move_safety_map[m.as_index()] {
                        return *m;
                    }
                }

                warn!("There are safe moves, but none of the scores moves matched any safe moves. This is weird and is probably a bug. We are going right");
                Move::Right
            }
            (None, Some((explored_depth, explored_return))) => {
                info!(scored_depth = 0, explored_depth, "finished make_move",);

                let explored_options = explored_return.first_options_for_snake(my_id).unwrap();
                explored_options.first().unwrap().0
            }
            (Some((scored_depth, scored_return)), None) => {
                info!(scored_depth, explored_depth = 0, "finished make_move",);

                let scored_options = scored_return.first_options_for_snake(my_id).unwrap();
                scored_options.first().unwrap().0
            }
            (None, None) => {
                warn!(
                    scored_depth = 0,
                    explored_depth = 0,
                    "We couldn't get a single depth for either the scoring or exploring thread",
                );

                Move::Right
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn minimax(
        &self,
        node: Cow<T>,
        players: &[T::SnakeIDType],
        depth: usize,
        alpha: WrappedScore<ScoreType>,
        beta: WrappedScore<ScoreType>,
        max_depth: usize,
        previous_return: Option<MinMaxReturn<T, ScoreType>>,
        mut pending_moves: Vec<(T::SnakeIDType, Move)>,
        worker_halt_reciever: Option<&mpsc::Receiver<()>>,
    ) -> Result<MinMaxReturn<T, ScoreType>, AbortedEarly> {
        let mut alpha = alpha;
        let mut beta = beta;

        let snake_ids = node.get_snake_ids();

        // Remove pending moves for dead snakes
        pending_moves.retain(|(snake_id, _)| snake_ids.contains(snake_id));

        let node = if !snake_ids.is_empty() && pending_moves.len() == snake_ids.len() {
            let mut simulate_result = node.simulate_with_moves(
                &Instruments {},
                pending_moves
                    .into_iter()
                    .map(|(sid, m)| (sid, vec![m]))
                    .collect_vec(),
            );
            let new_node = simulate_result.next().unwrap().1;
            drop(simulate_result);
            pending_moves = vec![];

            Cow::Owned(new_node)
        } else {
            node
        };

        let new_depth = depth.try_into().unwrap();
        if let Some(s) = self.wrapped_score(
            &node,
            new_depth,
            max_depth.try_into().unwrap(),
            players.len() as i64,
        ) {
            return Ok(MinMaxReturn::Leaf { score: s });
        }

        let snake_id = &players[depth % players.len()];

        let mut options: Vec<(Move, MinMaxReturn<T, ScoreType>)> = vec![];

        let is_maximizing = snake_id == node.you_id();

        if node.get_health_i64(snake_id) == 0 {
            return self.minimax(
                node,
                players,
                depth + 1,
                alpha,
                beta,
                max_depth,
                previous_return,
                pending_moves,
                worker_halt_reciever,
            );
        }

        assert!(node.get_health_i64(snake_id) > 0);
        let possible_moves = node
            .possible_moves(&node.get_head_as_native_position(snake_id))
            .filter(|(_, pos)| !node.is_neck(snake_id, pos));

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
            if let Some(worker_halt_reciever) = worker_halt_reciever {
                if worker_halt_reciever.try_recv().is_ok() {
                    return Err(AbortedEarly);
                }
            }

            // let last_move = node.move_to(&coor, &snake_id);
            let mut new_pending_moves = pending_moves.clone();
            new_pending_moves.push((snake_id.clone(), dir));
            let next_move_return = self.minimax(
                node.clone(),
                players,
                depth + 1,
                alpha,
                beta,
                max_depth,
                previous_return,
                new_pending_moves,
                worker_halt_reciever,
            )?;
            let value = *next_move_return.score();
            // node.reverse_move(last_move);
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

        Ok(MinMaxReturn::Node {
            options,
            is_maximizing,
            moving_snake_id: snake_id.clone(),
            score: chosen_score,
        })
    }

    fn max_duration(&self) -> Duration {
        let timeout = self
          .game_info
          .timeout
          .try_into()
          .expect("We are dealing with things on the order of hundreds of millis or a couple seconds. We shouldn't have a padding that can't convert from an i64 to a u64");
        let timeout = Duration::from_millis(timeout);

        timeout - self.options.network_latency_padding
    }

    /// This will do a iterative deepening minimax until we reach the time limit [with some padding
    /// for network latency]. Iterative deepening means it will first start by evaluating minimax
    /// at a turn count of 1. Then it moves on to a minimax for turn 2, but evaluating the best
    /// move from the previous turn first. This allows the Alpha-Beta pruning to be more efficient
    /// for the second round. We keep repeating this process with deeper depths until we run out of time.
    ///
    /// The actual minimax algorithm is run in a separate thread so that we don't have issues with
    /// returning in time if we started a long minimax process that may not return in time.
    /// When we return from the main/timing thread we also send a signal the the 'worker' thread
    /// telling it to stop, so as not to waste CPU cycles
    pub fn deepened_minimax_until_timelimit(
        self,
        players: Vec<T::SnakeIDType>,
    ) -> MinMaxReturn<T, ScoreType> {
        let inner_span = info_span!(
            "deepened_minmax_inner",
            chosen_score = tracing::field::Empty,
            chosen_direction = tracing::field::Empty,
            all_moves = tracing::field::Empty,
            depth = tracing::field::Empty,
        );
        let max_duration = self.max_duration();
        let node = &self.game;

        let started_at = Instant::now();
        let you_id = node.you_id().clone();
        let threads_you_id = you_id.clone();

        let (to_main_thread, from_worker_thread) = mpsc::channel();
        let (suspend_worker, worker_halt_reciever) = mpsc::channel();

        let cloned_inner = inner_span.clone();
        thread::spawn(move || {
            let you_id = threads_you_id;
            let mut current_depth = players.len();
            let mut current_return = None;

            loop {
                let next = info_span!(
                    parent: &cloned_inner,
                    "single_depth",
                    depth = current_depth,
                    score = tracing::field::Empty,
                    direction = tracing::field::Empty
                )
                .in_scope(|| {
                    let result = self.minimax(
                        Cow::Borrowed(&self.game),
                        &players,
                        0,
                        WrappedScore::<ScoreType>::worst_possible_score(),
                        WrappedScore::<ScoreType>::best_possible_score(),
                        current_depth,
                        current_return,
                        vec![],
                        Some(&worker_halt_reciever),
                    );

                    if let Ok(ref result) = result {
                        let current_span = tracing::Span::current();
                        current_span.record("score", &format!("{:?}", result.score()).as_str());
                        current_span.record(
                            "direction",
                            &format!("{:?}", result.your_best_move(&you_id)).as_str(),
                        );
                    }

                    result
                });

                let next = match next {
                    Ok(x) => x,
                    Err(AbortedEarly) => break,
                };

                let current_score = next.score();
                let terminal_depth = current_score.terminal_depth();

                let action = match terminal_depth {
                    Some(terminal_depth) => {
                        if current_depth >= terminal_depth.try_into().unwrap() {
                            FromWorkerAction::Stop
                        } else {
                            FromWorkerAction::KeepGoing
                        }
                    }
                    None => FromWorkerAction::KeepGoing,
                };

                let send_result = to_main_thread.send((action, current_depth, next.clone()));

                if send_result.is_err() || matches!(action, FromWorkerAction::Stop) {
                    return;
                }

                current_return = Some(next);

                current_depth += players.len();
            }
        });

        let mut current = None;

        while started_at.elapsed() < max_duration {
            if let Ok((action, depth, result)) = from_worker_thread.try_recv() {
                // println!("{}", self.game.evaluate_moves(&result.all_moves()));
                current = Some((depth, result));

                match action {
                    FromWorkerAction::KeepGoing => {}
                    FromWorkerAction::Stop => {
                        info!(depth, "This game is over, no need to keep going");
                        break;
                    }
                }
            }
        }

        // We send a signal to the worker thread to stop
        // We can't kill the thread so we use this to help the
        // worker know when to stop
        let _ = suspend_worker.send(());

        if let Some((depth, result)) = &current {
            inner_span.record("chosen_score", &format!("{:?}", result.score()).as_str());
            inner_span.record(
                "chosen_direction",
                &format!("{:?}", result.your_best_move(&you_id)).as_str(),
            );
            inner_span.record(
                "all_moves",
                &format!("{:?}", result.chosen_route()).as_str(),
            );
            inner_span.record("depth", &depth);
        }

        current
            .map(|(_depth, result)| result)
            .expect("We weren't able to do even a single layer of minmax")
    }

    /// This differs from the `deepened_minimax_until_timelimit` in that not only do we start a
    /// thread for the scored minimax, but we also start one with an empty scoring function to
    /// serve as an exploration thread. Ideally this thread will be able to get to a deeper depth
    /// than the scoring thread
    pub fn deepened_minimax_until_timelimit_with_exploration_thread(
        self,
        players: Vec<T::SnakeIDType>,
    ) -> (
        Option<(usize, MinMaxReturn<T, ScoreType>)>,
        Option<(usize, MinMaxReturn<T, ()>)>,
    ) {
        let inner_span = info_span!(
            "deepened_minmax_inner",
            chosen_score = tracing::field::Empty,
            chosen_direction = tracing::field::Empty,
            all_moves = tracing::field::Empty,
            depth = tracing::field::Empty,
        );
        let max_duration = self.max_duration();
        let node = &self.game;

        let started_at = Instant::now();
        let you_id = node.you_id().clone();

        let explorer_game = self.game.clone();
        let explorer_game_info = self.game_info.clone();
        let explorer_players = players.clone();
        let turn = self.turn;

        // Setup and spawn the scoring thread
        // This one uses the 'real' scoring function
        // to pick the best move.
        let (from_scored_thread, suspend_scorer) = {
            let (to_manager_thread, from_scored_thread) = mpsc::channel();
            let (suspend_scorer, scorer_halt_reciever) = mpsc::channel();

            let scorer_cloned_inner = inner_span.clone();
            let threads_you_id = you_id.clone();
            thread::spawn(move || {
                let you_id = threads_you_id;
                let mut current_depth = players.len();
                let mut current_return = None;

                loop {
                    let next = info_span!(
                        parent: &scorer_cloned_inner,
                        "scored_single_depth",
                        depth = current_depth,
                        score = tracing::field::Empty,
                        direction = tracing::field::Empty
                    )
                    .in_scope(|| {
                        let result = self.minimax(
                            Cow::Borrowed(&self.game),
                            &players,
                            0,
                            WrappedScore::<ScoreType>::worst_possible_score(),
                            WrappedScore::<ScoreType>::best_possible_score(),
                            current_depth,
                            current_return,
                            vec![],
                            Some(&scorer_halt_reciever),
                        );

                        if let Ok(ref result) = result {
                            let current_span = tracing::Span::current();
                            current_span.record("score", &format!("{:?}", result.score()).as_str());
                            current_span.record(
                                "direction",
                                &format!("{:?}", result.your_best_move(&you_id)).as_str(),
                            );
                        }

                        result
                    });

                    let next = match next {
                        Ok(x) => x,
                        Err(AbortedEarly) => break,
                    };

                    let current_score = next.score();
                    let terminal_depth = current_score.terminal_depth();

                    let action = match terminal_depth {
                        Some(terminal_depth) => {
                            if current_depth >= terminal_depth.try_into().unwrap() {
                                FromWorkerAction::Stop
                            } else {
                                FromWorkerAction::KeepGoing
                            }
                        }
                        None => FromWorkerAction::KeepGoing,
                    };

                    let send_result = to_manager_thread.send((action, current_depth, next.clone()));

                    if send_result.is_err() || matches!(action, FromWorkerAction::Stop) {
                        return;
                    }

                    current_return = Some(next);

                    current_depth += players.len();
                }
            });

            (from_scored_thread, suspend_scorer)
        };

        // Setup and spawn the explorer thread
        // This uses an empty scoring function to try and find and avoid traps
        let (from_explorer_thread, suspend_explorer) = {
            let (to_manager_thread, from_explorer_thread) = mpsc::channel();
            let (suspend_explorer, explorer_halt_reciever) = mpsc::channel();

            let explorer_snake = MinimaxSnake::new(
                explorer_game.clone(),
                explorer_game_info,
                turn,
                &|_| {},
                "explorer",
            );

            let scorer_cloned_inner = inner_span.clone();
            thread::spawn(move || {
                let mut current_depth = explorer_players.len();
                let mut current_return = None;
                let explorer_game = explorer_game;
                let you_id = explorer_game.you_id();

                loop {
                    let next = info_span!(
                        parent: &scorer_cloned_inner,
                        "explored_single_depth",
                        depth = current_depth,
                        score = tracing::field::Empty,
                        direction = tracing::field::Empty
                    )
                    .in_scope(|| {
                        let result = explorer_snake.minimax(
                            Cow::Borrowed(&explorer_game),
                            &explorer_players,
                            0,
                            WrappedScore::<()>::worst_possible_score(),
                            WrappedScore::<()>::best_possible_score(),
                            current_depth,
                            current_return,
                            vec![],
                            Some(&explorer_halt_reciever),
                        );

                        if let Ok(ref result) = result {
                            let current_span = tracing::Span::current();
                            current_span.record("score", &format!("{:?}", result.score()).as_str());
                            current_span.record(
                                "direction",
                                &format!("{:?}", result.your_best_move(you_id)).as_str(),
                            );
                        }

                        result
                    });

                    let next = match next {
                        Ok(x) => x,
                        Err(AbortedEarly) => break,
                    };

                    let current_score = next.score();
                    let terminal_depth = current_score.terminal_depth();

                    let action = match terminal_depth {
                        Some(terminal_depth) => {
                            if current_depth >= terminal_depth.try_into().unwrap() {
                                FromWorkerAction::Stop
                            } else {
                                FromWorkerAction::KeepGoing
                            }
                        }
                        None => FromWorkerAction::KeepGoing,
                    };

                    let send_result = to_manager_thread.send((action, current_depth, next.clone()));

                    if send_result.is_err() || matches!(action, FromWorkerAction::Stop) {
                        return;
                    }

                    current_return = Some(next);

                    current_depth += explorer_players.len();
                }
            });

            (from_explorer_thread, suspend_explorer)
        };

        let mut current = None;
        let mut current_explorer = None;

        while started_at.elapsed() < max_duration {
            if let Ok((action, depth, result)) = from_scored_thread.try_recv() {
                // println!("{}", self.game.evaluate_moves(&result.all_moves()));
                current = Some((depth, result));

                match action {
                    FromWorkerAction::KeepGoing => {}
                    FromWorkerAction::Stop => {
                        info!(depth, "This game is over, no need to keep going");
                        break;
                    }
                }
            }

            if let Ok((action, depth, result)) = from_explorer_thread.try_recv() {
                // println!("{}", self.game.evaluate_moves(&result.all_moves()));
                current_explorer = Some((depth, result));

                if let FromWorkerAction::Stop = action {
                    info!(depth, "This game is over, no need to keep going");
                    break;
                }
            }
        }

        // We send a signal to the worker threads to stop
        // We can't kill the thread so we use this to help the
        // worker know when to stop
        let _ = suspend_scorer.send(());
        let _ = suspend_explorer.send(());

        if let Some((depth, result)) = &current {
            inner_span.record("chosen_score", &format!("{:?}", result.score()).as_str());
            inner_span.record(
                "chosen_direction",
                &format!("{:?}", result.your_best_move(&you_id)).as_str(),
            );
            inner_span.record(
                "all_moves",
                &format!("{:?}", result.chosen_route()).as_str(),
            );
            inner_span.record("depth", &depth);
        }

        (current, current_explorer)
    }

    /// This runs the minimax algorithm to the specified number of turns, returning an struct that
    /// contains all the information about the 'tree' we searched.
    ///
    /// The return value is a recursive struct that tells you the score of the current, and the
    /// score of all its children nodes.
    ///
    /// This can/is also be used as a benchmark entry point
    pub fn single_minimax(&self, max_turns: usize) -> MinMaxReturn<T, ScoreType> {
        let my_id = self.game.you_id();
        let mut sorted_ids = self.game.get_snake_ids();
        sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

        self.minimax(
            Cow::Borrowed(&self.game),
            &sorted_ids,
            0,
            WrappedScore::<ScoreType>::worst_possible_score(),
            WrappedScore::<ScoreType>::best_possible_score(),
            max_turns * sorted_ids.len(),
            None,
            vec![],
            None,
        )
        .unwrap()
    }

    /// This will do a iterative deepening minimax until the specified number of turns. This is
    /// currently used mostly for debugging and benchmarking
    ///
    /// Iterative deepening means it will first start by evaluating minimax
    /// at a turn count of 1. Then it moves on to a minimax for turn 2, but evaluating the best
    /// move from the previous turn first. This allows the Alpha-Beta pruning to be more efficient
    /// for the second round. We keep repeating this process with deeper depths until we hit the
    /// specified
    pub fn deepend_minimax_to_turn(&self, max_turns: usize) -> MinMaxReturn<T, ScoreType> {
        let my_id = self.game.you_id();
        let mut sorted_ids = self.game.get_snake_ids();
        sorted_ids.sort_by_key(|snake_id| if snake_id == my_id { -1 } else { 1 });

        let players = sorted_ids;

        let max_depth = max_turns * players.len();
        let mut current_depth = players.len();
        let mut current_return = None;
        while current_depth <= max_depth {
            current_return = Some(
                self.minimax(
                    Cow::Borrowed(&self.game),
                    &players,
                    0,
                    WrappedScore::<ScoreType>::worst_possible_score(),
                    WrappedScore::<ScoreType>::best_possible_score(),
                    current_depth,
                    current_return,
                    vec![],
                    None,
                )
                .unwrap(),
            );

            current_depth += players.len();
        }

        current_return.unwrap()
    }
}

#[derive(Debug, Copy, Clone)]
enum FromWorkerAction {
    KeepGoing,
    Stop,
}
