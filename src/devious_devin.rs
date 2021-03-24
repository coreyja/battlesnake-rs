use rocket_contrib::json::Json;

use super::*;

#[post("/start")]
pub fn start() -> Status {
    Status::NoContent
}

#[post("/end")]
pub fn end() -> Status {
    Status::NoContent
}

#[get("/")]
pub fn me() -> Json<AboutMe> {
    Json(AboutMe {
        apiversion: "1".to_owned(),
        author: Some("coreyja".to_owned()),
        color: Some("#99cc00".to_owned()),
        head: None,
        tail: None,
        version: None,
    })
}

const MAX_DEPTH: i64 = 16;

fn score(node: &GameState, depth: i64) -> Option<i64> {
    println!("Depth: {}", depth);
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

    if me
        .body
        .iter()
        .any(|c| !c.valid(&node.board) || other_body.contains(c))
    {
        return Some(SCORE_LOSE);
    }

    if me.body[1..].contains(&me.body[0]) && depth != 0 {
        return Some(SCORE_LOSE);
    }

    let num_snakes: i64 = node.board.snakes.len().try_into().unwrap();
    if depth % num_snakes != 0 {
        return None;
    }

    if me.body[0] == not_me.body[0] {
        if me.length > not_me.length {
            return Some(SCORE_WIN);
        } else {
            return Some(SCORE_LOSE);
        }
    }

    if not_me
        .body
        .iter()
        .any(|c| !c.valid(&node.board) || my_body.contains(c))
    {
        return Some(SCORE_WIN);
    }

    if not_me.body[1..].contains(&not_me.body[0]) && depth != 0 {
        return Some(SCORE_WIN);
    }

    if depth == MAX_DEPTH {
        let h: i64 = me.health.into();
        return Some(h);
    }

    None
}

const SCORE_LOSE: i64 = -5;
const SCORE_WIN: i64 = 5;

fn children(node: &GameState, turn_snake_id: &str) -> Vec<(Direction, GameState)> {
    node.you
        .head
        .possbile_moves(&node.board)
        .iter()
        .map(|(dir, coor)| (dir.clone(), node.move_to(coor, turn_snake_id)))
        .collect()
}
use std::convert::TryInto;

fn minimax(
    node: &GameState,
    depth: usize,
    is_maximizing: bool,
    alpha: i64,
    beta: i64,
) -> (i64, Option<Direction>) {
    let mut alpha = alpha;
    let mut beta = beta;

    let new_depth = depth.try_into().unwrap();
    if let Some(s) = score(&node, new_depth) {
        return (s + new_depth, None);
    }

    if is_maximizing {
        let mut best = (i64::MIN, None);

        for (dir, child) in children(node, &node.you.id).into_iter() {
            let value = minimax(&child, depth + 1, false, alpha, beta).0;
            if depth == 0 {
                println! {"Top Level Dir: {:?} Score: {}", dir, value};
            }

            if value > best.0 {
                best = (value, Some(dir));
            }
            alpha = std::cmp::max(alpha, best.0);
            if beta <= alpha {
                break;
            }
        }

        best
    } else {
        let mut best = (i64::MAX, None);

        let not_me = node
            .board
            .snakes
            .iter()
            .filter(|s| s.id != node.you.id)
            .next()
            .unwrap();
        for (dir, child) in children(node, &not_me.id).into_iter() {
            let value = minimax(&child, depth + 1, true, alpha, beta).0;

            if value < best.0 {
                best = (value, Some(dir));
            }
            beta = std::cmp::min(beta, best.0);
            if beta <= alpha {
                break;
            }
        }

        best
    }
}

#[post("/move", data = "<game_state>")]
pub fn api_move(game_state: Json<GameState>) -> Json<MoveOutput> {
    let (score, dir) = minimax(&game_state, 0, true, i64::MIN, i64::MAX);
    println!("Score: {} Dir: {:?}", score, dir);

    Json(MoveOutput {
        r#move: dir.unwrap().value(),
        shout: None,
    })
}
