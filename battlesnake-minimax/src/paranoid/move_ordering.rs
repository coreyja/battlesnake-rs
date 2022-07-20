use std::fmt::Debug;

use types::types::{Move, SnakeIDGettableGame};

use super::MinMaxReturn;

pub trait MoveOrderable<GameType, ScoreType>
where
    GameType: Debug + Clone + SnakeIDGettableGame,
    ScoreType: Copy + Ord + PartialOrd + Debug,
{
    fn order_moves(
        &self,
        previous_return: Option<MinMaxReturn<GameType, ScoreType>>,
        possible_moves: impl Iterator<Item = Move>,
    ) -> Vec<(Move, Option<MinMaxReturn<GameType, ScoreType>>)>;
}

#[derive(Copy, Clone, Debug)]
pub struct BestFirst;

impl<GameType, ScoreType> MoveOrderable<GameType, ScoreType> for BestFirst
where
    GameType: Debug + Clone + SnakeIDGettableGame,
    ScoreType: Copy + Ord + PartialOrd + Debug,
{
    fn order_moves(
        &self,
        previous_return: Option<MinMaxReturn<GameType, ScoreType>>,
        possible_moves: impl Iterator<Item = Move>,
    ) -> Vec<(Move, Option<MinMaxReturn<GameType, ScoreType>>)> {
        if let Some(MinMaxReturn::Node { mut options, .. }) = previous_return {
            let mut v: Vec<_> = possible_moves
                .into_iter()
                .map(|m| {
                    (
                        m,
                        options
                            .iter()
                            .position(|x| x.0 == m)
                            .map(|x| options.remove(x).1),
                    )
                })
                .collect();
            v.sort_by_cached_key(|(_, r)| r.as_ref().map(|x| *x.score()));
            v.reverse();
            v
        } else {
            possible_moves.into_iter().map(|m| (m, None)).collect()
        }
    }
}
