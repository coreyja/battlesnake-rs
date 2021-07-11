use std::collections::{HashMap, VecDeque};

use crate::{Battlesnake, BoardGridItem, Coordinate, GameState};

pub fn squares_per_snake(state: &GameState) -> HashMap<String, u32> {
    let mut to_search: VecDeque<(&Battlesnake, Coordinate)> = VecDeque::new();
    let mut grid = state.board.to_grid().0;
    let mut counts = HashMap::new();

    for s in &state.board.snakes {
        to_search.push_back((s, s.head));
        counts.insert(s.id.clone(), 0);
    }

    let max_count = state.board.width * state.board.height / 8;

    while let Some((snake, c)) = to_search.pop_front() {
        for n in c.neighbors(&state.board) {
            let (x, y) = n.to_usize();

            match grid[x][y] {
                None | Some(BoardGridItem::Food) | Some(BoardGridItem::Hazard) => {
                    grid[x][y] = Some(BoardGridItem::Snake(&snake.id));
                    to_search.push_back((snake, n));

                    let prev_count = *counts.get(&snake.id).unwrap_or(&0);
                    counts.insert(snake.id.clone(), prev_count + 1);

                    if prev_count > max_count {
                        return counts;
                    }
                }
                Some(BoardGridItem::Snake(_)) => {}
            };
        }
    }

    counts
}
