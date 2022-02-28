use std::cmp::Reverse;

use battlesnake_game_types::compact_representation::*;
use battlesnake_game_types::types::{
    HazardQueryableGame, HeadGettableGame, LengthGettableGame, NeighborDeterminableGame,
    PositionGettableGame, SnakeBodyGettableGame, SnakeIDGettableGame,
};

use battlesnake_game_types::compact_representation::CellNum;
use tinyvec::TinyVec;

pub struct Grid<T>
where
    T: SnakeIDGettableGame + ?Sized,
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

impl<T: CellNum, const BOARD_SIZE: usize, const MAX_SNAKES: usize> SpreadFromHead
    for StandardCellBoard<T, BOARD_SIZE, MAX_SNAKES>
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
                let value = if self
                    .is_hazard(&<Self as PositionGettableGame>::NativePositionType::from_usize(i))
                {
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
        let mut grid: Grid<Self> = Grid {
            cells: [None; 11 * 11],
        };

        let sorted_snake_ids = {
            let mut sids = self.get_snake_ids();
            sids.sort_unstable_by_key(|sid| Reverse(self.get_length(sid)));

            sids
        };

        let mut todos = Vec::with_capacity(16);
        let mut todos_per_snake = [0; 4];

        for sid in &sorted_snake_ids {
            for pos in self.get_snake_body_iter(sid) {
                grid.cells[pos.as_usize()] = Some(*sid);
            }
        }

        for sid in &sorted_snake_ids {
            let head = self.get_head_as_native_position(sid);
            todos.push(head);
            todos_per_snake[sid.as_usize()] += 1;
        }

        for _ in 0..number_of_cycles {
            let mut new_todos = Vec::with_capacity(todos.capacity() + 16);
            let mut new_todos_per_snake = [0; 4];

            let mut todos_iter = todos.into_iter();

            for sid in &sorted_snake_ids {
                for _ in 0..todos_per_snake[sid.as_usize()] {
                    // Mark Neighbors
                    let pos = todos_iter.next().unwrap();

                    for neighbor in self.neighbors(&pos) {
                        if grid.cells[neighbor.as_usize()].is_none() {
                            grid.cells[neighbor.as_usize()] = Some(*sid);
                            new_todos.push(neighbor);
                            new_todos_per_snake[sid.as_usize()] += 1;
                        }
                    }
                }
            }

            todos = new_todos;
            todos_per_snake = new_todos_per_snake;
        }

        grid
    }
}

impl<T: CellNum, const BOARD_SIZE: usize, const MAX_SNAKES: usize> SpreadFromHead
    for WrappedCellBoard<T, BOARD_SIZE, MAX_SNAKES>
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
                let value = if self
                    .is_hazard(&<Self as PositionGettableGame>::NativePositionType::from_usize(i))
                {
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
        let mut grid: Grid<Self> = Grid {
            cells: [None; 11 * 11],
        };

        let sorted_snake_ids = {
            let mut sids = self.get_snake_ids();
            sids.sort_unstable_by_key(|sid| Reverse(self.get_length(sid)));

            sids
        };

        let mut todo_per_snake: [TinyVec<
            [Option<<Self as PositionGettableGame>::NativePositionType>; 4],
        >; 4] = Default::default();

        for sid in &sorted_snake_ids {
            for pos in self.get_snake_body_iter(sid) {
                grid.cells[pos.as_usize()] = Some(*sid);
            }

            let head = self.get_head_as_native_position(sid);
            todo_per_snake[sid.0 as usize].push(Some(head));
        }

        for _ in 0..number_of_cycles {
            for sid in &sorted_snake_ids {
                let mut new_todo: TinyVec<
                    [Option<<Self as PositionGettableGame>::NativePositionType>; 4],
                > = Default::default();

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
