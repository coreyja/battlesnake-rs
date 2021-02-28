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

fn score(game_state: &GameState, coor: &Coordinate, times_to_recurse: u8) -> i64 {
    const PREFERRED_HEALTH: i64 = 80;

    if game_state.you.body.contains(coor) {
        return 0;
    }

    if game_state.you.health == 0 {
        return 0;
    }

    if game_state
        .board
        .snakes
        .iter()
        .any(|x| x.body.contains(coor))
    {
        return 0;
    }

    let ihealth: i64 = game_state.you.health.into();
    let current_score: i64 = (ihealth - PREFERRED_HEALTH).abs().into();
    let current_score = PREFERRED_HEALTH - current_score;

    if times_to_recurse == 0 {
        return current_score;
    }

    let recursed_score: i64 = coor
        .possbile_moves(&game_state.board)
        .iter()
        .map(|(_d, c)| score(&game_state.move_to(coor), &c, times_to_recurse - 1))
        .sum();

    current_score + recursed_score / 2
}

#[post("/move", data = "<game_state>")]
pub fn api_move(game_state: Json<GameState>) -> Json<MoveOutput> {
    let possible = game_state.you.possbile_moves(&game_state.board);
    let recursion_limit: u8 = match std::env::var("RECURSION_LIMIT").map(|x| x.parse()) {
        Ok(Ok(x)) => x,
        _ => 5,
    };
    let next_move = possible
        .iter()
        .max_by_key(|(_dir, coor)| score(&game_state, &coor, recursion_limit));

    let stuck_response: MoveOutput = MoveOutput {
        r#move: Direction::UP.value(),
        shout: Some("Oh NO we are stuck".to_owned()),
    };
    let output = next_move.map_or(stuck_response, |(dir, _coor)| MoveOutput {
        r#move: dir.value(),
        shout: None,
    });
    Json(output)
}
