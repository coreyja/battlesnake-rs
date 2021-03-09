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
        color: Some("#AA66CC".to_owned()),
        head: None,
        tail: None,
        version: None,
    })
}

fn isTerminal(node: &GameState) -> bool {
    false
}

fn score(node: &GameState) -> i64 {
    0
}

fn children(node: &GameState) -> Vec<GameState> {
    [].into()
}

fn minimax(node: &GameState, depth: usize, is_maximizing: bool, alpha: i64, beta: i64) -> i64 {
    let mut alpha = alpha;
    let mut beta = beta;

    if isTerminal(&node) {
        return score(&node);
    }

    if is_maximizing {
        let mut best = i64::MIN;

        for child in children(node).iter() {
            let value = minimax(child, depth + 1, false, alpha, beta);

            best = std::cmp::max(best, value);
            alpha = std::cmp::max(alpha, best);
            if beta <= alpha {
                break;
            }
        }

        best
    } else {
        let mut best = i64::MAX;

        for child in children(node).iter() {
            let value = minimax(child, depth + 1, true, alpha, beta);

            best = std::cmp::min(best, value);
            beta = std::cmp::min(beta, best);
            if beta <= alpha {
                break;
            }
        }

        best
    }
}

#[post("/move", data = "<game_state>")]
pub fn api_move(game_state: Json<GameState>) -> Json<MoveOutput> {
    Json(MoveOutput {
        r#move: Direction::DOWN.value(),
        shout: None,
    })
}
