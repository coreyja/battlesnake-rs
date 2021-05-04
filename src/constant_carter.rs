use rocket_contrib::json::Json;

use super::*;

trait Battlesnake {
    fn start(&self) {}
    fn end(&self) {}
    fn make_move(&self) -> Result<MoveOutput, Box<dyn std::error::Error>>;
    fn name() -> String;

    fn about(&self) -> AboutMe {
        Default::default()
    }
}

struct ConstantCarter {}

impl Battlesnake for ConstantCarter {
    fn name() -> String {
        "constant-carter".to_owned()
    }

    fn about(&self) -> AboutMe {
        AboutMe {
            author: Some("coreyja".to_owned()),
            color: Some("#AA66CC".to_owned()),
            ..Default::default()
        }
    }

    fn make_move(&self) -> Result<MoveOutput, Box<dyn std::error::Error>> {
        Ok(MoveOutput {
            r#move: Direction::DOWN.value(),
            shout: None,
        })
    }
}

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

#[post("/move")]
pub fn api_move() -> Json<MoveOutput> {
    Json(MoveOutput {
        r#move: Direction::DOWN.value(),
        shout: None,
    })
}
