use std::collections::HashMap;

use battlesnake_game_types::{
    compact_representation::{
        dimensions::Dimensions, CellNum, StandardCellBoard4Snakes11x11, WrappedCellBoard,
    },
    types::{HeadGettableGame, PositionGettableGame, SnakeIDGettableGame},
    wire_representation::Position,
};

use itertools::Itertools;

pub trait JumpFlooding: SnakeIDGettableGame
where
    Self::SnakeIDType: Copy,
{
    fn squares_per_snake(&self) -> HashMap<Self::SnakeIDType, usize>;
}

struct Grid<T>
where
    T: SnakeIDGettableGame,
    T::SnakeIDType: Copy,
{
    cells: [Option<T::SnakeIDType>; 11 * 11],
}

impl<T: CellNum, D: Dimensions, const BOARD_SIZE: usize, const MAX_SNAKES: usize> JumpFlooding
    for WrappedCellBoard<T, D, BOARD_SIZE, MAX_SNAKES>
{
    fn squares_per_snake(&self) -> HashMap<Self::SnakeIDType, usize> {
        let mut grid: Grid<StandardCellBoard4Snakes11x11> = Grid {
            cells: [None; 11 * 11],
        };

        // Pre-seed the grid from the Board
        for sid in self.get_snake_ids().iter() {
            let head = self.get_head_as_native_position(sid);

            grid.cells[head.0.as_usize()] = Some(*sid);
        }

        // This comes from k = [ N/2, N/4, N/8, ..., 1 ]
        // But I introduced some specific rounding for N = 11
        let steps = [6, 3, 1];

        for neighbor_distance in steps {
            (0..(11 * 11)).into_iter().for_each(|i| {
                let neighbor_options = [-neighbor_distance, 0, neighbor_distance];
                let neighbors = neighbor_options
                    .iter()
                    .permutations(2)
                    .filter_map(|coords| {
                        let y = i / 11;
                        let x = i % 11;
                        let pos = Position { x, y };

                        let new_x = pos.x + coords[0];
                        if !(0..11).contains(&new_x) {
                            return None;
                        }

                        let new_y = pos.y + coords[1];
                        if !(0..11).contains(&new_y) {
                            return None;
                        }

                        Some(self.native_from_position(Position { x: new_x, y: new_y }))
                    });

                for neighbor in neighbors {
                    if let Some(nid) = grid.cells[neighbor.0.as_usize()] {
                        if let Some(sid) = grid.cells[i as usize] {
                            if sid != nid {
                                let n_dist = manhattan_distance(
                                    self.get_head_as_native_position(&nid).0.as_usize(),
                                    i,
                                );
                                let s_dist = manhattan_distance(
                                    self.get_head_as_native_position(&sid).0.as_usize(),
                                    i,
                                );

                                if n_dist < s_dist {
                                    grid.cells[i as usize] = Some(nid);
                                }
                            }
                        } else {
                            grid.cells[i as usize] = Some(nid);
                        }
                    }
                }
            })
        }

        grid.cells.iter().filter_map(|x| *x).counts()
    }
}

fn manhattan_distance(a: usize, b: i32) -> i32 {
    let a = a as i32;
    let diff = if a > b { a - b } else { b - a };

    (diff / 11) + (diff % 11)
}
