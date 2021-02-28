use rocket_contrib::json::Json;

use super::*;

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

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

impl Distribution<Direction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0..=3) {
            0 => Direction::UP,
            1 => Direction::DOWN,
            2 => Direction::RIGHT,
            _ => Direction::LEFT,
        }
    }
}

#[post("/move", data = "<game_state>")]
pub fn api_move(game_state: Json<GameState>) -> Json<MoveOutput> {
    let chosen_direction: Direction = rand::random();

    Json(MoveOutput {
        r#move: chosen_direction.value(),
        shout: None,
    })
}
