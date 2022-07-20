use std::{fmt::Debug, hash::Hash, sync::Arc, thread};

use dashmap::DashMap;
use derivative::Derivative;
use types::{types::*, wire_representation::NestedGame};

use crate::{
    paranoid::{move_ordering::MoveOrdering, CachedScore, Scorable, SnakeOptions},
    Instruments, ParanoidMinimaxSnake,
};

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[allow(missing_docs)]
pub struct LazySmpSnake<GameType, ScoreType, ScorableType, const N_SNAKES: usize>
where
    GameType: 'static + Hash + Eq + PartialEq + Copy + Sync + Send,
    ScoreType: 'static + Sync + Send + Clone,
    ScorableType: Scorable<GameType, ScoreType> + Sized + Send + Sync + 'static + Clone,
    CachedScore<ScorableType, GameType, ScoreType>: Scorable<GameType, ScoreType>,
{
    cache: Arc<DashMap<GameType, ScoreType>>,
    main_snake: ParanoidMinimaxSnake<
        GameType,
        ScoreType,
        CachedScore<ScorableType, GameType, ScoreType>,
        N_SNAKES,
    >,
    background_snake: ParanoidMinimaxSnake<
        GameType,
        ScoreType,
        CachedScore<ScorableType, GameType, ScoreType>,
        N_SNAKES,
    >,
}

impl<GameType, ScoreType, ScorableType, const N_SNAKES: usize>
    LazySmpSnake<GameType, ScoreType, ScorableType, N_SNAKES>
where
    GameType: SnakeIDGettableGame
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
        + Sized
        + Eq
        + PartialEq
        + Hash
        + Copy,
    GameType::SnakeIDType: Clone + Send + Sync,
    ScoreType: 'static + Copy + Send + Sync + Ord + PartialOrd + Debug,
    ScorableType: Scorable<GameType, ScoreType> + Sized + Send + Sync + 'static + Clone,
{
    #[allow(missing_docs)]
    pub fn new(
        game: GameType,
        game_info: NestedGame,
        turn: i32,
        score_function: ScorableType,
        name: &'static str,
        options: SnakeOptions,
    ) -> Self {
        let cache: DashMap<GameType, ScoreType> = DashMap::new();
        let cache = Arc::new(cache);
        let cached_score = CachedScore::new(score_function, cache.clone());

        let main_options = {
            let mut options = options;
            options.move_ordering = MoveOrdering::BestFirst;
            options
        };

        let main_snake = ParanoidMinimaxSnake::new(
            game,
            game_info.clone(),
            turn,
            cached_score.clone(),
            name,
            main_options,
        );

        let background_options = {
            let mut options = options;
            options.move_ordering = MoveOrdering::Random;
            options
        };

        let background_snake = ParanoidMinimaxSnake::new(
            game,
            game_info,
            turn,
            cached_score,
            name,
            background_options,
        );

        Self {
            cache,
            main_snake,
            background_snake,
        }
    }

    pub fn choose_move(&self) -> Move {
        let num_background_snakes: usize = std::thread::available_parallelism()
            .map(|x| x.into())
            .map(|x: usize| x - 1)
            .unwrap_or(1);

        for _ in 0..num_background_snakes {
            let snake = self.background_snake.clone();
            thread::spawn(move || {
                snake.choose_move();
            });
        }

        self.main_snake.choose_move()
    }
}
