use std::fmt::Debug;

use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::thread_rng;

use battlesnake_game_types::types::{Move, SnakeIDGettableGame};

use super::MinMaxReturn;

#[derive(Debug, Clone, Copy)]
pub enum MoveOrdering {
    BestFirst,
    Random,
}

fn best_first<GameType, ScoreType>(
    previous_return: Option<MinMaxReturn<GameType, ScoreType>>,
    possible_moves: impl Iterator<Item = Move>,
) -> Vec<(Move, Option<MinMaxReturn<GameType, ScoreType>>)>
where
    GameType: Debug + Clone + SnakeIDGettableGame,
    ScoreType: Copy + Ord + PartialOrd + Debug,
{
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

impl MoveOrdering {
    pub fn order_moves<GameType, ScoreType>(
        &self,
        previous_return: Option<MinMaxReturn<GameType, ScoreType>>,
        possible_moves: impl Iterator<Item = Move>,
    ) -> Vec<(Move, Option<MinMaxReturn<GameType, ScoreType>>)>
    where
        GameType: Debug + Clone + SnakeIDGettableGame,
        ScoreType: Copy + Ord + PartialOrd + Debug,
    {
        match &self {
            MoveOrdering::BestFirst => best_first(previous_return, possible_moves),
            MoveOrdering::Random => {
                let mut moves = possible_moves.map(|x| (x, None)).collect_vec();
                moves.shuffle(&mut thread_rng());

                moves
            }
        }
    }
}
