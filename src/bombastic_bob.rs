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

#[post("/move", data = "<_game_state>")]
pub fn api_move(_game_state: Json<GameState>) -> Json<MoveOutput> {
    let chosen_direction = ALL_DIRECTIONS.choose(&mut rand::thread_rng()).unwrap();

    Json(MoveOutput {
        r#move: chosen_direction.value(),
        shout: None,
    })
}
