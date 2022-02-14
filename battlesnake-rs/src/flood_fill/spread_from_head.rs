use std::cmp::Reverse;
use std::collections::HashMap;

use battlesnake_game_types::types::{
    HazardQueryableGame, HeadGettableGame, LengthGettableGame, NeighborDeterminableGame,
    PositionGettableGame, SnakeBodyIterableGame, SnakeIDGettableGame, SnakeId,
};
use itertools::Itertools;
use tinyvec::{tiny_vec, TinyVec};

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
    fn squares_per_snake(&self, number_of_cycles: usize) -> [u8; 4];
    fn squares_per_snake_with_hazard_cost(
        &self,
        number_of_cycles: usize,
        hazard_cost: u16,
    ) -> [u16; 4];
}

pub trait CellToUsizeAble<T> {
    fn from_usize(x: usize) -> Self;
    fn as_usize(&self) -> usize;
}

impl<T: battlesnake_game_types::compact_representation::CellNum> CellToUsizeAble<T>
    for battlesnake_game_types::compact_representation::CellIndex<T>
{
    fn from_usize(x: usize) -> Self {
        battlesnake_game_types::compact_representation::CellIndex(T::from_usize(x))
    }

    fn as_usize(&self) -> usize {
        self.0.as_usize()
    }
}

impl<T: battlesnake_game_types::wrapped_compact_representation::CellNum> CellToUsizeAble<T>
    for battlesnake_game_types::wrapped_compact_representation::CellIndex<T>
{
    fn from_usize(x: usize) -> Self {
        battlesnake_game_types::wrapped_compact_representation::CellIndex(T::from_usize(x))
    }

    fn as_usize(&self) -> usize {
        self.0.as_usize()
    }
}

impl<T> SpreadFromHead for T
where
    T: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + PositionGettableGame
        + LengthGettableGame
        + SnakeBodyIterableGame
        + HeadGettableGame
        + HazardQueryableGame
        + NeighborDeterminableGame
        + Sync,
    T::SnakeIDType: Copy,
    T::NativePositionType: CellToUsizeAble<u8>,
{
    fn squares_per_snake(&self, number_of_cycles: usize) -> [u8; 4] {
        let result = self.calculate(number_of_cycles);
        let cell_sids = result.cells.iter().filter_map(|x| *x);

        let mut total_values = [0; 4];

        for sid in cell_sids {
            total_values[sid.as_usize()] += 1;
        }

        total_values
    }

    fn squares_per_snake_with_hazard_cost(
        &self,
        number_of_cycles: usize,
        non_hazard_bonus: u16,
    ) -> [u16; 4] {
        let grid = self.calculate(number_of_cycles);

        let sid_and_values = grid
            .cells
            .iter()
            .enumerate()
            .filter_map(|x| x.1.map(|sid| (x.0, sid)))
            .map(|(i, sid)| {
                let value = if self.is_hazard(&T::NativePositionType::from_usize(i)) {
                    1
                } else {
                    non_hazard_bonus + 1
                };
                (sid, value)
            });

        let mut total_values = [0; 4];

        for (sid, value) in sid_and_values {
            total_values[sid.as_usize()] += value;
        }

        total_values
    }

    fn calculate(&self, number_of_cycles: usize) -> Grid<Self> {
        let mut grid: Grid<T> = Grid {
            cells: [None; 11 * 11],
        };

        let sorted_snake_ids = {
            let mut sids = self.get_snake_ids();
            sids.sort_unstable_by_key(|sid| Reverse(self.get_length(sid)));

            sids
        };

        let mut todo_per_snake: [TinyVec<[Option<T::NativePositionType>; 4]>; 4] =
            Default::default();

        for sid in &sorted_snake_ids {
            for pos in self.get_snake_body_iter(sid) {
                grid.cells[pos.as_usize()] = Some(*sid);
            }
        }

        for sid in &sorted_snake_ids {
            let head = self.get_head_as_native_position(sid);
            todo_per_snake[sid.0 as usize].push(Some(head));
        }

        for _ in 0..number_of_cycles {
            for sid in &sorted_snake_ids {
                let mut new_todo: TinyVec<[Option<T::NativePositionType>; 4]> = Default::default();

                // Mark Neighbors
                while let Some(pos) = todo_per_snake[sid.0 as usize].pop() {
                    let pos =
                        pos.expect("I forced everything into a Some so I could use a TinyVec here");

                    for neighbor in self.neighbors(&pos) {
                        if grid.cells[neighbor.as_usize()].is_none() {
                            grid.cells[neighbor.as_usize()] = Some(*sid);
                            new_todo.push(Some(neighbor));
                        }
                    }
                }

                todo_per_snake[sid.0 as usize] = new_todo;
            }
        }

        grid
    }
}
