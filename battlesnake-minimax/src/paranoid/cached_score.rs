use dashmap::DashMap;

use super::Scorable;

use std::hash::Hash;

#[derive(Debug, Clone)]
/// Cache the score
pub struct CachedScore<'cache, ScorableType>
where
    ScorableType: Scorable + Clone,
    ScorableType::GameType: Eq + Hash + Copy,
{
    scorable: ScorableType,
    cache: &'cache DashMap<ScorableType::GameType, ScorableType::ScoreType>,
}

impl<'cache, ScorableType> CachedScore<'cache, ScorableType>
where
    ScorableType: Scorable + Clone,
    ScorableType::GameType: Eq + Hash + Copy,
{
    /// TODO
    pub fn new(
        scorable: ScorableType,
        cache: &'cache DashMap<ScorableType::GameType, ScorableType::ScoreType>,
    ) -> Self {
        Self { scorable, cache }
    }
}

impl<'cache, ScorableType> Scorable for CachedScore<'cache, ScorableType>
where
    ScorableType: Scorable + Clone,
    ScorableType::GameType: Eq + Hash + Copy,
    ScorableType::ScoreType: PartialOrd + Ord + Clone + Copy,
{
    type ScoreType = ScorableType::ScoreType;
    type GameType = ScorableType::GameType;

    fn score(&self, game: &Self::GameType) -> Self::ScoreType {
        *self
            .cache
            .entry(*game)
            .or_insert_with(|| self.scorable.score(game))
    }
}
