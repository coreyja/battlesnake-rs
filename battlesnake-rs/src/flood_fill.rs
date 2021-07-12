use std::collections::{HashMap, VecDeque};

use crate::{Battlesnake, BoardGridItem, Coordinate, GameState};

pub fn squares_per_snake(
    state: &GameState,
    max_number_of_iterations: Option<usize>,
) -> HashMap<String, u32> {
    const DEFAULT_MAX_NUMBER_OF_ITERATIONS: usize = 50;
    let max_number_of_iterations =
        max_number_of_iterations.unwrap_or(DEFAULT_MAX_NUMBER_OF_ITERATIONS);

    let mut to_search: VecDeque<(&Battlesnake, Coordinate)> = VecDeque::new();
    let mut grid = state.board.to_grid().0;
    let mut counts = HashMap::new();
    let mut count = 0;

    for s in &state.board.snakes {
        to_search.push_back((s, s.head));
        counts.insert(s.id.clone(), 0);
    }

    while let Some((snake, c)) = to_search.pop_front() {
        count += 1;
        if count > max_number_of_iterations {
            break;
        }

        for n in c.neighbors(&state.board) {
            let (x, y) = n.to_usize();

            match grid[x][y] {
                None | Some(BoardGridItem::Food) | Some(BoardGridItem::Hazard) => {
                    grid[x][y] = Some(BoardGridItem::Snake(&snake.id));
                    to_search.push_back((snake, n));

                    let prev_count = *counts.get(&snake.id).unwrap_or(&0);
                    counts.insert(snake.id.clone(), prev_count + 1);
                }
                Some(BoardGridItem::Snake(_)) => {}
            };
        }
    }

    counts
}
