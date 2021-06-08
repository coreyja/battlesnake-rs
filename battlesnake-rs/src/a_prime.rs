use crate::{Board, Coordinate};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

const NEIGHBOR_DISTANCE: i64 = 1;
const HAZARD_PENALTY: i64 = 1;
const HEURISTIC_MAX: i64 = 500;

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    cost: i64,
    coordinate: Coordinate,
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
            .then_with(|| self.coordinate.x.cmp(&other.coordinate.x))
            .then_with(|| self.coordinate.y.cmp(&other.coordinate.y))
    }
}

// `PartialOrd` needs to be implemented as well.
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn hueristic(start: &Coordinate, targets: &Vec<Coordinate>) -> Option<i64> {
    targets.iter().map(|coor| start.dist_from(coor)).min()
}

struct APrimeResult {
    best_cost: i64,
    paths_from: HashMap<Coordinate, Option<Coordinate>>,
    best_target: Coordinate,
}

fn a_prime_inner(
    board: &Board,
    start: &Coordinate,
    targets: &Vec<Coordinate>,
) -> Option<APrimeResult> {
    let mut paths_from: HashMap<Coordinate, Option<Coordinate>> = HashMap::new();

    if targets.len() == 0 {
        return None;
    }

    let mut to_search: BinaryHeap<Node> = BinaryHeap::new();

    let mut known_score: HashMap<Coordinate, i64> = HashMap::new();

    to_search.push(Node {
        cost: 0,
        coordinate: start.clone(),
    });
    known_score.insert(start.clone(), 0);
    paths_from.insert(start.clone(), None);

    while let Some(Node { cost, coordinate }) = to_search.pop() {
        if targets.contains(&coordinate) {
            return Some(APrimeResult {
                best_cost: cost,
                paths_from,
                best_target: coordinate,
            });
        }

        let neighbor_distance = if board.hazards.contains(&coordinate) {
            HAZARD_PENALTY + NEIGHBOR_DISTANCE
        } else {
            NEIGHBOR_DISTANCE
        };

        let tentative = known_score.get(&coordinate).unwrap_or(&i64::MAX) + neighbor_distance;
        let neighbors = coordinate.possible_moves(&board);
        for (_, neighbor) in neighbors.iter().filter(|(_, n)| {
            // true
            board
                .snakes
                .iter()
                .all(|snake| !snake.body.contains(n) || targets.contains(n))
        }) {
            if &tentative < known_score.get(&neighbor).unwrap_or(&i64::MAX) {
                known_score.insert(neighbor.clone(), tentative);
                paths_from.insert(neighbor.clone(), Some(coordinate));
                to_search.push(Node {
                    coordinate: neighbor.clone(),
                    cost: tentative + hueristic(neighbor, &targets).unwrap_or(HEURISTIC_MAX),
                });
            }
        }
    }

    None
}

pub fn shortest_distance(
    board: &Board,
    start: &Coordinate,
    targets: &Vec<Coordinate>,
) -> Option<i64> {
    a_prime_inner(board, start, targets).map(|r| r.best_cost)
}

pub fn shortest_path(
    board: &Board,
    start: &Coordinate,
    targets: &Vec<Coordinate>,
) -> Vec<Coordinate> {
    let result = a_prime_inner(board, start, targets);

    let mut path = vec![];

    if let Some(result) = result {
        let mut current: Option<Coordinate> = Some(result.best_target);

        while let Some(c) = current {
            path.push(c);

            current = result.paths_from.get(&c).unwrap().clone();
        }
    }

    path.reverse();

    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Battlesnake;
    use serde_json::Value::Number;

    #[test]
    fn test_heuristic() {
        assert_eq!(
            hueristic(&Coordinate { x: 1, y: 1 }, &vec![Coordinate { x: 2, y: 2 }]),
            Some(2)
        );
    }

    #[test]
    fn test_multi_target_heuristic() {
        assert_eq!(
            hueristic(
                &Coordinate { x: 1, y: 1 },
                &vec![
                    Coordinate { x: 3, y: 3 },
                    Coordinate { x: 4, y: 4 },
                    Coordinate { x: 5, y: 5 },
                ]
            ),
            Some(4)
        );
    }

    #[test]
    fn test_basic_a_prime() {
        assert_eq!(
            shortest_distance(
                &Board {
                    food: vec![],
                    hazards: vec![],
                    height: 11,
                    width: 11,
                    snakes: vec![],
                },
                &Coordinate { x: 1, y: 1 },
                &vec![
                    Coordinate { x: 3, y: 3 },
                    Coordinate { x: 4, y: 4 },
                    Coordinate { x: 5, y: 5 },
                ]
            ),
            Some(4)
        );
    }

    #[test]
    fn test_real_example() {
        assert_eq!(
            shortest_distance(
                &Board {
                    food: vec![],
                    hazards: vec![],
                    height: 11,
                    width: 11,
                    snakes: vec![
                        Battlesnake {
                            id: "".to_owned(),
                            name: "".to_owned(),
                            health: 93,
                            body: vec![
                                Coordinate { x: 7, y: 10 },
                                Coordinate { x: 6, y: 10 },
                                Coordinate { x: 5, y: 10 },
                                Coordinate { x: 4, y: 10 }
                            ],
                            latency: Number(84.into()),
                            head: Coordinate { x: 0, y: 10 },
                            length: 4,
                            shout: Some("".to_owned()),
                            squad: Some("".to_owned())
                        },
                        Battlesnake {
                            id: "".to_owned(),
                            name: "".to_owned(),
                            health: 99,
                            body: vec![
                                Coordinate { x: 5, y: 4 },
                                Coordinate { x: 5, y: 5 },
                                Coordinate { x: 4, y: 5 },
                                Coordinate { x: 3, y: 5 },
                                Coordinate { x: 2, y: 5 }
                            ],
                            latency: Number(327.into()),
                            head: Coordinate { x: 2, y: 4 },
                            length: 4,
                            shout: Some("".to_owned()),
                            squad: Some("".to_owned())
                        }
                    ],
                },
                &Coordinate { x: 5, y: 4 },
                &vec![Coordinate { x: 7, y: 10 },]
            ),
            Some(8)
        );
    }
}
