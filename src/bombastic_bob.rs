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
use rand::seq::SliceRandom;
use std::iter::FromIterator;

#[post("/move", data = "<game_state>")]
pub fn api_move(game_state: Json<GameState>) -> Json<MoveOutput> {
    let body_set: HashSet<Coordinate> = HashSet::from_iter(game_state.you.body.iter().cloned());
    let possible_moves = game_state
        .you
        .head
        .possbile_moves(&game_state.board)
        .iter()
        .filter(|(_dir, coor)| !body_set.contains(coor))
        .cloned()
        .collect::<Vec<_>>();
    let chosen = possible_moves.choose(&mut rand::thread_rng());
    let dir = match chosen {
        Some(x) => x.0,
        _ => Direction::DOWN,
    };

    Json(MoveOutput {
        r#move: dir.value(),
        shout: None,
    })
}
