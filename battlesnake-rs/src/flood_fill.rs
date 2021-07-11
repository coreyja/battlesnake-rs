use std::{
    collections::{HashMap, VecDeque},
    convert::TryInto,
};

use crate::{Battlesnake, Coordinate, GameState};

pub fn squares_per_snake(state: &GameState) -> HashMap<String, u32> {
    let mut to_search: VecDeque<(&Battlesnake, Coordinate)> = VecDeque::new();
    let mut grid: Vec<Vec<Option<&str>>> = vec![
        vec![None; state.board.width.try_into().unwrap()];
        state.board.height.try_into().unwrap()
    ];
    let mut counts = HashMap::new();

    for s in &state.board.snakes {
        to_search.push_back((s, s.head));
        counts.insert(s.id.clone(), 0);

        for c in &s.body {
            let (x, y) = c.to_usize();

            grid[x][y] = Some(&s.id);
        }
    }

    let max_count = state.board.width * state.board.height / 8;

    while let Some((snake, c)) = to_search.pop_front() {
        for n in c.neighbors(&state.board) {
            let (x, y) = n.to_usize();

            if grid[x][y].is_none() {
                grid[x][y] = Some(&snake.id);
                to_search.push_back((snake, n));

                let prev_count = *counts.get(&snake.id).unwrap_or(&0);
                counts.insert(snake.id.clone(), prev_count + 1);

                if prev_count > max_count {
                    return counts;
                }
            }
        }
    }

    counts
}
