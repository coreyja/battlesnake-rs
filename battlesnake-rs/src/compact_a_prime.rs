use battlesnake_game_types::compact_representation::{Cell, CellBoard4Snakes11x11, CellIndex};
use battlesnake_game_types::types::Move;
use battlesnake_game_types::wire_representation::Position;

use crate::{Board, Direction};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

const NEIGHBOR_DISTANCE: i32 = 1;
const HAZARD_PENALTY: i32 = 1;
const HEURISTIC_MAX: i32 = 500;

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    cost: i32,
    coordinate: CellIndex<u8>,
}

// The priority queue depends on `Ord`.
// Explicitly implement the trait so the queue becomes a min-heap
// instead of a max-heap.
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice that the we flip the ordering on costs.
        // In case of a tie we compare positions - this step is necessary
        // to make implementations of `PartialEq` and `Ord` consistent.
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.coordinate.0.cmp(&other.coordinate.0))
    }
}

// `PartialOrd` needs to be implemented as well.
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn dist_between(a: &Position, b: &Position) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

fn hueristic(start: &CellIndex<u8>, targets: &[CellIndex<u8>]) -> Option<i32> {
    let width = ((11 * 11) as f32).sqrt() as u8;

    targets
        .iter()
        .map(|coor| dist_between(&coor.to_position(width), &start.to_position(width)))
        .min()
}

struct APrimeResult {
    best_cost: i32,
    paths_from: HashMap<CellIndex<u8>, Option<CellIndex<u8>>>,
    best_target: CellIndex<u8>,
}

pub struct APrimeOptions {
    pub food_penalty: i32,
}

fn a_prime_inner(
    board: &battlesnake_game_types::compact_representation::CellBoard4Snakes11x11,
    start: &CellIndex<u8>,
    targets: &[CellIndex<u8>],
    options: Option<APrimeOptions>,
) -> Option<APrimeResult> {
    let options = options.unwrap_or(APrimeOptions { food_penalty: 0 });
    let mut paths_from: HashMap<CellIndex<u8>, Option<CellIndex<u8>>> = HashMap::new();

    if targets.is_empty() {
        return None;
    }

    let mut to_search: BinaryHeap<Node> = BinaryHeap::new();

    let mut known_score: HashMap<CellIndex<u8>, i32> = HashMap::new();

    to_search.push(Node {
        cost: 0,
        coordinate: *start,
    });
    known_score.insert(*start, 0);
    paths_from.insert(*start, None);

    while let Some(Node { cost, coordinate }) = to_search.pop() {
        if targets.contains(&coordinate) {
            return Some(APrimeResult {
                best_cost: cost,
                paths_from,
                best_target: coordinate,
            });
        }

        let width = ((11 * 11) as f32).sqrt() as u8;

        let neighbor_distance = if board.cell_is_hazard(coordinate) {
            HAZARD_PENALTY + NEIGHBOR_DISTANCE
        } else if board.cell_is_food(coordinate) {
            NEIGHBOR_DISTANCE + options.food_penalty
        } else {
            NEIGHBOR_DISTANCE
        };

        fn neighbors(c: &CellIndex<u8>, board: &CellBoard4Snakes11x11) -> Vec<CellIndex<u8>> {
            let width = ((11 * 11) as f32).sqrt() as u8;

            Move::all()
                .into_iter()
                .map(|mv| {
                    let head_pos = c.into_position(width);
                    let new_head = head_pos.add_vec(mv.to_vector());
                    let ci = CellIndex::new(new_head, width);

                    (new_head, ci)
                })
                .filter(|(new_head, ci)| {
                    !board.off_board(*new_head, width) && !board.cell_is_snake_body_piece(*ci)
                })
                .map(|(_, ci)| ci)
                .collect()
        }

        let tentative = known_score.get(&coordinate).unwrap_or(&i32::MAX) + neighbor_distance;
        let neighbors = neighbors(&coordinate, &board);
        for neighbor in neighbors
            .into_iter()
            .filter(|n| targets.contains(n) || !board.cell_is_snake_body_piece(coordinate))
        {
            if &tentative < known_score.get(&neighbor).unwrap_or(&i32::MAX) {
                known_score.insert(neighbor, tentative);
                paths_from.insert(neighbor, Some(coordinate));
                to_search.push(Node {
                    coordinate: neighbor,
                    cost: tentative + hueristic(&neighbor, targets).unwrap_or(HEURISTIC_MAX),
                });
            }
        }
    }

    None
}

pub fn shortest_distance(
    board: &battlesnake_game_types::compact_representation::CellBoard4Snakes11x11,
    start: &CellIndex<u8>,
    targets: &[CellIndex<u8>],
    options: Option<APrimeOptions>,
) -> Option<i32> {
    a_prime_inner(board, start, targets, options).map(|r| r.best_cost)
}

pub fn shortest_path(
    board: &CellBoard4Snakes11x11,
    start: &CellIndex<u8>,
    targets: &[CellIndex<u8>],
    options: Option<APrimeOptions>,
) -> Vec<CellIndex<u8>> {
    let result = a_prime_inner(board, start, targets, options);

    let mut path = vec![];

    if let Some(result) = result {
        let mut current: Option<CellIndex<u8>> = Some(result.best_target);

        while let Some(c) = current {
            path.push(c);

            current = *result.paths_from.get(&c).expect(
                "Somehow we didn't look at this node during a-prime, but its still in the path",
            );
        }
    }

    path.reverse();

    path
}

fn direction_from_coordinate(from: &Position, to: &Position) -> Option<Direction> {
    if from.x == to.x && from.y + 1 == to.y {
        Some(Direction::Up)
    } else if from.x == to.x && from.y - 1 == to.y {
        Some(Direction::Down)
    } else if from.x - 1 == to.x && from.y == to.y {
        Some(Direction::Left)
    } else if from.x + 1 == to.x && from.y == to.y {
        Some(Direction::Right)
    } else {
        None
    }
}

pub fn shortest_path_next_direction(
    board: &CellBoard4Snakes11x11,
    start: &CellIndex<u8>,
    targets: &[CellIndex<u8>],
    options: Option<APrimeOptions>,
) -> Option<Direction> {
    let shortest_path = shortest_path(board, start, targets, options);
    let next_coordinate = shortest_path.get(1);

    if let Some(c) = next_coordinate {
        // direction_from_coordinate(start, c)
        todo!("The above method needs to be re-implemented")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Battlesnake, Coordinate, GameState};
    use battlesnake_game_types::wire_representation::Game;
    use serde_json::Value::Number;

    fn cell_index_from_position_default_width(pos: Position) -> CellIndex<u8> {
        let width = ((11 * 11) as f32).sqrt() as u8;

        CellIndex::new(pos, width)
    }

    #[test]
    fn test_heuristic() {
        assert_eq!(
            hueristic(
                &cell_index_from_position_default_width(Position { x: 1, y: 1 }),
                &[cell_index_from_position_default_width(Position {
                    x: 2,
                    y: 2
                })]
            ),
            Some(2)
        );
    }

    #[test]
    fn test_multi_target_heuristic() {
        assert_eq!(
            hueristic(
                &cell_index_from_position_default_width(Position { x: 1, y: 1 }),
                &[
                    cell_index_from_position_default_width(Position { x: 3, y: 3 }),
                    cell_index_from_position_default_width(Position { x: 4, y: 4 }),
                    cell_index_from_position_default_width(Position { x: 5, y: 5 }),
                ]
            ),
            Some(4)
        );
    }

    #[test]
    fn test_basic_a_prime() {
        let width = ((11 * 11) as f32).sqrt() as u8;

        let json = b"{\"game\":{\"id\":\"4e7c8fe2-a462-4015-95af-5eab3487d5ab\",\"ruleset\":{\"name\":\"royale\",\"version\":\"v1.0.17\"},\"timeout\":500},\"turn\":60,\"board\":{\"height\":11,\"width\":11,\"snakes\":[{\"id\":\"gs_MMxyjByhGFbtGSV8KJv3tqdV\",\"name\":\"\",\"latency\":\"100\",\"health\":86,\"body\":[{\"x\":10,\"y\":4}],\"head\":{\"x\":10,\"y\":4},\"length\":1,\"shout\":\"\"}],\"food\":[],\"hazards\":[]},\"you\":{\"id\":\"gs_MMxyjByhGFbtGSV8KJv3tqdV\",\"name\":\"\",\"latency\":\"100\",\"health\":86,\"body\":[{\"x\":10,\"y\":4}],\"head\":{\"x\":10,\"y\":4},\"length\":1,\"shout\":\"\"}}";
        let game: Game = serde_json::from_slice(json).unwrap();
        let id_map = battlesnake_game_types::types::build_snake_id_map(&game);
        let compact: CellBoard4Snakes11x11 =
            battlesnake_game_types::compact_representation::CellBoard::convert_from_game(
                game, &id_map,
            )
            .unwrap();
        assert_eq!(
            shortest_distance(
                &compact,
                &cell_index_from_position_default_width(Position { x: 1, y: 1 }),
                &[
                    cell_index_from_position_default_width(Position { x: 3, y: 3 }),
                    cell_index_from_position_default_width(Position { x: 4, y: 4 }),
                    cell_index_from_position_default_width(Position { x: 5, y: 5 }),
                ],
                None
            ),
            Some(4)
        );
    }

    #[test]
    fn test_real_example() {
        let board_json = r#"{"game":{"id":"4e7c8fe2-a462-4015-95af-5eab3487d5ab","ruleset":{"name":"royale","version":"v1.0.17"},"timeout":500},"turn":60,"board": {"height":11,"width":11,"food":[],"hazards":[],"snakes":[{"id":"","name":"","health":93,"body":[{"x":7,"y":10},{"x":6,"y":10},{"x":5,"y":10},{"x":4,"y":10}],"latency":84,"head":{"x":7,"y":10},"length":4,"shout":"","squad":""},{"id":"","name":"","health":99,"body":[{"x":5,"y":4},{"x":5,"y":5},{"x":4,"y":5},{"x":3,"y":5},{"x":2,"y":5}],"latency":327,"head":{"x":5,"y":4},"length":4,"shout":"","squad":""}]},"you":{"id":"","name":"","health":99,"body":[{"x":5,"y":4},{"x":5,"y":5},{"x":4,"y":5},{"x":3,"y":5},{"x":2,"y":5}],"latency":327,"head":{"x":5,"y":4},"length":4,"shout":"","squad":""}}"#;
        let game: Game = serde_json::from_str(board_json).unwrap();
        let id_map = battlesnake_game_types::types::build_snake_id_map(&game);
        let compact: CellBoard4Snakes11x11 =
            battlesnake_game_types::compact_representation::CellBoard::convert_from_game(
                game, &id_map,
            )
            .unwrap();

        assert_eq!(
            shortest_distance(
                &compact,
                &cell_index_from_position_default_width(Position { x: 5, y: 4 }),
                &[cell_index_from_position_default_width(Position {
                    x: 7,
                    y: 10
                }),],
                None
            ),
            Some(8)
        );
    }
}
