use super::*;

use debug_print::debug_println;

pub struct DeviousDevin {}

#[derive(Serialize)]
pub struct MoveOption {
    moves: Vec<SnakeMove>,
    score: i64,
}

#[derive(Serialize)]
pub struct EvaluateOutput {
    options: Vec<MoveOption>,
}

impl DeviousDevin {
    pub fn explain_move(
        &self,
        game_state: GameState,
    ) -> Result<EvaluateOutput, Box<dyn std::error::Error + Send + Sync>> {
        let mut game_state = game_state;
        let options = minimax_options(&mut game_state, 0, true, i64::MIN, i64::MAX, vec![]);

        let options: Vec<MoveOption> = options
            .into_iter()
            .map(|(score, moves)| MoveOption { score, moves })
            .collect();

        Ok(EvaluateOutput { options })
    }
}

impl BattlesnakeAI for DeviousDevin {
    fn make_move(
        &self,
        game_state: GameState,
    ) -> Result<MoveOutput, Box<dyn std::error::Error + Send + Sync>> {
        let mut game_state = game_state;
        let (score, moves) = minimax(&mut game_state, 0, true, i64::MIN, i64::MAX, vec![]);
        debug_println!(
            "Turn: {} Score: {} Dir: {:?}",
            game_state.turn,
            score,
            moves
        );

        Ok(MoveOutput {
            r#move: moves.get(0).unwrap().dir.value(),
            shout: None,
        })
    }

    fn name(&self) -> String {
        "devious-devin".to_owned()
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            apiversion: "1".to_owned(),
            author: Some("coreyja".to_owned()),
            color: Some("#99cc00".to_owned()),
            head: Some("snail".to_owned()),
            tail: Some("rbc-necktie".to_owned()),
            version: None,
        }
    }
}

const MAX_DEPTH: i64 = 14;

fn score(node: &GameState, depth: i64) -> Option<i64> {
    let me: &Battlesnake = node
        .board
        .snakes
        .iter()
        .find(|s| s.id == node.you.id)
        .unwrap();
    let not_me = node
        .board
        .snakes
        .iter()
        .find(|s| s.id != node.you.id)
        .unwrap();

    use std::iter::FromIterator;
    let my_body: HashSet<Coordinate> = HashSet::from_iter(me.body.iter().cloned());
    let other_body: HashSet<Coordinate> = HashSet::from_iter(not_me.body.iter().cloned());

    if other_body.contains(&me.body[0]) {
        return Some(SCORE_LOSE + depth);
    }

    if me.body[1..].contains(&me.body[0]) && depth != 0 {
        return Some(SCORE_LOSE + depth);
    }

    if my_body.contains(&not_me.body[0]) {
        return Some(SCORE_WIN - depth);
    }

    if not_me.body[1..].contains(&not_me.body[0]) && depth != 0 {
        return Some(SCORE_WIN - depth);
    }

    let num_snakes: i64 = node.board.snakes.len().try_into().unwrap();
    if depth % num_snakes != 0 {
        return None;
    }

    if me.body[0] == not_me.body[0] {
        if me.length > not_me.length {
            return Some(SCORE_WIN - depth);
        } else {
            return Some(SCORE_LOSE + depth);
        }
    }

    if depth == MAX_DEPTH {
        if not_me.body.len() + 4 > me.body.len() {
            let l: i64 = me.body.len().try_into().unwrap();
            return Some(
                (l + 1000)
                    - a_prime::shortest_distance(&node.board, &me.body[0], &node.board.food)
                        .unwrap_or(500),
            );
        }

        return Some(
            2000 - a_prime::shortest_distance(&node.board, &me.body[0], &vec![not_me.body[0]])
                .unwrap_or(500),
        );
    }

    None
}

const SCORE_LOSE: i64 = -10000;
const SCORE_WIN: i64 = 10000;

fn children(node: &GameState, turn_snake_id: &str) -> Vec<(Direction, Coordinate)> {
    let you: &Battlesnake = node
        .board
        .snakes
        .iter()
        .find(|s| s.id == turn_snake_id)
        .expect("We didn't find that snake");
    you.body[0]
        .possbile_moves(&node.board)
        .iter()
        .cloned()
        .collect()
}
use std::convert::TryInto;

fn minimax(
    node: &mut GameState,
    depth: usize,
    is_maximizing: bool,
    alpha: i64,
    beta: i64,
    current_moves: Vec<SnakeMove>,
) -> (i64, Vec<SnakeMove>) {
    let options = minimax_options(node, depth, is_maximizing, alpha, beta, current_moves);

    let score_multiplier = if is_maximizing { 1 } else { -1 };
    options
        .into_iter()
        .max_by_key(|(score, _)| score_multiplier * score.clone())
        .unwrap_or((0, vec![]))
}

#[derive(Clone, Debug, Serialize)]
struct SnakeMove {
    snake_name: String,
    snake_id: String,
    dir: Direction,
    move_to: Coordinate,
}

fn minimax_options(
    node: &mut GameState,
    depth: usize,
    is_maximizing: bool,
    alpha: i64,
    beta: i64,
    current_moves: Vec<SnakeMove>,
) -> Vec<(i64, Vec<SnakeMove>)> {
    let mut alpha = alpha;
    let mut beta = beta;

    let new_depth = depth.try_into().unwrap();
    if let Some(s) = score(&node, new_depth) {
        return vec![(s, current_moves)];
    }

    let mut options: Vec<(i64, Vec<SnakeMove>)> = vec![];

    if is_maximizing {
        let me = node.you.id.to_owned();
        for (dir, coor) in children(node, &node.you.id).into_iter() {
            let last_move = node.move_to(&coor, &me);
            let new_current_moves = {
                let mut x = current_moves.clone();
                x.push(SnakeMove {
                    dir,
                    snake_name: node.you.name.clone(),
                    snake_id: node.you.id.clone(),
                    move_to: coor.clone(),
                });
                x
            };
            let next_move_return = minimax(node, depth + 1, false, alpha, beta, new_current_moves);
            let value = next_move_return.0;
            node.reverse_move(last_move);

            options.push(next_move_return);
            alpha = std::cmp::max(alpha, value);
            if beta <= alpha {
                break;
            }
        }
    } else {
        let not_me = node
            .board
            .snakes
            .iter()
            .cloned()
            .filter(|s| s.id != node.you.id)
            .next()
            .unwrap();
        for (dir, coor) in children(node, &not_me.id).into_iter() {
            let last_move = node.move_to(&coor, &not_me.id);
            let new_current_moves = {
                let mut x = current_moves.clone();
                x.push(SnakeMove {
                    dir,
                    snake_name: not_me.name.clone(),
                    snake_id: not_me.id.clone(),
                    move_to: coor.clone(),
                });
                x
            };
            let next_move_return = minimax(node, depth + 1, true, alpha, beta, new_current_moves);
            let value = next_move_return.0;
            node.reverse_move(last_move);

            options.push(next_move_return);
            beta = std::cmp::min(beta, value);
            if beta <= alpha {
                break;
            }
        }
    }

    options
}

mod a_prime {
    use crate::{Board, Coordinate};
    use std::cmp::Ordering;
    use std::collections::{BinaryHeap, HashMap};

    const NEIGHBOR_DISTANCE: i64 = 1;
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

    pub fn shortest_distance(
        board: &Board,
        start: &Coordinate,
        targets: &Vec<Coordinate>,
    ) -> Option<i64> {
        if targets.len() == 0 {
            return Some(0);
        }

        let mut to_search: BinaryHeap<Node> = BinaryHeap::new();

        let mut known_score: HashMap<Coordinate, i64> = HashMap::new();

        to_search.push(Node {
            cost: 0,
            coordinate: start.clone(),
        });
        known_score.insert(start.clone(), 0);

        let mut count = 0;

        while let Some(Node { cost, coordinate }) = to_search.pop() {
            count = count + 1;
            if targets.contains(&coordinate) {
                return Some(cost);
            }

            let tentative = known_score.get(&coordinate).unwrap_or(&i64::MAX) + NEIGHBOR_DISTANCE;
            let neighbors = coordinate.possbile_moves(&board);
            for (_, neighbor) in neighbors.iter().filter(|(_, n)| {
                // true
                board.snakes.iter().all(|snake| !snake.body.contains(n))
            }) {
                if &tentative < known_score.get(&neighbor).unwrap_or(&i64::MAX) {
                    known_score.insert(neighbor.clone(), tentative);
                    to_search.push(Node {
                        coordinate: neighbor.clone(),
                        cost: tentative + hueristic(neighbor, &targets).unwrap_or(HEURISTIC_MAX),
                    });
                }
            }
        }

        None
    }

    #[cfg(test)]
    mod tests {
        // Note this useful idiom: importing names from outer (for mod tests) scope.
        use super::*;

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
    }
}
