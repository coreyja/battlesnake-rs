use std::{cmp::Reverse, fmt::Debug};

use battlesnake_game_types::types::{
    HealthGettableGame, VictorDeterminableGame, YouDeterminableGame,
};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Copy)]
/// The wrapped score type. This takes into account the score provided by the score function, but
/// wraps it with a Score based on the game state. This allows us to say that wins are better than
/// any score and loses are worse than any score, etc.
pub enum WrappedScore<ScoreType>
where
    ScoreType: PartialOrd + Ord + Debug + Clone + Copy,
{
    /// We lost
    /// The first value is the number of snakes left alive
    /// And then the depth
    /// Such that we prefer where less snales are alive, and deeper depths
    Lose(Reverse<u8>, i64),
    /// We tied
    /// The first value is the number of snakes left alive
    /// And then the depth
    /// Such that we prefer where less snales are alive, and deeper depths
    Tie(Reverse<u8>, i64),
    /// We order this based on the score provided by the score function
    Scored(ScoreType),
    /// We won, the depth is recorded because we prefer winning sooner
    Win(Reverse<i64>),
}

const LOWEST_DEPTH: i64 = i64::MIN;

impl<ScoreType> WrappedScore<ScoreType>
where
    ScoreType: PartialOrd + Ord + Debug + Clone + Copy,
{
    /// Returns the best possible score
    ///
    /// This is a Win with the depth set as the maximum i64 such that no WrappedScore can be higher
    /// than this given the Ord
    pub fn best_possible_score() -> Self {
        WrappedScore::Win(Reverse(LOWEST_DEPTH))
    }

    /// Returns the worst possible score
    ///
    /// This is a Lost with the depth set as the minimum i64 such that no WrappedScore can be higher
    /// than this given the Ord
    pub fn worst_possible_score() -> Self {
        WrappedScore::Lose(Reverse(u8::MAX), LOWEST_DEPTH)
    }

    /// Returns the depth from this score IFF the score is a terminal node. Otherwise returns None
    pub fn terminal_depth(&self) -> Option<i64> {
        match &self {
            Self::Win(Reverse(d)) => Some(*d),
            Self::Tie(_, d) | Self::Lose(_, d) => Some(*d),
            _ => None,
        }
    }
}

/// This trait is used to control something that can return a score from a game board
///
/// We use this trait to be able to layer in different scoring approaches, such as caching
pub trait Scorable<GameType, ScoreType> {
    /// Convert the given GameType into a ScoreType
    fn score(&self, game: &GameType) -> ScoreType;
}

impl<GameType, ScoreType, FnLike: Fn(&GameType) -> ScoreType> Scorable<GameType, ScoreType>
    for FnLike
{
    fn score(&self, game: &GameType) -> ScoreType {
        (self)(game)
    }
}

/// Provides an implementation for `wrapped_score` if the implementer implements the `score`
/// function.
///
/// `wrapped_score` takes into account if the node is an end_state, and depth based ordering so
/// that the underlying scoring functions don't need to worry about this
pub trait WrappedScorable<GameType, ScoreType>
where
    ScoreType: PartialOrd + Ord + Copy + Debug,
    GameType: YouDeterminableGame + VictorDeterminableGame + HealthGettableGame,
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
            let alive_count = node
                .get_snake_ids()
                .iter()
                .filter(|id| node.is_alive(id))
                .count() as u8;

            let score = match node.get_winner() {
                Some(s) => {
                    if s == *you_id {
                        WrappedScore::Win(Reverse(depth))
                    } else {
                        WrappedScore::Lose(Reverse(alive_count), depth)
                    }
                }
                None => WrappedScore::Tie(Reverse(alive_count), depth),
            };

            return Some(score);
        }

        if depth >= max_depth {
            return Some(WrappedScore::Scored(self.score(node)));
        }

        None
    }
}
