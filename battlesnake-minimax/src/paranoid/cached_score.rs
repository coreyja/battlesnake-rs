use dashmap::DashMap;
use fxhash::FxBuildHasher;

use super::Scorable;

use std::{hash::Hash, sync::Arc};

#[derive(Debug, Clone)]
/// Cache the score
pub struct CachedScore<ScorableType, GameType, ScoreType>
where
    ScorableType: Scorable<GameType, ScoreType>,
    GameType: Eq + Hash + Copy,
{
    scorable: ScorableType,
    cache: Arc<DashMap<GameType, ScoreType, FxBuildHasher>>,
    _phantom: std::marker::PhantomData<(ScoreType, GameType)>,
}

impl<ScorableType, GameType, ScoreType> CachedScore<ScorableType, GameType, ScoreType>
where
    ScorableType: Scorable<GameType, ScoreType>,
    GameType: Eq + Hash + Copy,
{
    /// Wrap the given scorable with a cache. We pass in a reference to the cache so that we can
    /// create multiple wrappers with a shared cache
    pub fn new(
        scorable: ScorableType,
        cache: Arc<DashMap<GameType, ScoreType, FxBuildHasher>>,
    ) -> Self {
        Self {
            scorable,
            cache,
            _phantom: Default::default(),
        }
    }
}

impl<InnerScorableType, GameType, ScoreType> Scorable<GameType, ScoreType>
    for CachedScore<InnerScorableType, GameType, ScoreType>
where
    InnerScorableType: Scorable<GameType, ScoreType>,
    GameType: Eq + Hash + Copy,
    ScoreType: Copy,
{
    fn score(&self, game: &GameType) -> ScoreType {
        *self
            .cache
            .entry(*game)
            .or_insert_with(|| self.scorable.score(game))
    }
}
