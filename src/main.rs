#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;

use rocket::http::Status;
use rocket_contrib::json::Json;

#[derive(Serialize)]
struct AboutMe {
    apiversion: String,
    author: Option<String>,
    color: Option<String>,
    head: Option<String>,
    tail: Option<String>,
    version: Option<String>,
}

#[get("/")]
fn me() -> Json<AboutMe> {
    Json(AboutMe {
        apiversion: "1".to_owned(),
        author: Some("coreyja".to_owned()),
        color: Some("#AA66CC".to_owned()),
        head: None,
        tail: None,
        version: None,
    })
}

#[derive(Deserialize, Debug)]
struct Ruleset {
    name: String,
    version: String,
}

#[derive(Deserialize, Debug)]
struct Game {
    id: String,
    ruleset: Option<Ruleset>,
    timeout: u64,
}

#[derive(Deserialize, Debug)]
struct Coordinate {
    x: u64,
    y: u64,
}

#[derive(Deserialize, Debug)]
struct Battlesnake {
    id: String,
    name: String,
    health: u8,
    body: Vec<Coordinate>,
    latency: u16,
    head: Coordinate,
    length: u64,
    shout: Option<String>,
    squad: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Board {
    height: u64,
    width: u64,
    food: Vec<Coordinate>,
    hazards: Vec<Coordinate>,
    snakes: Vec<Battlesnake>,
}

#[derive(Deserialize, Debug)]
struct GameState {
    game: Game,
    turn: u64,
    board: Board,
    you: Battlesnake,
}

#[post("/start", data = "<game_state>")]
fn start(game_state: Json<GameState>) -> Status {
    println!("{:?}", game_state);
    Status::NoContent
}

#[derive(Serialize, Debug)]
struct MoveOutput {
    r#move: String,
    shout: String,
}

#[post("/move", data = "<game_state>")]
fn api_move(game_state: Json<GameState>) -> Json<MoveOutput> {
    println!("{:?}", game_state);
    Json(MoveOutput {
        r#move: "down".to_owned(),
        shout: "".to_owned(),
    })
}

fn main() {
    rocket::ignite()
        .mount("/", routes![me, start, api_move])
        .launch();
}
