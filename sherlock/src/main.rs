use std::error::Error;

use battlesnake_game_types::wire_representation::{
    BattleSnake, Board, Game, NestedGame, Position, Ruleset,
};
use serde_json::Value;

fn frame_to_nested_game(game: &Value) -> Result<NestedGame, &'static str> {
    let id = game["ID"].as_str().ok_or("Missing Game ID")?.to_string();

    let map = game["Map"].as_str().map(|x| x.to_string());
    let source = game["Source"].as_str().map(|x| x.to_string());

    let timeout = game["SnakeTimeout"].as_i64().ok_or("Missing Timeout")?;

    let ruleset_name = game["Ruleset"]["name"]
        .as_str()
        .ok_or("Missing Ruleset Name")?
        .to_string();
    let ruleset_version = "No version in frames".to_string();

    let ruleset = Ruleset {
        name: ruleset_name,
        version: ruleset_version,
        settings: None,
    };

    Ok(NestedGame {
        id,
        map,
        source,
        timeout,
        ruleset,
    })
}

fn value_to_position_vec(value: &Value) -> Result<Vec<Position>, &'static str> {
    value
        .as_array()
        .ok_or("Not an array")?
        .iter()
        .map(|pos_arr| {
            let x = pos_arr["X"]
                .as_i64()
                .ok_or("X is not an integer")?
                .try_into()
                .map_err(|_| "Too big for an i32")?;

            let y = pos_arr["Y"]
                .as_i64()
                .ok_or("Y is not an integer")?
                .try_into()
                .map_err(|_| "Too big for an i32")?;

            Ok(Position { x, y })
        })
        .collect()
}

fn value_to_snake(value: &Value) -> Result<BattleSnake, &'static str> {
    let id = value["ID"].as_str().ok_or("Missing ID")?.to_string();
    let name = value["Name"].as_str().ok_or("Missing Name")?.to_string();
    let body = value_to_position_vec(&value["Body"])?;
    let head = body[0];
    let health = value["Health"]
        .as_i64()
        .ok_or("Missing Health")?
        .try_into()
        .map_err(|_| "Health is too big for an i32")?;
    let shout = value["Shout"].as_str().map(|x| x.to_string());
    let length = body.len() as i32;

    Ok(BattleSnake {
        id,
        name,
        head,
        body: body.into(),
        health,
        shout,
        actual_length: Some(length),
    })
}

fn frame_to_board(frame: &Value, game: &Value) -> Result<Board, &'static str> {
    let height = game["Height"]
        .as_i64()
        .ok_or("Missing Height")?
        .try_into()
        .map_err(|_| "Height doesn't fit in a u32")?;

    let width = game["Width"]
        .as_i64()
        .ok_or("Missing Width")?
        .try_into()
        .map_err(|_| "Width doesn't fit in a u32")?;

    let snakes = frame["Snakes"]
        .as_array()
        .ok_or("Missing Snakes")?
        .iter()
        .map(value_to_snake)
        .collect::<Result<Vec<BattleSnake>, &'static str>>()?;

    Ok(Board {
        height,
        width,
        food: value_to_position_vec(&frame["Food"])?,
        hazards: value_to_position_vec(&frame["Hazards"])?,
        snakes,
    })
}

fn frame_to_game(frame: &Value, game: &Value, you_name: &str) -> Result<Game, &'static str> {
    let turn = frame["Turn"]
        .as_i64()
        .ok_or("Turn is not an integer")?
        .try_into()
        .map_err(|_| "Turn is too big for an i32")?;

    let nested_game = frame_to_nested_game(game)?;

    let board = frame_to_board(frame, game)?;

    let you = board
        .snakes
        .iter()
        .find(|snake| snake.name == you_name)
        .ok_or("You are not in the game")?
        .clone();

    Ok(Game {
        turn,
        game: nested_game,
        board,
        you,
    })
}

fn main() -> Result<(), ureq::Error> {
    let body: Value =
        ureq::get("https://engine.battlesnake.com/games/bee9e1d4-9a95-4516-be42-61fcc7482430")
            .set("Example-Header", "header value")
            .call()?
            .into_json()?;

    let last_frame = &body["LastFrame"];

    println!("Ending Turn {}", &last_frame["Turn"]);

    dbg!(frame_to_game(last_frame, &body["Game"], "Ziggy Snakedust").unwrap());

    Ok(())
}
