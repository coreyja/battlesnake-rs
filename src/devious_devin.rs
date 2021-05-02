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
        head: Some("snail".to_owned()),
        tail: Some("rbc-necktie".to_owned()),
        version: None,
    })
}

const MAX_DEPTH: i64 = 16;

fn score(node: &GameState, depth: i64, current_moves: &Vec<Direction>) -> Option<i64> {
    // let print_end_state = |msg: &str| {
    //     println!(
    //         "Turn {} Depth: {} Msg: {}\n{:?}",
    //         node.turn, depth, msg, current_moves
    //     );
    // };

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
        // print_end_state("lost cause hit other snake or ran off board");
        return Some(SCORE_LOSE + depth);
    }

    if me.body[1..].contains(&me.body[0]) && depth != 0 {
        // print_end_state(&format!("lost cause hit self Body: {:?}", me.body));
        return Some(SCORE_LOSE + depth);
    }

    if me.health == 0 {
        return Some(SCORE_LOSE + depth);
    }

    let num_snakes: i64 = node.board.snakes.len().try_into().unwrap();
    if depth % num_snakes != 0 {
        return None;
    }

    if me.body[0] == not_me.body[0] {
        if me.length > not_me.length {
            // print_end_state("head to head win");
            return Some(SCORE_WIN - depth);
        } else {
            // print_end_state("head to head lose or draw");
            return Some(SCORE_LOSE + depth);
        }
    }

    if not_me
        .body
        .iter()
        .any(|c| !c.valid(&node.board) || my_body.contains(c))
    {
        // print_end_state("other snake off board or hit me");
        return Some(SCORE_WIN - depth);
    }

    if not_me.body[1..].contains(&not_me.body[0]) && depth != 0 {
        // print_end_state("other snake hit itself");
        return Some(SCORE_WIN - depth);
    }

    if not_me.health == 0 {
        return Some(SCORE_WIN - depth);
    }

    if depth == MAX_DEPTH {
        // let h: (i64, i64) = (me.health.into(), not_me.health.into());
        // print_end_state("Made it to max depth");
        let me_length: i64 = me.body.len().try_into().unwrap();
        let other_length: i64 = not_me.body.len().try_into().unwrap();
        return Some(me_length - other_length + depth);
    }

    None
}

const SCORE_LOSE: i64 = -200;
const SCORE_WIN: i64 = 200;

fn children(node: &GameState, turn_snake_id: &str) -> Vec<(Direction, GameState)> {
    let you: &Battlesnake = node
        .board
        .snakes
        .iter()
        .find(|s| s.id == turn_snake_id)
        .expect("We didn't find that snake");
    you.body[0]
        .possbile_moves(&node.board)
        .map(|(dir, coor)| (dir.clone(), node.move_to(&coor, turn_snake_id)))
        .collect()
}
use std::convert::TryInto;

fn minimax(
    node: &GameState,
    depth: usize,
    is_maximizing: bool,
    alpha: i64,
    beta: i64,
    current_moves: Vec<Direction>,
) -> (i64, Option<Direction>) {
    let mut alpha = alpha;
    let mut beta = beta;

    let new_depth = depth.try_into().unwrap();
    if let Some(s) = score(&node, new_depth, &current_moves) {
        return (s, None);
    }

    if is_maximizing {
        let mut best = (i64::MIN, None);

        for (dir, child) in children(node, &node.you.id).into_iter() {
            let new_current_moves = {
                let mut x = current_moves.clone();
                x.push(dir);
                x
            };
            let value = minimax(&child, depth + 1, false, alpha, beta, new_current_moves).0;

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
            let new_current_moves = {
                let mut x = current_moves.clone();
                x.push(dir);
                x
            };
            let value = minimax(&child, depth + 1, true, alpha, beta, new_current_moves).0;

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
    let (_score, dir) = minimax(&game_state, 0, true, i64::MIN, i64::MAX, vec![]);

    Json(MoveOutput {
        r#move: dir.unwrap().value(),
        shout: None,
    })
}
