use std::{collections::HashMap, fmt::Debug};

use itertools::Itertools;
use serde_json::Value;

use battlesnake_minimax::paranoid::{MinMaxReturn, MinimaxSnake, WrappedScore};
use types::{
    compact_representation::{dimensions::ArcadeMaze, WrappedCellBoard},
    types::{build_snake_id_map, Move, SnakeIDGettableGame, SnakeId, YouDeterminableGame},
    wire_representation::{BattleSnake, Board, Game, NestedGame, Position, Ruleset, Settings},
};

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

    let settings = Settings {
        food_spawn_chance: game["Ruleset"]["foodSpawnChance"]
            .as_str()
            .ok_or("Missing Food Spawn Chance")?
            .parse()
            .map_err(|_| "Too big for an i32")?,
        minimum_food: game["Ruleset"]["minimumFood"]
            .as_str()
            .ok_or("Missing minimumFood")?
            .parse()
            .map_err(|_| "Too big for an i32")?,
        hazard_damage_per_turn: game["Ruleset"]["damagePerTurn"]
            .as_str()
            .ok_or("Missing damagePerTurn")?
            .parse()
            .map_err(|_| "Too big for an i32")?,
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

fn value_to_position_vec(value: &Value) -> Result<Vec<Position>, &'static str> {
    value
        .as_array()
        .ok_or("Not an array")?
        .iter()
        .map(|pos| {
            let x = pos["X"]
                .as_i64()
                .ok_or("X is not an integer")?
                .try_into()
                .map_err(|_| "Too big for an i32")?;

            let y = pos["Y"]
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
        .filter(|snake_json| snake_json["Death"].is_null())
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

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Game ID to debug
    #[clap(short, long, value_parser)]
    game_id: String,

    /// Number of times to greet
    #[clap(short, long, value_parser)]
    you_name: String,

    /// Number of turns past the last frame to check
    #[clap(short, long, value_parser, default_value_t = 50)]
    turns_after_lose: i32,
}

fn get_frame_for_turn(game_id: &str, turn: i32) -> Result<Value, ureq::Error> {
    let body: Value = ureq::get(
        format!(
            "https://engine.battlesnake.com/games/{}/frames?offset={}&limit=1",
            game_id, turn
        )
        .as_str(),
    )
    .call()?
    .into_json()?;

    Ok(body["Frames"][0].clone())
}

fn print_moves<
    GameType: SnakeIDGettableGame<SnakeIDType = SnakeId> + Debug + Clone,
    ScoreType: Copy + Ord + Debug,
>(
    result: &MinMaxReturn<GameType, ScoreType>,
    current_turn: i32,
    m: Move,
) {
    let all_snake_path = result.chosen_route();
    let sids = all_snake_path
        .iter()
        .map(|(sid, _)| sid)
        .unique()
        .collect_vec();
    let mut paths_per_snake: HashMap<SnakeId, Vec<Move>> = HashMap::new();
    for &sid in &sids {
        let path = all_snake_path
            .iter()
            .filter(|(s, _)| s == sid)
            .map(|(_, p)| p)
            .cloned()
            .collect_vec();
        paths_per_snake.insert(*sid, path);
    }
    println!(
        "At turn {current_turn}, the {m} path takes {} turn lookahead:",
        all_snake_path.len() / sids.len()
    );
    for (sid, path) in paths_per_snake {
        println!("{sid:?}: {}", path.iter().join(", "));
    }
    println!()
}

fn main() -> Result<(), ureq::Error> {
    let args = Args::parse();

    let body: Value =
        ureq::get(format!("https://engine.battlesnake.com/games/{}", args.game_id).as_str())
            .call()?
            .into_json()?;

    let last_frame = &body["LastFrame"];
    let last_turn = last_frame["Turn"].as_i64().expect("Missing Turn") as i32;
    let mut current_turn = last_turn - 1;

    println!("Ending Turn {}", &last_frame["Turn"]);

    loop {
        // if current_turn == 855 || current_turn == 854 {
        //     current_turn -= 1;
        //     continue;
        // }
        let current_frame = get_frame_for_turn(&args.game_id, current_turn)?;
        let wire_game = frame_to_game(&current_frame, &body["Game"], &args.you_name).unwrap();

        if !wire_game.is_wrapped() {
            unimplemented!("Only implementing for wrapped games, RIGHT NOW");
        }

        let snake_ids = build_snake_id_map(&wire_game);
        let game_info = wire_game.game.clone();
        let game: WrappedCellBoard<u16, ArcadeMaze, { 19 * 21 }, 8> =
            wire_game.as_wrapped_cell_board(&snake_ids).unwrap();

        let you_id = game.you_id();

        let explorer_snake = MinimaxSnake::new(game, game_info, current_turn, &|_| {}, "explorer");

        let max_turns = (last_turn - current_turn + args.turns_after_lose) as usize;
        let result = explorer_snake.single_minimax(max_turns);

        let score = *result.score();

        if matches!(score, WrappedScore::Lose(_) | WrappedScore::Tie(_)) {
            println!("At turn {}, there were no safe options", current_turn);
        } else if matches!(score, WrappedScore::Win(_)) {
            println!("At turn {}, you could have won!", current_turn);
            if let MinMaxReturn::Node { options, .. } = &result {
                let winning_moves = options
                    .iter()
                    .filter(|(_, r)| matches!(r.score(), WrappedScore::Win(_)))
                    .map(|(m, _)| *m)
                    .collect_vec();

                println!(
                    "At turn {}, the winning moves were {:?}",
                    current_turn, winning_moves
                );
                print_moves(&result, current_turn, winning_moves[0]);
            }
            break;
        } else if let MinMaxReturn::Node {
            options,
            moving_snake_id,
            ..
        } = &result
        {
            assert!(moving_snake_id == you_id);
            let safe_options = options
                .iter()
                .filter(|(_, r)| matches!(r.score(), WrappedScore::Scored(_)))
                .collect_vec();
            let safe_moves = safe_options.iter().map(|(m, _)| *m).collect_vec();

            println!(
                "At turn {}, the safe options were {:?}",
                current_turn, safe_moves
            );
            println!("Turn {} is the decision point", current_turn);

            for m in safe_moves {
                print_moves(&result, current_turn, m);
            }

            // let mut file = File::create("tmp.dot").unwrap();
            // file.write_all(format!("{}", result.to_dot_graph(you_id)).as_bytes())
            //     .unwrap();

            // Command::new("dot")
            //     .arg("-Tsvg")
            //     .arg("-O")
            //     .arg("tmp.dot")
            //     .output()
            //     .unwrap();
            // Command::new("open").arg("tmp.dot.svg").output().unwrap();

            break;
        } else {
            panic!("We shouldn't ever have a leaf here")
        }

        current_turn -= 1;
    }

    Ok(())
}
