//! A tail-aware variant of [`crate::flood_fill::spread_from_head`].
//!
//! The original spread seeds every snake body square as impassable and never releases it, so a
//! square that your own tail is about to vacate counts as a wall forever. That is wrong in the
//! positions that matter most: a long snake coiled against itself has almost no "free" area even
//! when it can survive indefinitely by chasing its own tail.
//!
//! This variant tracks, per body square, how many ticks remain until the tail passes it, and lets
//! the spread enter that square on the cycle after it vacates. Squares are counted for scoring
//! exactly as the original does, so the two variants are directly comparable.
//!
//! Known approximations:
//! - Growth is ignored. A snake that eats does not move its tail that tick, so vacate times are
//!   optimistic by one tick per food eaten along the way.
//! - Stacked body pieces (a freshly spawned or just-fed snake) are handled by taking the *latest*
//!   vacate time for a square, which is the conservative choice.

use std::cmp::Reverse;

use battlesnake_game_types::{
    compact_representation::{CellNum, *},
    types::{
        FoodQueryableGame, HazardQueryableGame, HeadGettableGame, LengthGettableGame,
        NeighborDeterminableGame, PositionGettableGame, SizeDeterminableGame,
        SnakeBodyGettableGame, SnakeIDGettableGame, SnakeId,
    },
};
use tinyvec::TinyVec;

use super::spread_from_head::{count_grid, score_grid, CellWrapper, Grid, Scores};

pub trait SpreadFromHeadTailAware<CellType, const MAX_SNAKES: usize> {
    type GridType;

    fn calculate_tail_aware(&self, number_of_cycles: usize) -> Self::GridType;
    fn squares_per_snake_tail_aware(&self, number_of_cycles: usize) -> [u8; MAX_SNAKES];
    fn squares_per_snake_with_scores_tail_aware(
        &self,
        number_of_cycles: usize,
        scores: Scores,
    ) -> [u16; MAX_SNAKES];
}

impl<BoardType, CellType, const MAX_SNAKES: usize> SpreadFromHeadTailAware<CellType, MAX_SNAKES>
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

    fn calculate_tail_aware(&self, number_of_cycles: usize) -> Self::GridType {
        let board_size = (self.get_height() * self.get_width()) as usize;

        let mut grid: Grid<BoardType> = Grid {
            cells: vec![None; board_size],
        };

        // `body_life[i]` is the first cycle on which cell `i` is enterable, or 0 if the cell is not
        // a body square at all. A tail is enterable on cycle 1 (it vacates this tick), the square
        // in front of it on cycle 2, and so on.
        let mut body_life: Vec<u16> = vec![0; board_size];

        // Whether the spread has already reached a cell. Body squares start unvisited so they can
        // be claimed once they vacate; heads start visited because that is where every frontier
        // begins.
        let mut visited: Vec<bool> = vec![false; board_size];

        let sorted_snake_ids = {
            let mut sids = self.get_snake_ids();
            sids.sort_unstable_by_key(|sid| Reverse(self.get_length(sid)));

            sids
        };

        for sid in &sorted_snake_ids {
            let body = self.get_snake_body_vec(sid);
            let len = body.len();

            for (index_from_head, pos) in body.iter().enumerate() {
                let cell = pos.as_usize();
                // Ticks until the tail passes this square: the tail (index len - 1) gives 1.
                let enterable_on = (len - index_from_head) as u16;

                grid.cells[cell] = Some(*sid);
                // A stacked square appears twice; the entry nearer the head vacates later and wins.
                body_life[cell] = body_life[cell].max(enterable_on);
            }
        }

        let mut todos: TinyVec<[CellWrapper<CellType>; 16]> = TinyVec::new();
        let mut todos_per_snake: [u8; MAX_SNAKES] = [0; MAX_SNAKES];

        for sid in &sorted_snake_ids {
            let head = self.get_head_as_native_position(sid);
            visited[head.as_usize()] = true;
            todos.push(CellWrapper(head));
            todos_per_snake[sid.as_usize()] += 1;
        }

        for cycle in 1..=number_of_cycles {
            if todos.is_empty() {
                break;
            }

            let cycle = cycle as u16;

            let mut new_todos = TinyVec::new();
            let mut new_todos_per_snake = [0; MAX_SNAKES];

            let mut todos_iter = todos.into_iter();

            for sid in &sorted_snake_ids {
                for _ in 0..todos_per_snake[sid.as_usize()] {
                    let pos = todos_iter.next().unwrap();

                    for neighbor in self.neighbors(&pos) {
                        let cell = neighbor.as_usize();

                        if visited[cell] {
                            continue;
                        }

                        // Empty cells have `body_life == 0`; body cells open once their tail has
                        // passed.
                        if body_life[cell] > cycle {
                            continue;
                        }

                        visited[cell] = true;
                        grid.cells[cell] = Some(*sid);
                        new_todos.push(CellWrapper(neighbor));
                        new_todos_per_snake[sid.as_usize()] += 1;
                    }
                }
            }

            todos = new_todos;
            todos_per_snake = new_todos_per_snake;
        }

        grid
    }

    fn squares_per_snake_tail_aware(&self, number_of_cycles: usize) -> [u8; MAX_SNAKES] {
        let grid = SpreadFromHeadTailAware::<CellType, MAX_SNAKES>::calculate_tail_aware(
            self,
            number_of_cycles,
        );

        count_grid(&grid)
    }

    fn squares_per_snake_with_scores_tail_aware(
        &self,
        number_of_cycles: usize,
        scores: Scores,
    ) -> [u16; MAX_SNAKES] {
        let grid = SpreadFromHeadTailAware::<CellType, MAX_SNAKES>::calculate_tail_aware(
            self,
            number_of_cycles,
        );

        score_grid(self, &grid, scores)
    }
}

#[cfg(test)]
mod tests {
    use battlesnake_game_types::{
        compact_representation::{StandardCellBoard4Snakes11x11, WrappedCellBoard4Snakes11x11},
        types::build_snake_id_map,
        wire_representation::{Game, Position},
    };

    use super::*;
    use crate::flood_fill::spread_from_head::SpreadFromHead;

    /// Build an 11x11 standard board from explicit snake bodies, head first.
    fn board(bodies: &[&[(i32, i32)]]) -> StandardCellBoard4Snakes11x11 {
        let snakes: Vec<_> = bodies
            .iter()
            .enumerate()
            .map(|(i, body)| {
                let body: Vec<Position> = body
                    .iter()
                    .map(|(x, y)| Position { x: *x, y: *y })
                    .collect();
                serde_json::json!({
                    "id": format!("snake-{i}"),
                    "name": format!("snake-{i}"),
                    "head": body[0],
                    "body": body,
                    "health": 100,
                    "shout": "",
                })
            })
            .collect();

        let wire = serde_json::json!({
            "you": snakes[0],
            "board": { "height": 11, "width": 11, "food": [], "snakes": snakes, "hazards": [] },
            "turn": 1,
            "game": {
                "id": "test",
                "ruleset": { "name": "standard", "version": "1.0" },
                "timeout": 500,
            },
        });

        let game: Game = serde_json::from_value(wire).unwrap();
        let id_map = build_snake_id_map(&game);

        StandardCellBoard4Snakes11x11::convert_from_game(game, &id_map).unwrap()
    }

    fn totals_naive<B: SpreadFromHead<u8, 4>>(board: &B, cycles: usize) -> u32 {
        SpreadFromHead::<u8, 4>::squares_per_snake(board, cycles)
            .iter()
            .map(|x| u32::from(*x))
            .sum()
    }

    fn totals_tail_aware<B: SpreadFromHeadTailAware<u8, 4>>(board: &B, cycles: usize) -> u32 {
        SpreadFromHeadTailAware::<u8, 4>::squares_per_snake_tail_aware(board, cycles)
            .iter()
            .map(|x| u32::from(*x))
            .sum()
    }

    /// A snake coiled into the corner so that both of its head's on-board neighbours are its own
    /// body: `(0,0) -> (0,1) -> (1,1) -> (1,0)`. The head at `(0,0)` is boxed in by `(0,1)` (three
    /// ticks of life left) and the tail at `(1,0)` (vacating this tick).
    const SEALED_CORNER: &[(i32, i32)] = &[(0, 0), (0, 1), (1, 1), (1, 0)];

    #[test]
    fn naive_spread_cannot_leave_a_self_sealed_head() {
        let board = board(&[SEALED_CORNER]);

        // Only the four body squares are ever owned.
        assert_eq!(SpreadFromHead::<u8, 4>::squares_per_snake(&board, 5)[0], 4);
    }

    #[test]
    fn tail_aware_spread_escapes_through_the_vacating_tail() {
        let board = board(&[SEALED_CORNER]);

        // Hand-derived: cycle 1 enters the tail (1,0); 2 adds (2,0) and reclaims (1,1); 3 adds
        // (3,0),(2,1),(1,2); 4 adds (4,0),(3,1),(2,2),(0,2),(1,3); 5 adds
        // (5,0),(4,1),(3,2),(2,3),(0,3),(1,4). 4 body + 15 new = 19.
        assert_eq!(
            SpreadFromHeadTailAware::<u8, 4>::squares_per_snake_tail_aware(&board, 5)[0],
            19
        );
    }

    #[test]
    fn body_squares_open_on_schedule_not_immediately() {
        let board = board(&[SEALED_CORNER]);

        let at = |cycles| {
            SpreadFromHeadTailAware::<u8, 4>::squares_per_snake_tail_aware(&board, cycles)[0]
        };

        // Cycle 0: nothing has moved, so only the body.
        assert_eq!(at(0), 4);
        // Cycle 1: the tail vacates, but it already counts as ours, so the total holds at 4. This
        // is the assertion that the *next* square up the body has not opened early.
        assert_eq!(at(1), 4);
        // Cycle 2: (1,1) opens on schedule and (2,0) is reached through the vacated tail.
        assert_eq!(at(2), 5);
    }

    #[test]
    fn a_long_body_square_stays_a_wall_inside_the_cycle_budget() {
        // A 2x5 serpentine filling the bottom-left corner, head at (0,0). The head's only on-board
        // neighbours are body squares nine and seven ticks from vacating, so nothing opens within
        // five cycles and tail-awareness changes nothing here.
        let coiled: &[(i32, i32)] = &[
            (0, 0),
            (1, 0),
            (1, 1),
            (0, 1),
            (0, 2),
            (1, 2),
            (1, 3),
            (0, 3),
            (0, 4),
            (1, 4),
        ];
        let board = board(&[coiled]);

        assert_eq!(SpreadFromHead::<u8, 4>::squares_per_snake(&board, 5)[0], 10);
        assert_eq!(
            SpreadFromHeadTailAware::<u8, 4>::squares_per_snake_tail_aware(&board, 5)[0],
            10
        );
    }

    #[test]
    fn open_board_spreads_agree_when_no_body_is_in_the_way() {
        // Two short snakes in open space, far apart: no body square blocks either frontier within
        // the budget, so both variants claim exactly the same squares.
        let a: &[(i32, i32)] = &[(2, 2), (2, 1), (2, 0)];
        let b: &[(i32, i32)] = &[(8, 8), (8, 9), (8, 10)];
        let board = board(&[a, b]);

        assert_eq!(
            SpreadFromHead::<u8, 4>::squares_per_snake(&board, 5),
            SpreadFromHeadTailAware::<u8, 4>::squares_per_snake_tail_aware(&board, 5)
        );
    }

    /// The tail-aware frontier is a superset of the naive one at every cycle, so the total number
    /// of owned squares can only grow. An individual snake can lose squares to a rival that now
    /// reaches them first, which is why this checks the sum.
    #[test]
    fn tail_awareness_never_shrinks_the_claimed_area_on_real_games() {
        let fixtures = [
            include_str!("../../../fixtures/095b30fa-f2c7-4826-ac93-90b4dde6b785_5.json"),
            include_str!("../../../fixtures/130b18e2-8689-4d64-a09f-c4345f80ae79_25.json"),
            include_str!("../../../fixtures/4f198c01-d613-4109-b8b9-226208cde009_505.json"),
            include_str!("../../../fixtures/7311099d-b98a-4589-9b05-32dc80362bcc_135.json"),
            include_str!("../../../fixtures/7a02e19b-f658-4639-8ace-ece46629a6ed_192.json"),
            include_str!("../../../fixtures/95d72d73-352b-4ad5-83e4-86139fa556a9_54.json"),
        ];

        for (i, fixture) in fixtures.iter().enumerate() {
            let game: Game = serde_json::from_str(fixture).unwrap();
            let wrapped = game.game.ruleset.name == "wrapped";
            let id_map = build_snake_id_map(&game);

            for cycles in [1, 3, 5, 8, 20] {
                let (naive, tail_aware) = if wrapped {
                    let board =
                        WrappedCellBoard4Snakes11x11::convert_from_game(game.clone(), &id_map)
                            .unwrap();
                    (
                        totals_naive(&board, cycles),
                        totals_tail_aware(&board, cycles),
                    )
                } else {
                    let board =
                        StandardCellBoard4Snakes11x11::convert_from_game(game.clone(), &id_map)
                            .unwrap();
                    (
                        totals_naive(&board, cycles),
                        totals_tail_aware(&board, cycles),
                    )
                };

                assert!(
                    tail_aware >= naive,
                    "fixture {i} at {cycles} cycles: tail-aware {tail_aware} < naive {naive}"
                );
            }
        }
    }
}
