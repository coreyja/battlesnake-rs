use std::cmp::Reverse;
use std::ops::Deref;

use battlesnake_game_types::{
    compact_representation::{CellNum, *},
    types::{
        FoodQueryableGame, HazardQueryableGame, HeadGettableGame, LengthGettableGame,
        NeighborDeterminableGame, PositionGettableGame, SizeDeterminableGame,
        SnakeBodyGettableGame, SnakeIDGettableGame, SnakeId,
    },
};
use tinyvec::TinyVec;

pub struct Grid<BoardType>
where
    BoardType: SnakeIDGettableGame + ?Sized,
    BoardType::SnakeIDType: Copy,
{
    pub(crate) cells: Vec<Option<BoardType::SnakeIDType>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scores {
    pub(crate) food: u16,
    pub(crate) hazard: u16,
    pub(crate) empty: u16,
}

pub trait SpreadFromHead<CellType, const MAX_SNAKES: usize> {
    type GridType;

    fn calculate(&self, number_of_cycles: usize) -> Self::GridType;
    fn squares_per_snake(&self, number_of_cycles: usize) -> [u8; MAX_SNAKES];
    fn squares_per_snake_with_scores(
        &self,
        number_of_cycles: usize,
        scores: Scores,
    ) -> [u16; MAX_SNAKES];
}

pub struct CellWrapper<CellType: CellNum>(pub(crate) CellIndex<CellType>);

impl<CellType: CellNum> Default for CellWrapper<CellType> {
    fn default() -> Self {
        CellWrapper(CellIndex::from_usize(0))
    }
}

impl<CellType: CellNum> Deref for CellWrapper<CellType> {
    type Target = CellIndex<CellType>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<BoardType, CellType, const MAX_SNAKES: usize> SpreadFromHead<CellType, MAX_SNAKES>
    for BoardType
where
    BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + PositionGettableGame<NativePositionType = CellIndex<CellType>>
        + SizeDeterminableGame
        + HazardQueryableGame
        + FoodQueryableGame
        + LengthGettableGame
        + NeighborDeterminableGame
        + HeadGettableGame
        + SnakeBodyGettableGame,
    CellType: CellNum,
{
    type GridType = Grid<BoardType>;

    fn calculate(&self, number_of_cycles: usize) -> Self::GridType {
        let mut grid: Grid<BoardType> = Grid {
            cells: vec![None; (self.get_height() * self.get_width()) as usize],
        };

        let sorted_snake_ids = {
            let mut sids = self.get_snake_ids();
            sids.sort_unstable_by_key(|sid| Reverse(self.get_length(sid)));

            sids
        };

        let mut todos: TinyVec<[CellWrapper<CellType>; 16]> = TinyVec::new();
        let mut todos_per_snake: [u8; MAX_SNAKES] = [0; MAX_SNAKES];

        for sid in &sorted_snake_ids {
            for pos in self.get_snake_body_iter(sid) {
                grid.cells[pos.as_usize()] = Some(*sid);
            }
        }

        for sid in &sorted_snake_ids {
            let head = self.get_head_as_native_position(sid);
            todos.push(CellWrapper(head));
            todos_per_snake[sid.as_usize()] += 1;
        }

        for _ in 0..number_of_cycles {
            if todos.is_empty() {
                break;
            }

            let mut new_todos = TinyVec::new();
            let mut new_todos_per_snake = [0; MAX_SNAKES];

            let mut todos_iter = todos.into_iter();

            for sid in &sorted_snake_ids {
                for _ in 0..todos_per_snake[sid.as_usize()] {
                    // Mark Neighbors
                    let pos = todos_iter.next().unwrap();

                    for neighbor in self.neighbors(&pos) {
                        if grid.cells[neighbor.as_usize()].is_none() {
                            grid.cells[neighbor.as_usize()] = Some(*sid);
                            new_todos.push(CellWrapper(neighbor));
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

    fn squares_per_snake(&self, number_of_cycles: usize) -> [u8; MAX_SNAKES] {
        let result = SpreadFromHead::<CellType, MAX_SNAKES>::calculate(self, number_of_cycles);
        let cell_sids = result.cells.iter().filter_map(|x| *x);

        let mut total_values = [0; MAX_SNAKES];

        for sid in cell_sids {
            total_values[sid.as_usize()] += 1;
        }

        total_values
    }

    fn squares_per_snake_with_scores(
        &self,
        number_of_cycles: usize,
        scores: Scores,
    ) -> [u16; MAX_SNAKES] {
        let grid = SpreadFromHead::<CellType, MAX_SNAKES>::calculate(self, number_of_cycles);

        let sid_and_values = grid
            .cells
            .iter()
            .enumerate()
            .filter_map(|x| x.1.map(|sid| (x.0, sid)))
            .map(|(i, sid)| {
                let value = if self.is_hazard(
                    &<BoardType as PositionGettableGame>::NativePositionType::from_usize(i),
                ) {
                    scores.hazard
                } else if self.is_food(
                    &<BoardType as PositionGettableGame>::NativePositionType::from_usize(i),
                ) {
                    scores.food
                } else {
                    scores.empty
                };

                (sid, value)
            });

        let mut total_values = [0_u16; MAX_SNAKES];

        for (sid, value) in sid_and_values {
            total_values[sid.as_usize()] += value;
        }

        total_values
    }
}
