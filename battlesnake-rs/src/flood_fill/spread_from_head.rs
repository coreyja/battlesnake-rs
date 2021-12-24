use std::collections::{HashMap, VecDeque};

use battlesnake_game_types::{
    compact_representation::{CellBoard4Snakes11x11, CellIndex},
    types::{
        HazardQueryableGame, HeadGettableGame, LengthGettableGame, NeighborDeterminableGame,
        PositionGettableGame, SnakeBodyGettableGame, SnakeIDGettableGame, SnakeId,
    },
};
use itertools::Itertools;

pub struct Grid<T>
where
    T: SpreadFromHead + ?Sized,
    T::SnakeIDType: Copy,
{
    cells: [Option<T::SnakeIDType>; 11 * 11],
}

pub trait SpreadFromHead: SnakeIDGettableGame
where
    Self::SnakeIDType: Copy,
{
    fn calculate(&self, number_of_cycles: usize) -> Grid<Self>;
    fn squares_per_snake(&self, number_of_cycles: usize) -> HashMap<Self::SnakeIDType, usize>;
    fn squares_per_snake_with_hazard_cost(
        &self,
        number_of_cycles: usize,
        hazard_cost: u64,
    ) -> HashMap<Self::SnakeIDType, u64>;
}

impl<T> SpreadFromHead for T
where
    T: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + PositionGettableGame<NativePositionType = CellIndex<u8>>
        + LengthGettableGame
        + SnakeBodyGettableGame
        + HeadGettableGame<NativePositionType = CellIndex<u8>>
        + HazardQueryableGame<NativePositionType = CellIndex<u8>>
        + NeighborDeterminableGame<NativePositionType = CellIndex<u8>>
        + Sync,
    T::SnakeIDType: Copy,
{
    fn squares_per_snake(&self, number_of_cycles: usize) -> HashMap<Self::SnakeIDType, usize> {
        self.calculate(number_of_cycles)
            .cells
            .iter()
            .filter_map(|x| *x)
            .counts()
    }

    fn squares_per_snake_with_hazard_cost(
        &self,
        number_of_cycles: usize,
        non_hazard_bonus: u64,
    ) -> HashMap<Self::SnakeIDType, u64> {
        let grid = self.calculate(number_of_cycles);

        let sid_and_values = grid
            .cells
            .iter()
            .enumerate()
            .filter_map(|x| x.1.map(|sid| (x.0, sid)))
            .map(|(i, sid)| {
                let value = if self.is_hazard(&CellIndex(i as u8)) {
                    1
                } else {
                    non_hazard_bonus + 1
                };
                (sid, value)
            });

        let mut total_values = HashMap::new();

        for (sid, value) in sid_and_values {
            let total_value = total_values.entry(sid).or_insert(0);
            *total_value += value;
        }

        total_values
    }

    fn calculate(&self, number_of_cycles: usize) -> Grid<Self> {
        let mut grid: Grid<T> = Grid {
            cells: [None; 11 * 11],
        };

        let sorted_snake_ids = {
            let mut sids = self.get_snake_ids();
            sids.sort_unstable_by_key(|sid| self.get_length(sid));
            sids.reverse();

            sids
        };

        let mut todo_per_snake = vec![vec![]; 4];

        for sid in &sorted_snake_ids {
            for pos in self.get_snake_body_vec(sid) {
                grid.cells[pos.0 as usize] = Some(*sid);
            }
        }

        for sid in &sorted_snake_ids {
            let head = self.get_head_as_native_position(sid);
            todo_per_snake[sid.0 as usize].push(head);
        }

        for _ in 0..number_of_cycles {
            for sid in self.get_snake_ids() {
                let mut new_todo: Vec<_> = Default::default();

                // Mark Neighbors
                while let Some(pos) = todo_per_snake[sid.0 as usize].pop() {
                    for neighbor in self.neighbors(&pos) {
                        if grid.cells[neighbor.0 as usize].is_none() {
                            grid.cells[neighbor.0 as usize] = Some(sid);
                            new_todo.push(neighbor);
                        }
                    }
                }

                todo_per_snake[sid.0 as usize] = new_todo;
            }
        }

        grid
    }
}
