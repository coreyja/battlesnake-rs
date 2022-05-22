use std::fmt::Debug;

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
    /// Returns the best possible score
    ///
    /// This is a Win with the depth set as the maximum i64 such that no WrappedScore can be higher
    /// than this given the Ord
    pub fn best_possible_score() -> Self {
        WrappedScore::Win(std::i64::MAX)
    }

    /// Returns the worst possible score
    ///
    /// This is a Lost with the depth set as the minimum i64 such that no WrappedScore can be higher
    /// than this given the Ord
    pub fn worst_possible_score() -> Self {
        WrappedScore::Lose(std::i64::MIN)
    }

    /// Returns the depth from this score IFF the score is a terminal node. Otherwise returns None
    pub fn terminal_depth(&self) -> Option<i64> {
        match &self {
            Self::Win(d) => Some(-d),
            Self::Tie(d) | Self::Lose(d) => Some(*d),
            _ => None,
        }
    }
}
