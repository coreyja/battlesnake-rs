use std::fmt::Debug;

use battlesnake_game_types::types::{VictorDeterminableGame, YouDeterminableGame};

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

/// Provides an implementation for `wrapped_score` if the implementer implements the `score`
/// function.
///
/// `wrapped_score` takes into account if the node is an end_state, and depth based ordering so
/// that the underlying scoring functions don't need to worry about this
pub trait Scorable<GameType, ScoreType>
where
    ScoreType: PartialOrd + Ord + Copy + Debug,
    GameType: YouDeterminableGame + VictorDeterminableGame,
{
    /// This is the the scoring function for your Minimax snake
    ///
    /// The score for all non end state nodes will be defined by this score
    fn score(&self, node: &GameType) -> ScoreType;

    /// `wrapped_score` takes into account the depth and number of players. It checks the game
    /// board and decides if this is a leaf in our Minimax tree. If it IS a leaf we score it based
    /// on the outcome of the game board. If we've hit the maximum depth, we use the scoring
    /// function provided by `score`
    fn wrapped_score(
        &self,
        node: &GameType,
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
            return Some(WrappedScore::Scored(self.score(node)));
        }

        None
    }
}
