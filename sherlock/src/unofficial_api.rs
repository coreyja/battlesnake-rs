use color_eyre::eyre::{eyre, Result, WrapErr};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use battlesnake_game_types::wire_representation::{
    BattleSnake, Board, Game, NestedGame, Position, Ruleset, Settings,
};

fn frame_to_nested_game(game: &Value) -> Result<NestedGame> {
    let id = game["ID"]
        .as_str()
        .ok_or_else(|| eyre!("Missing Game ID"))?
        .to_string();

    let map = game["Map"].as_str().map(|x| x.to_string());
    let source = game["Source"].as_str().map(|x| x.to_string());

    let timeout = game["SnakeTimeout"]
        .as_i64()
        .ok_or_else(|| eyre!("Missing Timeout"))?;

    let ruleset_name = game["Ruleset"]["name"]
        .as_str()
        .ok_or_else(|| eyre!("Missing Ruleset Name"))?
        .to_string();
    let ruleset_version = "No version in frames".to_string();

    let settings = Settings {
        food_spawn_chance: game["Ruleset"]["foodSpawnChance"]
            .as_str()
            .ok_or_else(|| eyre!("Missing Food Spawn Chance"))?
            .parse()
            .wrap_err("Too big for an i32")?,
        minimum_food: game["Ruleset"]["minimumFood"]
            .as_str()
            .ok_or_else(|| eyre!("Missing minimumFood"))?
            .parse()
            .wrap_err("Too big for an i32")?,
        hazard_damage_per_turn: game["Ruleset"]["damagePerTurn"]
            .as_str()
            .ok_or_else(|| eyre!("Missing damagePerTurn"))?
            .parse()
            .wrap_err("Too big for an i32")?,
        hazard_map: None,
        hazard_map_author: None,
        royale: None,
    };

    let ruleset = Ruleset {
        name: ruleset_name,
        version: ruleset_version,
        settings: Some(settings),
    };

    Ok(NestedGame {
        id,
        map,
        source,
        timeout,
        ruleset,
    })
}

fn value_to_position_vec(value: &Value) -> Result<Vec<Position>> {
    value
        .as_array()
        .ok_or_else(|| eyre!("Not an array"))?
        .iter()
        .map(|pos| {
            let x = pos["X"]
                .as_i64()
                .ok_or_else(|| eyre!("X is not an integer"))?
                .try_into()
                .wrap_err("Too big for an i32")?;

            let y = pos["Y"]
                .as_i64()
                .ok_or_else(|| eyre!("Y is not an integer"))?
                .try_into()
                .wrap_err("Too big for an i32")?;

            Ok(Position { x, y })
        })
        .collect()
}

fn value_to_snake(value: &Value) -> Result<BattleSnake> {
    let id = value["ID"]
        .as_str()
        .ok_or_else(|| eyre!("Missing ID"))?
        .to_string();
    let name = value["Name"]
        .as_str()
        .ok_or_else(|| eyre!("Missing Name"))?
        .to_string();
    let body = value_to_position_vec(&value["Body"])?;
    let head = body[0];
    let health = value["Health"]
        .as_i64()
        .ok_or_else(|| eyre!("Missing Health"))?
        .try_into()
        .wrap_err("Health is too big for an i32")?;
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

fn frame_to_board(frame: &Value, game: &Value) -> Result<Board> {
    let height = game["Height"]
        .as_i64()
        .ok_or_else(|| eyre!("Missing Height"))?
        .try_into()
        .wrap_err("Height doesn't fit in a u32")?;

    let width = game["Width"]
        .as_i64()
        .ok_or_else(|| eyre!("Missing Width"))?
        .try_into()
        .wrap_err("Width doesn't fit in a u32")?;

    let snakes = frame["Snakes"]
        .as_array()
        .ok_or_else(|| eyre!("Missing Snakes"))?
        .iter()
        .filter(|snake_json| snake_json["Death"].is_null())
        .map(value_to_snake)
        .collect::<Result<Vec<BattleSnake>>>()?;

    Ok(Board {
        height,
        width,
        food: value_to_position_vec(&frame["Food"])?,
        hazards: value_to_position_vec(&frame["Hazards"])?,
        snakes,
    })
}

pub(crate) fn frame_to_game(frame: &Value, game: &Value, you_name: &str) -> Result<Game> {
    let turn = frame["Turn"]
        .as_i64()
        .ok_or_else(|| eyre!("Turn is not an integer"))?
        .try_into()
        .wrap_err("Turn is too big for an i32")?;

    let nested_game = frame_to_nested_game(game)?;

    let board = frame_to_board(frame, game)?;

    let you = board
        .snakes
        .iter()
        .find(|snake| snake.name == you_name)
        .unwrap_or_else(|| {
            board
                .snakes
                .first()
                .expect("There are no snakes in this game")
        })
        .clone();

    Ok(Game {
        turn,
        game: nested_game,
        board,
        you,
    })
}

pub(crate) fn get_frame_for_turn(game_id: &str, turn: i32) -> Result<Value> {
    let body: Value = ureq::get(
        format!("https://engine.battlesnake.com/games/{game_id}/frames?offset={turn}&limit=1",)
            .as_str(),
    )
    .call()?
    .into_json()?;

    Ok(body["Frames"][0].clone())
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FrameResponse {
    #[serde(rename = "Frames")]
    frames: Option<Vec<Value>>,
}

pub(crate) fn get_batch_of_frames_for_games(
    game_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Option<Vec<Value>>> {
    Ok(ureq::get(&format!(
        "https://engine.battlesnake.com/games/{game_id}/frames?offset={offset}&limit={limit}",
    ))
    .call()?
    .into_json::<FrameResponse>()?
    .frames)
}

pub(crate) fn get_frames_for_game(game_id: &str, end_turn: usize) -> Result<Vec<Value>> {
    const LIMIT: usize = 100;
    let mut offset = 0;

    let mut all_frames: Vec<Value> = Vec::with_capacity(end_turn);

    while let Some(frames) = get_batch_of_frames_for_games(game_id, offset, LIMIT)? && !frames.is_empty()
    {
        all_frames.extend(frames.iter().cloned());
        offset += LIMIT;
    }

    all_frames.sort_by_key(|f| f["Turn"].as_u64());

    Ok(all_frames)
}
