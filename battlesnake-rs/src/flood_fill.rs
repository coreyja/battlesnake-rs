use std::{
    collections::{HashMap, VecDeque},
    convert::TryInto,
};

use crate::{Battlesnake, Coordinate, GameState};

pub fn squares_per_snake(state: &GameState) -> HashMap<String, usize> {
    let mut to_search: VecDeque<(&Battlesnake, Coordinate)> = VecDeque::new();
    let mut grid: Vec<Vec<Option<String>>> = vec![
        vec![None; state.board.width.try_into().unwrap()];
        state.board.height.try_into().unwrap()
    ];

    for s in &state.board.snakes {
        to_search.push_back((s, s.head));

        for c in &s.body {
            let (x, y) = c.to_usize();

            grid[x][y] = Some(s.id.clone());
        }
    }

    while let Some((snake, c)) = to_search.pop_front() {
        for n in c.neighbors(&state.board) {
            let (x, y) = n.to_usize();

            if grid[x][y].is_none() {
                grid[x][y] = Some(snake.id.clone());
                to_search.push_back((snake, n));
            }
        }
    }

    let mut counts = HashMap::new();

    for inner in grid {
        for id in inner.into_iter().flatten() {
            let prev_count = *counts.get(&id).unwrap_or(&0);
            counts.insert(id, prev_count + 1);
        }
    }

    counts
}
