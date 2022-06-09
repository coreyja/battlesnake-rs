use std::cmp::Reverse;

use battlesnake_game_types::{
    compact_representation::{CellIndex, CellNum},
    types::{
        HazardQueryableGame, HeadGettableGame, LengthGettableGame, NeighborDeterminableGame,
        PositionGettableGame, SizeDeterminableGame, SnakeBodyGettableGame, SnakeIDGettableGame,
        SnakeId,
    },
};
use tinyvec::TinyVec;

pub use super::spread_from_head::*;

pub trait SpreadFromHeadArcadeMaze<CellType> {
    type GridType;

    fn calculate(&self, number_of_cycles: usize) -> Self::GridType;
    fn squares_per_snake_hazard_maze(&self, number_of_cycles: usize) -> [u8; 4];
}

// Board: 19 x 21
// (3, 11) = 11 + 3*19 = 68
// (9, 11) = 11 + 9*19 = 182
// (15, 11) = 11 + 15*19 = 296
const FOOD_SPAWN_LOCATION_INDEX: [usize; 3] = [68, 182, 296];

impl<BoardType, CellType> SpreadFromHeadArcadeMaze<CellType> for BoardType
where
    BoardType: SnakeIDGettableGame<SnakeIDType = SnakeId>
        + PositionGettableGame<NativePositionType = CellIndex<CellType>>
        + SizeDeterminableGame
        + HazardQueryableGame
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
        let mut todos_per_snake: [u8; 4] = [0; 4];

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
            let mut new_todos_per_snake = [0; 4];

            let mut todos_iter = todos.into_iter();

            for sid in &sorted_snake_ids {
                for _ in 0..todos_per_snake[sid.as_usize()] {
                    // Mark Neighbors
                    let pos = todos_iter.next().unwrap();

                    for neighbor in self.neighbors(&pos).filter(|p| !self.is_hazard(p)) {
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

    fn squares_per_snake_hazard_maze(&self, number_of_cycles: usize) -> [u8; 4] {
        let result = SpreadFromHeadArcadeMaze::calculate(self, number_of_cycles);

        let mut total_values = [0; 4];

        for (i, sid) in result
            .cells
            .iter()
            .enumerate()
            .filter_map(|(i, x)| x.map(|sid| (i, sid)))
        {
            if FOOD_SPAWN_LOCATION_INDEX.contains(&i) {
                total_values[sid.as_usize()] += 10
            } else {
                total_values[sid.as_usize()] += 1
            }
        }

        total_values
    }
}
