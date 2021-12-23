use std::collections::{HashMap, VecDeque};

use battlesnake_game_types::{
    compact_representation::{CellBoard4Snakes11x11, CellIndex},
    types::{
        HeadGettableGame, LengthGettableGame, NeighborDeterminableGame, PositionGettableGame,
        SnakeIDGettableGame, SnakeId,
    },
};
use itertools::Itertools;

struct Grid<T>
where
    T: SpreadFromHead,
    T::SnakeIDType: Copy,
{
    cells: [Option<T::SnakeIDType>; 11 * 11],
}

pub trait SpreadFromHead: SnakeIDGettableGame
where
    Self::SnakeIDType: Copy,
{
    fn squares_per_snake(&self) -> HashMap<Self::SnakeIDType, usize>;
}

impl<T> SpreadFromHead for T
where
    T: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + PositionGettableGame<NativePositionType = CellIndex<u8>>
        + LengthGettableGame
        + HeadGettableGame
        + NeighborDeterminableGame<NativePositionType = CellIndex<u8>>
        + Sync,
    T::SnakeIDType: Copy,
{
    fn squares_per_snake(&self) -> HashMap<Self::SnakeIDType, usize> {
        let mut grid: Grid<CellBoard4Snakes11x11> = Grid {
            cells: [None; 11 * 11],
        };

        let sorted_snake_ids = {
            let mut sids = self.get_snake_ids();
            sids.sort_unstable_by_key(|sid| self.get_length(sid));
            sids.reverse();

            sids
        };

        let mut todo: VecDeque<_> = Default::default();

        for sid in sorted_snake_ids {
            let head = self.get_head_as_native_position(&sid);
            grid.cells[head.0 as usize] = Some(sid);
            todo.push_back(head);
        }

        // Mark Neighbors
        while let Some(pos) = todo.pop_front() {
            let sid = grid.cells[pos.0 as usize];

            for neighbor in self.neighbors(&pos) {
                if grid.cells[neighbor.0 as usize].is_none() {
                    grid.cells[neighbor.0 as usize] = sid;
                    todo.push_back(neighbor);
                }
            }
        }

        grid.cells.iter().filter_map(|x| *x).counts()
    }
}
